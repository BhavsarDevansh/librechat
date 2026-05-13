//! OpenAI-compatible provider client with SSE streaming support.
//!
//! Implements the [`LlmProvider`] trait for any backend that exposes the
//! OpenAI Chat Completions API (Ollama, OpenAI, etc.). Streaming uses
//! Server-Sent Events (SSE) to deliver token-level responses via a
//! [`tokio::sync::mpsc`] channel.

use crate::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, LlmProvider, ModelInfo,
    ProviderError,
};

use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::mpsc;

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

/// Buffer size for the mpsc channel used to stream chunks to the caller.
const STREAM_CHANNEL_BUFFER: usize = 32;

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

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut builder = self.client.post(&url).json(&request);
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

        response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>, ProviderError> {
        let mut body =
            serde_json::to_value(&request).expect("failed to serialize ChatCompletionRequest");
        body["stream"] = serde_json::Value::Bool(true);

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

        let (tx, rx) = mpsc::channel(STREAM_CHANNEL_BUFFER);
        let byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer: Vec<u8> = Vec::new();
            let mut stream = byte_stream;

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(Err(ProviderError::ConnectionFailed(e.to_string())))
                            .await;
                        return;
                    }
                };

                buffer.extend_from_slice(&chunk);

                // Process complete SSE events. Each event ends with \n\n or \r\n\r\n.
                while let Some((pos, delim_len)) = find_event_delimiter(&buffer) {
                    let event_bytes: Vec<u8> = buffer.drain(..pos + delim_len).collect();
                    // The event bytes include the trailing delimiter — trim it.
                    let event_bytes = &event_bytes[..event_bytes.len() - delim_len];

                    if event_bytes.is_empty() {
                        continue;
                    }

                    let event_text = match String::from_utf8(event_bytes.to_vec()) {
                        Ok(t) => t,
                        Err(e) => {
                            if tx
                                .send(Err(ProviderError::InvalidResponse(format!(
                                    "invalid UTF-8 in SSE event: {e}"
                                ))))
                                .await
                                .is_err()
                            {
                                return;
                            }
                            continue;
                        }
                    };

                    match process_sse_event(&event_text, &tx).await {
                        Ok(true) => return, // [DONE] received
                        Ok(false) => {}
                        Err(()) => return, // Receiver dropped
                    }
                }
            }

            // Stream ended without [DONE] — process any remaining data in the buffer.
            if !buffer.is_empty() {
                let remaining = String::from_utf8_lossy(&buffer);
                let remaining = remaining.trim();
                if !remaining.is_empty() {
                    match process_sse_event(remaining, &tx).await {
                        Ok(true) => return,
                        Ok(false) => {}
                        Err(()) => return,
                    }
                }
            }

            // Stream ended without a [DONE] sentinel — notify the caller.
            let _ = tx.send(Err(ProviderError::StreamEnded)).await;
        });

        Ok(rx)
    }

    /// List available models from the provider.
    ///
    /// Tries the OpenAI-compatible `/v1/models` endpoint first. If that
    /// returns an empty list or fails, falls back to Ollama's native
    /// `/api/tags` endpoint. Returns a list of [`ModelInfo`] with model
    /// identifiers.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // Try OpenAI-compatible /v1/models endpoint first.
        let url = format!("{}/v1/models", self.base_url);

        let mut builder = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = match builder.send().await {
            Ok(r) => r,
            Err(_e) => {
                // Connection failed on /v1/models — try Ollama fallback.
                return self.list_models_ollama().await;
            }
        };

        if response.status().is_success() {
            let body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

            // OpenAI format: { "data": [ { "id": "model-name" }, ... ] }
            if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
                let models: Vec<ModelInfo> = data
                    .iter()
                    .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
                    .map(|id| ModelInfo { id: id.to_string() })
                    .collect();
                if !models.is_empty() {
                    return Ok(models);
                }
            }

            // Some providers return a flat array of model names under "models".
            if let Some(models_arr) = body.get("models").and_then(|m| m.as_array()) {
                let models: Vec<ModelInfo> = models_arr
                    .iter()
                    .filter_map(|item| {
                        item.as_str()
                            .map(|s| ModelInfo { id: s.to_string() })
                            .or_else(|| {
                                item.get("id")
                                    .and_then(|id| id.as_str())
                                    .map(|s| ModelInfo { id: s.to_string() })
                            })
                    })
                    .collect();
                if !models.is_empty() {
                    return Ok(models);
                }
            }
        }

        // Fallback: try Ollama's native /api/tags endpoint.
        self.list_models_ollama().await
    }

    /// Human-readable name for this provider.
    fn name(&self) -> &str {
        "OpenAI-compatible"
    }
}

