//! Frontend API client for chat history endpoints.

use gloo_net::http::{Request, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::api::{resolve_api_base, ApiError};

/// Summary of a conversation from the history endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConversation {
    pub id: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A message within a conversation detail response.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiHistoryMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub sequence: i64,
    pub is_error: bool,
    pub created_at: Option<String>,
}

/// Full conversation with ordered messages.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConversationDetail {
    pub id: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub messages: Vec<ApiHistoryMessage>,
}

/// Request to create a conversation.
#[derive(Debug, Clone, Serialize)]
pub struct ApiCreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Request to update a conversation.
#[derive(Debug, Clone, Serialize)]
pub struct ApiUpdateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// A single message to append.
#[derive(Debug, Clone, Serialize)]
pub struct ApiAppendMessage {
    pub role: String,
    pub content: String,
    pub sequence: i64,
    pub is_error: bool,
}

/// Request to append messages.
#[derive(Debug, Clone, Serialize)]
pub struct ApiAppendMessagesRequest {
    pub messages: Vec<ApiAppendMessage>,
}

fn builder_with_auth(method: &str, url: &str, auth_key: &str) -> RequestBuilder {
    let builder = match method {
        "GET" => Request::get(url),
        "POST" => Request::post(url),
        "PATCH" => Request::patch(url),
        "DELETE" => Request::delete(url),
        _ => panic!("Unsupported HTTP method: {method}"),
    };

    if !auth_key.is_empty() {
        builder.header("Authorization", &format!("Bearer {auth_key}"))
    } else {
        builder
    }
}

/// Fetch the list of saved conversations.
pub async fn fetch_conversations(
    custom_endpoint: &str,
    auth_key: &str,
) -> Result<Vec<ApiConversation>, ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations");
    let response = builder_with_auth("GET", &url, auth_key)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    response
        .json::<Vec<ApiConversation>>()
        .await
        .map_err(|e| ApiError::Parse(format!("invalid conversation list: {e}")))
}

/// Create a new conversation on the backend.
pub async fn create_conversation(
    custom_endpoint: &str,
    auth_key: &str,
    request: &ApiCreateConversationRequest,
) -> Result<ApiConversation, ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations");
    let response = builder_with_auth("POST", &url, auth_key)
        .json(&request)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    response
        .json::<ApiConversation>()
        .await
        .map_err(|e| ApiError::Parse(format!("invalid conversation: {e}")))
}

/// Fetch a single conversation with its messages.
pub async fn fetch_conversation(
    custom_endpoint: &str,
    auth_key: &str,
    id: i64,
) -> Result<ApiConversationDetail, ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations/{id}");
    let response = builder_with_auth("GET", &url, auth_key)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    response
        .json::<ApiConversationDetail>()
        .await
        .map_err(|e| ApiError::Parse(format!("invalid conversation detail: {e}")))
}

/// Update conversation metadata.
pub async fn update_conversation(
    custom_endpoint: &str,
    auth_key: &str,
    id: i64,
    request: &ApiUpdateConversationRequest,
) -> Result<ApiConversation, ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations/{id}");
    let response = builder_with_auth("PATCH", &url, auth_key)
        .json(&request)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    response
        .json::<ApiConversation>()
        .await
        .map_err(|e| ApiError::Parse(format!("invalid conversation: {e}")))
}

/// Append messages to a conversation.
pub async fn append_messages(
    custom_endpoint: &str,
    auth_key: &str,
    id: i64,
    request: &ApiAppendMessagesRequest,
) -> Result<Vec<ApiHistoryMessage>, ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations/{id}/messages");
    let response = builder_with_auth("POST", &url, auth_key)
        .json(&request)
        .map_err(|e| ApiError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    response
        .json::<Vec<ApiHistoryMessage>>()
        .await
        .map_err(|e| ApiError::Parse(format!("invalid message list: {e}")))
}

/// Delete a conversation.
pub async fn delete_conversation(
    custom_endpoint: &str,
    auth_key: &str,
    id: i64,
) -> Result<(), ApiError> {
    let base = resolve_api_base(custom_endpoint);
    let url = format!("{base}/api/conversations/{id}");
    let response = builder_with_auth("DELETE", &url, auth_key)
        .send()
        .await
        .map_err(|e| ApiError::Network(e.to_string()))?;

    if !response.ok() && response.status() != 404 {
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::Http {
            status: response.status(),
            body,
        });
    }

    Ok(())
}
