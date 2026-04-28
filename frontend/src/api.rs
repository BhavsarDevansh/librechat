//! Frontend API client for the LibreChat non-streaming chat endpoint.
//!
//! Provides [`send_chat_request`] which posts a conversation history to the
//! backend `POST /api/chat/completions` endpoint and returns the full
//! [`ApiChatCompletionResponse`]. The API base URL is configurable via the
//! `window.__LIBRECHAT_API_URL__` JavaScript property (default: current origin).

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
///
/// Fields `id`, `model`, and `usage` are deserialized from the server response
/// but not yet consumed by the frontend — they are kept to maintain a
/// complete API contract for future use.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ApiChoice>,
    pub usage: ApiUsage,
}

/// A single completion choice in a non-streaming response.
///
/// Fields `index` and `finish_reason` are kept for API completeness.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChoice {
    pub index: u32,
    pub message: ApiChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage statistics returned by the provider.
///
/// Kept for API completeness; not yet displayed in the UI.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Errors that can occur when calling the chat completions API.
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

/// Read the API base URL from the `window.__LIBRECHAT_API_URL__` JavaScript
/// property. Returns an empty string (i.e. relative URLs) when the property
/// is not set, which works correctly when the frontend is served by the same
/// origin as the backend.
fn api_base_url() -> String {
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

/// Send a non-streaming chat completion request to the backend.
///
/// Constructs a `POST /api/chat/completions` request with the given messages
/// and model, then awaits the full response. The model defaults to
/// [`DEFAULT_MODEL`] when an empty string is supplied.
pub async fn send_chat_request(
    messages: &[ApiChatMessage],
    model: &str,
) -> Result<ApiChatCompletionResponse, ApiError> {
    let base = api_base_url();
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

    let response = Request::post(&url)
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
}
