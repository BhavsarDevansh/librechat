//! Non-streaming chat completion API handler.

use crate::providers::ChatCompletionRequest;
use crate::routes::error::{error_response, map_provider_error};
use crate::state::AppState;
use axum::extract::{rejection::JsonRejection, Json, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::{error, info, warn};

/// `POST /api/chat/completions` — forwards a chat completion request to the
/// configured provider and returns the full JSON response.
pub async fn chat_completion(
    State(state): State<AppState>,
    payload: Result<Json<ChatCompletionRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match payload {
        Ok(Json(request)) => request,
        Err(error) => {
            warn!(error = %error, "failed to parse chat completion request");
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse JSON request: {error}"),
            );
        }
    };

    info!(
        model = %request.model,
        message_count = request.messages.len(),
        "forwarding chat completion request"
    );

    match state.provider.chat_completion(request).await {
        Ok(response) => {
            info!(
                model = %response.model,
                choice_count = response.choices.len(),
                "chat completion succeeded"
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => {
            let (status, message) = map_provider_error(&error);
            error!(status = %status, error = %error, "chat completion failed");
            error_response(status, message)
        }
    }
}
