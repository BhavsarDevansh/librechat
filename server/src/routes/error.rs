//! Shared error handling utilities for route handlers.

use crate::providers::ProviderError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

/// Map a [`ProviderError`] to an HTTP status code and a human-readable message.
pub(crate) fn map_provider_error(error: &ProviderError) -> (StatusCode, String) {
    match error {
        ProviderError::ApiError { status, message } => {
            let status = match *status {
                400..=599 => StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY),
                _ => StatusCode::BAD_GATEWAY,
            };
            (status, message.clone())
        }
        ProviderError::ConnectionFailed(_) => (StatusCode::BAD_GATEWAY, error.to_string()),
        ProviderError::InvalidResponse(_) => (StatusCode::BAD_GATEWAY, error.to_string()),
        ProviderError::StreamEnded => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        ProviderError::StreamingNotSupported => {
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        }
    }
}

/// Build a JSON error response with the given status code and message.
pub(crate) fn error_response(status: StatusCode, message: String) -> axum::response::Response {
    (status, axum::Json(ErrorResponse { error: message })).into_response()
}
