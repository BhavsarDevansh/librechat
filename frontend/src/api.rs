//! Frontend API client for the LibreChat chat and models endpoints.
//!
//! Provides [`send_chat_request`] and [`fetch_models`] which communicate with
//! the backend. The API base URL and optional auth key are read from the
//! application state settings rather than hardcoded constants.

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

/// Default model used when the caller does not specify one.
pub const DEFAULT_MODEL: &str = "llama3";

/// Role of a participant in a chat conversation (API serialisation format).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiMessageRole {
    System,
    User,
    Assistant,
}

/// A single message in a chat conversation (API serialisation format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChatMessage {
    pub role: ApiMessageRole,
    pub content: String,
}

/// Request payload sent to `POST /api/chat/completions`.
#[derive(Debug, Clone, Serialize)]
pub struct ApiChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ApiChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Non-streaming response received from the chat completions endpoint.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ApiChoice>,
    pub usage: ApiUsage,
}

/// A single completion choice in a non-streaming response.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChoice {
    pub index: u32,
    pub message: ApiChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage statistics returned by the provider.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single model returned by the `/api/models` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiModelInfo {
    pub id: String,
}

/// A single chunk in a streaming response (SSE format).
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ApiChatCompletionChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ApiChunkChoice>,
}

/// A single choice within a streaming chunk.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ApiChunkChoice {
    pub index: u32,
    pub delta: ApiChunkDelta,
    pub finish_reason: Option<String>,
}

/// Delta content within a streaming chunk choice.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ApiChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<ApiMessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Response from the `/api/models` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiModelsResponse {
    pub models: Vec<ApiModelInfo>,
}

/// Errors that can occur when calling the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// The network request failed (e.g. CORS, DNS, connection refused).
    Network(String),
    /// The server returned a non-2xx HTTP status code.
    Http { status: u16, body: String },
    /// The response body could not be parsed as valid JSON.
    Parse(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(msg) => write!(f, "Network error: {msg}"),
            ApiError::Http { status, body } => write!(f, "HTTP {status}: {body}"),
            ApiError::Parse(msg) => write!(f, "Failed to parse response: {msg}"),
        }
    }
}

/// Resolve the full API base URL. If the user has configured a custom
/// endpoint in settings, that is used. Otherwise falls back to the
/// `window.__LIBRECHAT_API_URL__` JS property (empty string = same origin).
fn resolve_api_base(custom_endpoint: &str) -> String {
    if !custom_endpoint.is_empty() {
        return custom_endpoint.trim_end_matches('/').to_string();
    }
    js_api_base_url()
}

/// Read the API base URL from the `window.__LIBRECHAT_API_URL__` JavaScript
/// property. Returns an empty string (i.e. relative URLs) when the property
/// is not set.
fn js_api_base_url() -> String {
    use web_sys::window;

    let Some(win) = window() else {
        return String::new();
    };

    let Ok(value) = js_sys::Reflect::get(
        &win.into(),
        &js_sys::JsString::from("__LIBRECHAT_API_URL__"),
    ) else {
        return String::new();
    };

    value.as_string().unwrap_or_default()
}

/// Build a `RequestBuilder` with optional `Authorization: Bearer` header.
fn builder_with_auth(method: &str, url: &str, auth_key: &str) -> gloo_net::http::RequestBuilder {
    let builder = match method {
        "GET" => Request::get(url),
        "POST" => Request::post(url),
        _ => panic!("Unsupported HTTP method: {method}"),
    };

    if !auth_key.is_empty() {
        builder.header("Authorization", &format!("Bearer {auth_key}"))
    } else {
        builder
    }
}

