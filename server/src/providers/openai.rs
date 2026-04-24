//! OpenAI-compatible provider client with SSE streaming support.
//!
//! Implements the [`LlmProvider`] trait for any backend that exposes the
//! OpenAI Chat Completions API (Ollama, OpenAI, etc.). Streaming uses
//! Server-Sent Events (SSE) to deliver token-level responses via a
//! [`tokio::sync::mpsc`] channel.

use crate::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, LlmProvider, ProviderError,
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

/// Truncate a byte slice to `max_len` bytes, ensuring the result is valid
/// UTF-8 by finding a char boundary. Appends "…" if truncation occurred.
fn truncate_bytes_to_string(bytes: &[u8], max_len: usize) -> String {
    if bytes.len() <= max_len {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    let mut end = max_len;
    while end > 0 {
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

/// Process all `data:` lines in a decoded SSE event string, sending parsed
/// chunks through the channel.
///
/// Per the SSE specification, a `data:` field may optionally be followed by a
/// single space before the value. Both `data:{"…"}` and `data: {"…"}` are
/// accepted.
///
/// Returns `Ok(true)` if `[DONE]` was encountered, `Ok(false)` otherwise,
/// or `Err(())` if the receiver was dropped.
async fn process_sse_event(
    event: &str,
    tx: &mpsc::Sender<Result<ChatCompletionChunk, ProviderError>>,
) -> Result<bool, ()> {
    for line in event.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            // Per SSE spec, strip at most one leading space after "data:".
            let data = data.strip_prefix(' ').unwrap_or(data);
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

    /// Send a streaming chat completion request via Server-Sent Events.
    ///
    /// Sends a `POST` to `{base_url}/v1/chat/completions` with
    /// `"stream": true` in the request body. Reads the response as a byte
    /// stream, parses SSE `data:` lines, and sends each parsed
    /// [`ChatCompletionChunk`] through the returned mpsc channel.
    ///
    /// # SSE protocol
    ///
    /// - Each `data:` line contains a serialised `ChatCompletionChunk`.
    /// - `data: [DONE]` signals end-of-stream; the channel is closed gracefully.
    /// - Partial SSE lines that span multiple TCP chunks are buffered and
    ///   reassembled before parsing.
    /// - Malformed JSON sends `Err(InvalidResponse)` but does **not** terminate
    ///   the stream.
    ///
    /// # Error mapping (initial request)
    ///
    /// - HTTP 4xx/5xx → [`ProviderError::ApiError`]
    /// - Connection refused / timeout → [`ProviderError::ConnectionFailed`]
    ///
    /// # Error mapping (during streaming)
    ///
    /// - Bytes-stream error → [`ProviderError::ConnectionFailed`]
    /// - Clean EOF without `[DONE]` → [`ProviderError::StreamEnded`]
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
                    // Trimmed &str may yield InvalidResponse for partial JSON fragments,
                    // but we intentionally continue to fallthrough and send StreamEnded.
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

    /// Human-readable name for this provider.
    fn name(&self) -> &str {
        "OpenAI-compatible"
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
