//! OpenAI-compatible provider client (non-streaming).
//!
//! Implements the [`LlmProvider`] trait for any backend that exposes the
//! OpenAI Chat Completions API (Ollama, OpenAI, etc.). Streaming is stubbed
//! and will be implemented in a separate issue.

use crate::providers::{ChatCompletionRequest, ChatCompletionResponse, LlmProvider, ProviderError};

use async_trait::async_trait;

/// Environment variable for the provider base URL.
const ENV_BASE_URL: &str = "LLM_BASE_URL";
/// Environment variable for the API key (optional).
const ENV_API_KEY: &str = "LLM_API_KEY";
/// Environment variable for the default model.
const ENV_MODEL: &str = "LLM_MODEL";
/// Environment variable for the connect timeout in seconds.
const ENV_CONNECT_TIMEOUT_SECS: &str = "LLM_CONNECT_TIMEOUT_SECS";
/// Environment variable for the overall request timeout in seconds.
const ENV_TIMEOUT_SECS: &str = "LLM_TIMEOUT_SECS";

/// Default base URL (Ollama local).
const DEFAULT_BASE_URL: &str = "http://localhost:11434";
/// Default model name (Ollama default).
const DEFAULT_MODEL: &str = "llama3";
/// Default connect timeout in seconds.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Default overall request timeout in seconds (long for LLM generation).
const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Maximum bytes to read from an error response body before truncating.
const MAX_ERROR_BODY_BYTES: usize = 4096;

/// An LLM provider client that speaks the OpenAI Chat Completions API.
///
/// Holds a single [`reqwest::Client`] for connection pooling across requests.
/// Works with any compatible backend — Ollama, OpenAI, etc. — determined by
/// `base_url` and `api_key`.
pub struct OpenAiProvider {
    /// Reusable HTTP client for connection pooling.
    client: reqwest::Client,
    /// Base URL for the API (e.g. `http://localhost:11434` or `https://api.openai.com`).
    base_url: String,
    /// Optional API key — `None` for Ollama, `Some("sk-...")` for OpenAI.
    api_key: Option<String>,
    /// Default model to use for requests (e.g. `llama3`, `gpt-4o-mini`).
    model: String,
}

impl OpenAiProvider {
    /// Create a new `OpenAiProvider` with explicit configuration.
    ///
    /// The `reqwest::Client` is constructed internally with sensible timeouts
    /// (short connect timeout, long overall timeout) and reused for all
    /// subsequent requests to benefit from connection pooling.
    #[must_use]
    pub fn new(base_url: String, api_key: Option<String>, model: String) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("failed to build reqwest::Client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.filter(|k| !k.is_empty()),
            model,
        }
    }

    /// Create an `OpenAiProvider` from environment variables.
    ///
    /// Reads:
    /// - `LLM_BASE_URL` — defaults to `http://localhost:11434`
    /// - `LLM_API_KEY` — optional; `None` if unset or empty
    /// - `LLM_MODEL` — defaults to `llama3`
    /// - `LLM_CONNECT_TIMEOUT_SECS` — defaults to `10`
    /// - `LLM_TIMEOUT_SECS` — defaults to `300`
    #[must_use]
    pub fn from_env() -> Self {
        let base_url = std::env::var(ENV_BASE_URL).unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let api_key = std::env::var(ENV_API_KEY).ok().filter(|k| !k.is_empty());
        let model = std::env::var(ENV_MODEL).unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let connect_timeout_secs = std::env::var(ENV_CONNECT_TIMEOUT_SECS)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_CONNECT_TIMEOUT_SECS);

        let timeout_secs = std::env::var(ENV_TIMEOUT_SECS)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs))
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .expect("failed to build reqwest::Client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    /// Returns the base URL this provider is configured with.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the API key, if set.
    #[must_use]
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Returns the default model name.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Truncate a byte slice to `max_len` bytes, ensuring the result is valid
/// UTF-8 by finding a char boundary. Appends "…" if truncation occurred.
fn truncate_bytes_to_string(bytes: &[u8], max_len: usize) -> String {
    if bytes.len() <= max_len {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    // Find a safe UTF-8 boundary by scanning backwards from max_len.
    // Error bodies are typically ASCII, so the first byte that starts
    // a new UTF-8 character (top bit clear or continuation byte pattern)
    // marks a valid split point.
    let mut end = max_len;
    while end > 0 {
        // A leading byte in UTF-8 has the top two bits as 0b11 (0xC0).
        // Scanning for this ensures we split at a char boundary.
        if bytes[end] & 0xC0 != 0x80 {
            break;
        }
        end -= 1;
    }
    if end == 0 {
        end = max_len;
    }

    let truncated = String::from_utf8_lossy(&bytes[..end]);
    format!("{truncated}…")
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    /// Send a non-streaming chat completion request.
    ///
    /// Builds the request body from the given [`ChatCompletionRequest`],
    /// explicitly setting `stream: false`. Sends a `POST` to
    /// `{base_url}/v1/chat/completions` with an `Authorization: Bearer`
    /// header when an API key is configured.
    ///
    /// # Error mapping
    ///
    /// - HTTP 4xx/5xx → [`ProviderError::ApiError`]
    /// - Connection refused / timeout → [`ProviderError::ConnectionFailed`]
    /// - Malformed JSON → [`ProviderError::InvalidResponse`]
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        // Build the request body, forcing stream: false.
        // ChatCompletionRequest derives Serialize so this is infallible.
        let mut body =
            serde_json::to_value(&request).expect("failed to serialize ChatCompletionRequest");
        body["stream"] = serde_json::Value::Bool(false);

        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut builder = self.client.post(&url).json(&body);

        if let Some(ref key) = self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::ConnectionFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response
                .bytes()
                .await
                .map(|b| truncate_bytes_to_string(&b, MAX_ERROR_BODY_BYTES))
                .unwrap_or_else(|e| format!("(failed to read error body: {e})"));
            return Err(ProviderError::ApiError {
                status: status_code,
                message,
            });
        }

        let response_text = response.text().await.map_err(|e| {
            ProviderError::InvalidResponse(format!("failed to read response body: {e}"))
        })?;

        serde_json::from_str::<ChatCompletionResponse>(&response_text).map_err(|e| {
            ProviderError::InvalidResponse(format!("failed to deserialize response: {e}"))
        })
    }

    /// Streaming stub — returns [`ProviderError::StreamingNotSupported`].
    ///
    /// Streaming chat completion will be implemented in a separate issue.
    async fn chat_completion_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<crate::providers::ChatCompletionChunk, ProviderError>>,
        ProviderError,
    > {
        Err(ProviderError::StreamingNotSupported)
    }

    /// Human-readable name for this provider.
    fn name(&self) -> &str {
        "OpenAI-compatible"
    }
}