impl OpenAiProvider {
    /// Fallback: list models via Ollama's native `/api/tags` endpoint.
    async fn list_models_ollama(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let url = format!("{}/api/tags", self.base_url);

        let mut builder = self.client.get(&url);
        if let Some(ref key) = self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response
                .bytes()
                .await
                .map(|b| truncate_bytes_to_string(&b, MAX_ERROR_BODY_BYTES))
                .unwrap_or_else(|e| format!("(failed to read error body: {e})"));
            return Err(ProviderError::ApiError { status, message });
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        // Ollama format: { "models": [ { "name": "llama3:latest", ... }, ... ] }
        if let Some(models) = body.get("models").and_then(|m| m.as_array()) {
            let model_list: Vec<ModelInfo> = models
                .iter()
                .filter_map(|item| {
                    item.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| ModelInfo { id: n.to_string() })
                })
                .collect();
            if !model_list.is_empty() {
                return Ok(model_list);
            }
        }

        Err(ProviderError::InvalidResponse(
            "No models found from provider: both /v1/models and /api/tags returned empty or invalid responses".to_string(),
        ))
    }
}

/// Find the position of the first SSE event delimiter (`\n\n`) in a byte slice.
/// Returns the byte index of the start of the delimiter and its length as (position, length),
/// or `None` if not found. Detects both "\n\n" and "\r\n\r\n" per the SSE spec.
fn find_event_delimiter(buffer: &[u8]) -> Option<(usize, usize)> {
    // Try to find \r\n\r\n first (length 4)
    if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some((pos, 4));
    }
    // Fall back to \n\n (length 2)
    buffer
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|pos| (pos, 2))
}

/// Process a single SSE event from the byte stream.
///
/// Returns `Ok(true)` if [DONE] was received (stream should end),
/// `Ok(false)` if a normal chunk was processed, or `Err(())` if the
/// receiver has been dropped.
async fn process_sse_event(
    event_text: &str,
    tx: &mpsc::Sender<Result<ChatCompletionChunk, ProviderError>>,
) -> Result<bool, ()> {
    for line in event_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" {
                return Ok(true);
            }
            match serde_json::from_str::<ChatCompletionChunk>(data) {
                Ok(chunk) => {
                    if tx.send(Ok(chunk)).await.is_err() {
                        return Err(());
                    }
                }
                Err(e) => {
                    if tx
                        .send(Err(ProviderError::InvalidResponse(format!(
                            "failed to parse SSE chunk: {e}"
                        ))))
                        .await
                        .is_err()
                    {
                        return Err(());
                    }
                }
            }
        }
    }
    Ok(false)
}

/// Truncate a byte slice to a string, limiting to `max_bytes`.
/// Used for error response bodies that may be very large.
/// Finds the last valid UTF-8 character boundary at or before `max_bytes`
/// to avoid slicing in the middle of a multi-byte sequence.
fn truncate_bytes_to_string(bytes: &[u8], max_bytes: usize) -> String {
    let limit = if bytes.len() > max_bytes {
        // Walk backwards from max_bytes to find a valid UTF-8 char boundary.
        let mut end = max_bytes;
        while end > 0 && std::str::from_utf8(&bytes[..end]).is_err() {
            end -= 1;
        }
        if end == 0 {
            max_bytes
        } else {
            end
        }
    } else {
        bytes.len()
    };
    String::from_utf8_lossy(&bytes[..limit]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_constant() {
        assert_eq!(DEFAULT_MODEL, "llama3");
    }

    #[test]
    fn test_provider_new_trims_trailing_slash() {
        let provider = OpenAiProvider::new(
            "http://localhost:11434/".to_string(),
            None,
            "test-model".to_string(),
        );
        assert_eq!(provider.base_url(), "http://localhost:11434");
    }

    #[test]
    fn test_provider_new_strips_empty_api_key() {
        let provider = OpenAiProvider::new(
            "http://localhost:11434".to_string(),
            Some("".to_string()),
            "test-model".to_string(),
        );
        assert!(provider.api_key().is_none());
    }

    #[test]
    fn test_provider_new_keeps_nonempty_api_key() {
        let provider = OpenAiProvider::new(
            "http://localhost:11434".to_string(),
            Some("sk-test".to_string()),
            "test-model".to_string(),
        );
        assert_eq!(provider.api_key(), Some("sk-test"));
    }

    #[test]
    fn test_find_event_delimiter_double_newline() {
        assert_eq!(find_event_delimiter(b"hello\n\nworld"), Some((5, 2)));
    }

    #[test]
    fn test_find_event_delimiter_crlf_crlf() {
        assert_eq!(find_event_delimiter(b"hello\r\n\r\nworld"), Some((5, 4)));
    }

    #[test]
    fn test_find_event_delimiter_none() {
        assert_eq!(find_event_delimiter(b"no delimiter here"), None);
    }

    #[test]
    fn test_truncate_bytes_to_string_short() {
        assert_eq!(truncate_bytes_to_string(b"hello", 100), "hello");
    }

    #[test]
    fn test_truncate_bytes_to_string_truncated() {
        assert_eq!(truncate_bytes_to_string(b"hello world", 5), "hello");
    }
}