/// Send a non-streaming chat completion request to the backend.
///
/// Constructs a `POST /api/chat/completions` request with the given messages
/// and model, then awaits the full response. The model defaults to
/// [`DEFAULT_MODEL`] when an empty string is supplied. The `endpoint` and
/// `auth_key` parameters override the default origin and add auth headers.
#[allow(dead_code)]
pub async fn send_chat_request(
    messages: &[ApiChatMessage],
    model: &str,
    endpoint: &str,
    auth_key: &str,
) -> Result<ApiChatCompletionResponse, ApiError> {
    let base = resolve_api_base(endpoint);
    let url = format!("{base}/api/chat/completions");

    let request = ApiChatCompletionRequest {
        model: if model.is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            model.to_string()
        },
        messages: messages.to_vec(),
        temperature: None,
        max_tokens: None,
        stream: Some(false),
    };

    let response = builder_with_auth("POST", &url, auth_key)
        .json(&request)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = response.status();

    if !response.ok() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        return Err(ApiError::Http {
            status,
            body: body.chars().take(512).collect(),
        });
    }

    response
        .json()
        .await
        .map_err(|e| ApiError::Parse(e.to_string()))
}

/// Fetch the list of available models from the backend.
///
/// Calls `GET /api/models` and returns the model identifiers. The `endpoint`
/// and `auth_key` parameters override the default origin and add auth headers.
pub async fn fetch_models(endpoint: &str, auth_key: &str) -> Result<Vec<ApiModelInfo>, ApiError> {
    let base = resolve_api_base(endpoint);
    let url = format!("{base}/api/models");

    let response = builder_with_auth("GET", &url, auth_key)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = response.status();

    if !response.ok() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        return Err(ApiError::Http {
            status,
            body: body.chars().take(512).collect(),
        });
    }

    let models_response: ApiModelsResponse = response
        .json()
        .await
        .map_err(|e| ApiError::Parse(e.to_string()))?;

    Ok(models_response.models)
}

