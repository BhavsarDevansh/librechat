//! Models API route handler.
//!
//! `GET /api/models` — returns the list of available models from the
//! configured LLM provider.

use crate::providers::ModelInfo;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use tracing::{error, info};

/// JSON response payload for the models list endpoint.
#[derive(Serialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// `GET /api/models` — returns available models from the configured provider.
pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    info!("listing available models");

    match state.provider.list_models().await {
        Ok(models) => {
            info!(count = models.len(), "listed models");
            (StatusCode::OK, axum::Json(ModelsResponse { models })).into_response()
        }
        Err(error) => {
            error!(error = %error, "failed to list models");
            (
                StatusCode::BAD_GATEWAY,
                axum::Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response()
        }
    }
}
