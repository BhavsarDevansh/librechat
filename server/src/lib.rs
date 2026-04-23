//! LibreChat server library.
//!
//! Exposes the [`app`] constructor, [`AppState`], and [`resolve_port`] for use
//! by the binary entry point and integration tests alike.

pub mod state;

mod routes;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use state::AppState;

/// Build the Axum application router with all middleware and state wired in.
///
/// Routes:
/// - `GET /health` — returns `{"status":"ok"}` with `200 OK`
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
/// - `CorsLayer::permissive()` — allows all origins (development mode)
pub fn app(state: AppState) -> Router {
    let static_dir = &state.static_dir;
    let index_path = static_dir.join("index.html");

    let serve_dir = ServeDir::new(static_dir)
        .append_index_html_on_directories(true)
        .fallback(ServeFile::new(index_path));

    Router::new()
        .route("/health", axum::routing::get(routes::health::health))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
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
