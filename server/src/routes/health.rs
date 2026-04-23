//! Health check handler.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

/// `GET /health` — returns `{"status": "ok"}` with `200 OK` and
/// `Content-Type: application/json`.
///
/// `axum::Json` automatically sets the `Content-Type` header.
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(json!({"status": "ok"})))
}