/// Send a streaming chat completion request to the backend.
///
/// Constructs a `POST /api/chat/completions/stream` request with the given
/// messages and model, then reads the SSE response body chunk-by-chunk.
/// For each parsed [`ApiChatCompletionChunk`], the `on_chunk` callback is
/// invoked. The stream terminates when `data: [DONE]` is received or an
/// error occurs.
pub async fn stream_chat_request(
    messages: &[ApiChatMessage],
    model: &str,
    endpoint: &str,
    auth_key: &str,
    mut on_chunk: impl FnMut(ApiChatCompletionChunk),
) -> Result<(), ApiError> {
    let base = resolve_api_base(endpoint);
    let url = format!("{base}/api/chat/completions/stream");

    let request = ApiChatCompletionRequest {
        model: if model.is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            model.to_string()
        },
        messages: messages.to_vec(),
        temperature: None,
        max_tokens: None,
        stream: Some(true),
    };

    let response = builder_with_auth("POST", &url, auth_key)
        .json(&request)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = response.status();

    if !response.ok() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        return Err(ApiError::Http {
            status,
            body: body.chars().take(512).collect(),
        });
    }

    let body = response
        .body()
        .ok_or_else(|| ApiError::Network("response body missing".to_string()))?;

    let reader = web_sys::ReadableStreamDefaultReader::new(&body)
        .map_err(|e| ApiError::Network(format!("failed to create stream reader: {e:?}")))?;

    let mut parser = crate::sse::SseParser::new();
    let mut leftover = Vec::new();

    loop {
        let promise = reader.read();
        let result = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| ApiError::Network(format!("stream read failed: {e:?}")))?;

        let done = js_sys::Reflect::get(&result, &"done".into())
            .map_err(|e| ApiError::Parse(format!("invalid stream result: {e:?}")))?
            .as_bool()
            .unwrap_or(false);

        if done {
            if let Some(event) = parser.finalize() {
                if event.data == "[DONE]" {
                    return Ok(());
                }
                if event.event_type == "error" {
                    let msg = serde_json::from_str::<serde_json::Value>(&event.data)
                        .ok()
                        .and_then(|v| v.get("error")?.get("message")?.as_str().map(String::from))
                        .unwrap_or_else(|| event.data.clone());
                    return Err(ApiError::Http {
                        status: 500,
                        body: msg,
                    });
                }
                let chunk: ApiChatCompletionChunk = serde_json::from_str(&event.data)
                    .map_err(|e| ApiError::Parse(format!("invalid chunk JSON: {e}")))?;
                on_chunk(chunk);
            }
            return Ok(());
        }

        let value = js_sys::Reflect::get(&result, &"value".into())
            .map_err(|e| ApiError::Parse(format!("invalid stream value: {e:?}")))?;

        if value.is_undefined() || value.is_null() {
            continue;
        }

        let array = js_sys::Uint8Array::from(value);
        let mut bytes = vec![0u8; array.length() as usize];
        array.copy_to(&mut bytes);

        leftover.extend_from_slice(&bytes);
        let mut split = leftover.len();
        while split > 0 && (leftover[split - 1] & 0b1100_0000) == 0b1000_0000 {
            split -= 1;
        }
        let tail = leftover.split_off(split);
        let valid = std::mem::replace(&mut leftover, tail);
        let text = String::from_utf8_lossy(&valid);

        let events = parser.feed(&text);
        for event in events {
            if event.data == "[DONE]" {
                return Ok(());
            }
            if event.event_type == "error" {
                let msg = serde_json::from_str::<serde_json::Value>(&event.data)
                    .ok()
                    .and_then(|v| v.get("error")?.get("message")?.as_str().map(String::from))
                    .unwrap_or_else(|| event.data.clone());
                return Err(ApiError::Http {
                    status: 500,
                    body: msg,
                });
            }
            let chunk: ApiChatCompletionChunk = serde_json::from_str(&event.data)
                .map_err(|e| ApiError::Parse(format!("invalid chunk JSON: {e}")))?;
            on_chunk(chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_constant() {
        assert_eq!(DEFAULT_MODEL, "llama3");
    }

    #[test]
    fn test_api_error_display_network() {
        let err = ApiError::Network("connection refused".to_string());
        assert_eq!(format!("{err}"), "Network error: connection refused");
    }

    #[test]
    fn test_api_error_display_http() {
        let err = ApiError::Http {
            status: 502,
            body: "bad gateway".to_string(),
        };
        assert_eq!(format!("{err}"), "HTTP 502: bad gateway");
    }

    #[test]
    fn test_api_error_display_parse() {
        let err = ApiError::Parse("invalid json".to_string());
        assert_eq!(format!("{err}"), "Failed to parse response: invalid json");
    }

    #[test]
    fn test_api_message_role_serialisation() {
        let role = ApiMessageRole::User;
        let json = serde_json::to_string(&role).expect("serialise");
        assert_eq!(json, "\"user\"");

        let role = ApiMessageRole::Assistant;
        let json = serde_json::to_string(&role).expect("serialise");
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn test_chat_completion_request_serialisation() {
        let req = ApiChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ApiChatMessage {
                role: ApiMessageRole::User,
                content: "hello".to_string(),
            }],
            temperature: None,
            max_tokens: None,
            stream: Some(false),
        };
        let json = serde_json::to_string(&req).expect("serialise");
        assert!(json.contains("\"stream\":false"));
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn test_resolve_api_base_uses_custom_endpoint() {
        assert_eq!(
            resolve_api_base("http://localhost:11434"),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_resolve_api_base_strips_trailing_slash() {
        assert_eq!(
            resolve_api_base("http://localhost:11434/"),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_chat_completion_chunk_deserialisation() {
        let json = r#"{
            "id": "chatcmpl-123",
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "delta": { "role": "assistant", "content": "Hello" },
                    "finish_reason": null
                }
            ]
        }"#;
        let chunk: ApiChatCompletionChunk = serde_json::from_str(json).expect("deserialise chunk");
        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.model, "gpt-4");
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].index, 0);
        assert_eq!(chunk.choices[0].delta.role, Some(ApiMessageRole::Assistant));
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_stream_request_payload_serialisation() {
        let req = ApiChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ApiChatMessage {
                role: ApiMessageRole::User,
                content: "hello".to_string(),
            }],
            temperature: None,
            max_tokens: None,
            stream: Some(true),
        };
        let json = serde_json::to_string(&req).expect("serialise");
        assert!(json.contains("\"stream\":true"));
    }
}
