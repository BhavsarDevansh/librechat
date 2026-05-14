//! LibreChat server binary — starts the Axum HTTP server.
//!
//! Configures structured logging via `tracing-subscriber`, resolves the listen
//! port from `LIBRECHAT_PORT` (default `3000`), initialises the SQLite
//! persistence layer, and serves the application.

use server::{app, resolve_port, state::AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("server=info".parse().expect("directive")),
        )
        .init();

    let port = resolve_port();
    let addr = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind listener");

    tracing::info!("Listening on {addr}");

    let init_timeout = std::time::Duration::from_secs(30);
    let state = match tokio::time::timeout(init_timeout, AppState::init()).await {
        Ok(Ok(state)) => state,
        Ok(Err(e)) => {
            tracing::error!("Database initialisation failed: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            tracing::error!("Database initialisation timed out after {init_timeout:?}");
            std::process::exit(1);
        }
    };

    axum::serve(listener, app(state))
        .await
        .expect("server error");
}
