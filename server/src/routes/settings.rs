//! Application settings API handlers.
//!
//! Provides `GET /api/settings` and `PUT /api/settings` for durable
//! user preferences backed by SQLite.

use crate::database::{get_settings, upsert_settings};
use crate::routes::error::error_response;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Request payload to update settings.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub auth_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<i64>,
    #[serde(default)]
    pub sidebar_collapsed: Option<bool>,
}

/// Safe settings response — mirrors the database row but serialises the
/// boolean `sidebar_collapsed` field as JSON boolean.
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub api_endpoint: String,
    pub auth_key: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    pub sidebar_collapsed: bool,
}

impl From<crate::database::AppSettingsRow> for SettingsResponse {
    fn from(row: crate::database::AppSettingsRow) -> Self {
        Self {
            api_endpoint: row.api_endpoint,
            auth_key: row.auth_key,
            model: row.model,
            temperature: row.temperature,
            max_tokens: row.max_tokens,
            sidebar_collapsed: row.sidebar_collapsed != 0,
        }
    }
}

fn service_unavailable_error() -> impl IntoResponse {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "Database not available".to_string(),
    )
}

/// `GET /api/settings` — return persisted application settings.
///
/// When no row exists yet (fresh database) the current defaults are
/// returned: empty endpoint, empty auth key, model `llama3`, and
/// sidebar expanded.
pub async fn get_settings_handler(State(state): State<AppState>) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return service_unavailable_error().into_response(),
    };

    match get_settings(pool).await {
        Ok(row) => (StatusCode::OK, Json(SettingsResponse::from(row))).into_response(),
        Err(e) => {
            error!(error = %e, "failed to read settings");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read settings".to_string(),
            )
            .into_response()
        }
    }
}

const MAX_ENDPOINT_LEN: usize = 2048;
const MAX_AUTH_KEY_LEN: usize = 2048;
const MAX_MODEL_LEN: usize = 256;

/// `PUT /api/settings` — replace the settings document after validation.
///
/// Partial updates are supported: omitted fields leave the existing value
/// unchanged.
pub async fn update_settings_handler(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    let pool = match &state.db_pool {
        Some(p) => p,
        None => return service_unavailable_error().into_response(),
    };

    // Read current row so partial updates can reuse unchanged fields.
    let current = match get_settings(pool).await {
        Ok(row) => row,
        Err(e) => {
            error!(error = %e, "failed to read current settings for update");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read current settings".to_string(),
            )
            .into_response();
        }
    };

    let api_endpoint = payload.api_endpoint.unwrap_or(current.api_endpoint);
    if api_endpoint.len() > MAX_ENDPOINT_LEN {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("API endpoint exceeds {MAX_ENDPOINT_LEN} characters"),
        )
        .into_response();
    }

    let auth_key = payload.auth_key.unwrap_or(current.auth_key);
    if auth_key.len() > MAX_AUTH_KEY_LEN {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("Auth key exceeds {MAX_AUTH_KEY_LEN} characters"),
        )
        .into_response();
    }

    let model = payload.model.unwrap_or(current.model);
    if model.len() > MAX_MODEL_LEN {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("Model name exceeds {MAX_MODEL_LEN} characters"),
        )
        .into_response();
    }

    let temperature = payload.temperature.or(current.temperature);
    if let Some(t) = temperature {
        if !(0.0..=2.0).contains(&t) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Temperature must be between 0.0 and 2.0".to_string(),
            )
            .into_response();
        }
    }

    let max_tokens = payload.max_tokens.or(current.max_tokens);
    if let Some(mt) = max_tokens {
        if mt < 1 {
            return error_response(
                StatusCode::BAD_REQUEST,
                "max_tokens must be at least 1".to_string(),
            )
            .into_response();
        }
    }

    let sidebar_collapsed = payload
        .sidebar_collapsed
        .unwrap_or(current.sidebar_collapsed != 0);

    match upsert_settings(
        pool,
        &api_endpoint,
        &auth_key,
        &model,
        temperature,
        max_tokens,
        sidebar_collapsed,
    )
    .await
    {
        Ok(()) => {
            let row = match get_settings(pool).await {
                Ok(r) => r,
                Err(e) => {
                    error!(error = %e, "failed to read settings after update");
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to read settings after update".to_string(),
                    )
                    .into_response();
                }
            };
            (StatusCode::OK, Json(SettingsResponse::from(row))).into_response()
        }
        Err(e) => {
            error!(error = %e, "failed to update settings");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update settings".to_string(),
            )
            .into_response()
        }
    }
}
