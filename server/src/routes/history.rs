//! Chat history API handlers for persistent conversations.
//!
//! Provides CRUD endpoints for conversations and messages backed by SQLite.
//! All handlers return `503 Service Unavailable` when the application was
//! started without a database pool.

use crate::database::{
    create_conversation, delete_conversation, get_conversation, get_messages, insert_messages,
    list_conversations, update_conversation,
};
use crate::routes::error::error_response;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// New conversation request.
#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

/// Conversation update request (all fields optional).
#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

/// A single message to append.
#[derive(Debug, Deserialize)]
pub struct AppendMessage {
    pub role: String,
    pub content: String,
    pub sequence: i64,
    #[serde(default)]
    pub is_error: bool,
}

/// Batch message append request.
#[derive(Debug, Deserialize)]
pub struct AppendMessagesRequest {
    pub messages: Vec<AppendMessage>,
}

/// Conversation response including ordered messages.
#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub messages: Vec<MessageResponse>,
}

/// Message representation in API responses.
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub sequence: i64,
    pub is_error: bool,
    pub created_at: Option<String>,
}

impl From<crate::database::Message> for MessageResponse {
    fn from(msg: crate::database::Message) -> Self {
        Self {
            id: msg.id,
            role: msg.role,
            content: msg.content,
            sequence: msg.sequence,
            is_error: msg.is_error != 0,
            created_at: msg.created_at,
        }
    }
}

fn no_db_error() -> impl IntoResponse {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "Database not available".to_string(),
    )
}

/// `GET /api/conversations` — list conversation summaries ordered by updated desc.
pub async fn list_conversations_handler(State(state): State<AppState>) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    match list_conversations(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            error!(error = %e, "failed to list conversations");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list conversations".to_string(),
            )
            .into_response()
        }
    }
}

/// `POST /api/conversations` — create a new conversation.
pub async fn create_conversation_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateConversationRequest>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    match create_conversation(
        pool,
        payload.title.as_deref(),
        payload.model.as_deref(),
        payload.provider.as_deref(),
    )
    .await
    {
        Ok(id) => {
            info!(conversation_id = id, "created conversation");
            match get_conversation(pool, id).await {
                Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
                Ok(None) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Created conversation not found".to_string(),
                )
                .into_response(),
                Err(e) => {
                    error!(error = %e, "failed to fetch created conversation");
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to fetch created conversation".to_string(),
                    )
                    .into_response()
                }
            }
        }
        Err(e) => {
            error!(error = %e, "failed to create conversation");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create conversation".to_string(),
            )
            .into_response()
        }
    }
}

/// `GET /api/conversations/{id}` — fetch a conversation with its messages.
pub async fn get_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    let conv = match get_conversation(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, conversation_id = id, "failed to fetch conversation");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch conversation".to_string(),
            )
            .into_response();
        }
    };

    let messages: Vec<MessageResponse> = match get_messages(pool, id).await {
        Ok(rows) => rows.into_iter().map(MessageResponse::from).collect(),
        Err(e) => {
            error!(error = %e, conversation_id = id, "failed to fetch messages");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch messages".to_string(),
            )
            .into_response();
        }
    };

    let response = ConversationResponse {
        id: conv.id,
        title: conv.title,
        model: conv.model,
        provider: conv.provider,
        created_at: conv.created_at,
        updated_at: conv.updated_at,
        messages,
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// `PATCH /api/conversations/{id}` — update conversation metadata.
pub async fn update_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateConversationRequest>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    match update_conversation(
        pool,
        id,
        payload.title.as_deref(),
        payload.model.as_deref(),
        payload.provider.as_deref(),
    )
    .await
    {
        Ok(true) => match get_conversation(pool, id).await {
            Ok(Some(row)) => (StatusCode::OK, Json(row)).into_response(),
            Ok(None) => error_response(
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
            .into_response(),
            Err(e) => {
                error!(error = %e, "failed to fetch updated conversation");
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch updated conversation".to_string(),
                )
                .into_response()
            }
        },
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            format!("Conversation {id} not found"),
        )
        .into_response(),
        Err(e) => {
            error!(error = %e, "failed to update conversation");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update conversation".to_string(),
            )
            .into_response()
        }
    }
}

/// `POST /api/conversations/{id}/messages` — append messages to a conversation.
pub async fn append_messages_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AppendMessagesRequest>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    // Verify the conversation exists.
    match get_conversation(pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
            .into_response();
        }
        Err(e) => {
            error!(error = %e, conversation_id = id, "failed to verify conversation");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify conversation".to_string(),
            )
            .into_response();
        }
    }

    let msgs: Vec<(String, String, i64, bool)> = payload
        .messages
        .into_iter()
        .map(|m| (m.role, m.content, m.sequence, m.is_error))
        .collect();

    match insert_messages(pool, id, &msgs).await {
        Ok(()) => {
            info!(
                conversation_id = id,
                message_count = msgs.len(),
                "appended messages"
            );
            let messages: Vec<MessageResponse> = match get_messages(pool, id).await {
                Ok(rows) => rows.into_iter().map(MessageResponse::from).collect(),
                Err(e) => {
                    error!(error = %e, "failed to fetch messages after insert");
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to fetch messages".to_string(),
                    )
                    .into_response();
                }
            };
            (StatusCode::OK, Json(messages)).into_response()
        }
        Err(e) => {
            error!(error = %e, conversation_id = id, "failed to insert messages");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to insert messages".to_string(),
            )
            .into_response()
        }
    }
}

/// `DELETE /api/conversations/{id}` — delete a conversation and its messages.
pub async fn delete_conversation_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return no_db_error().into_response(),
    };

    match delete_conversation(pool, id).await {
        Ok(true) => {
            info!(conversation_id = id, "deleted conversation");
            StatusCode::OK.into_response()
        }
        Ok(false) => error_response(
            StatusCode::NOT_FOUND,
            format!("Conversation {id} not found"),
        )
        .into_response(),
        Err(e) => {
            error!(error = %e, conversation_id = id, "failed to delete conversation");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete conversation".to_string(),
            )
            .into_response()
        }
    }
}
