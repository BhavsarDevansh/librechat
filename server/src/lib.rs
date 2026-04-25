//! LibreChat server library.
//!
//! Exposes the [`app`] constructor, [`AppState`], [`resolve_port`], and the
//! [`providers`] module for use by the binary entry point and integration
//! tests alike.

pub mod providers;
pub mod state;

mod routes;

use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::warn;

use state::AppState;

/// Environment variable containing a comma-separated CORS allowlist.
const ALLOWED_ORIGINS_ENV: &str = "LIBRECHAT_ALLOWED_ORIGINS";

/// Default origins allowed for local development when no allowlist is set.
const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://127.0.0.1:3000",
    "http://localhost:3001",
    "http://127.0.0.1:3001",
    "http://localhost:4173",
    "http://127.0.0.1:4173",
    "http://localhost:5173",
    "http://127.0.0.1:5173",
    "http://localhost:8080",
    "http://127.0.0.1:8080",
];

/// Build the Axum application router with all middleware and state wired in.
///
/// Routes:
/// - `GET /health` — returns `{"status":"ok"}` with `200 OK`
/// - `POST /api/chat/completions` — proxies non-streaming chat completions to
///   the configured provider
/// - `POST /api/chat/completions/stream` — streams chat completions to the
///   client using Server-Sent Events
///
/// Static files:
/// - `/` — `ServeDir` serves the Leptos WASM frontend from the configured
///   static directory, with `index.html` appended for directory requests and
///   SPA-style fallback for unknown paths.
///
/// API routes are registered before the static file catch-all so they take
/// priority.
///
/// Middleware (applied in order):
/// - `TraceLayer` — structured request/response logging
/// - `CorsLayer` — allows a configured origin allowlist, defaulting to common
///   local development origins when `LIBRECHAT_ALLOWED_ORIGINS` is unset
pub fn app(state: AppState) -> Router {
    let static_dir = &state.static_dir;
    let index_path = static_dir.join("index.html");

    let serve_dir = ServeDir::new(static_dir)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index_path));

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/api/chat/completions", post(routes::chat::chat_completion))
        .route(
            "/api/chat/completions/stream",
            post(routes::chat_stream::chat_completion_stream),
        )
        .layer(TraceLayer::new_for_http())
        .layer(build_cors_layer())
        .fallback_service(serve_dir)
        .with_state(state)
}

/// Read the server port from the `LIBRECHAT_PORT` environment variable,
/// defaulting to `3000` if unset or invalid.
#[must_use]
pub fn resolve_port() -> u16 {
    std::env::var("LIBRECHAT_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000)
}

fn build_cors_layer() -> CorsLayer {
    let allowed_origins = resolve_allowed_origins();

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(false)
}

fn resolve_allowed_origins() -> Vec<HeaderValue> {
    match std::env::var(ALLOWED_ORIGINS_ENV) {
        Ok(configured) => configured
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .filter_map(parse_origin_header)
            .collect(),
        Err(_) => DEFAULT_ALLOWED_ORIGINS
            .iter()
            .filter_map(|origin| parse_origin_header(origin))
            .collect(),
    }
}

fn parse_origin_header(origin: &str) -> Option<HeaderValue> {
    match HeaderValue::from_str(origin) {
        Ok(value) => Some(value),
        Err(error) => {
            warn!(origin, %error, "ignoring invalid CORS origin");
            None
        }
    }
}
