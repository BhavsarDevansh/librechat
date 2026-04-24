//! Integration tests for static file serving (Issue #4).
//!
//! Verifies that the Axum server serves the Leptos WASM frontend as static
//! files with SPA-style fallback routing.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use server::app;
use server::state::AppState;

use std::path::PathBuf;
use std::sync::Mutex;

/// Mutex serialising tests that mutate the `LIBRECHAT_STATIC_DIR` environment
/// variable. Cargo runs tests in parallel, so unsynchronised `set_var` /
/// `remove_var` calls would cause data races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Helper: create a temporary directory with test static files.
///
/// Writes an `index.html` and a `test-asset.txt` so that tests can verify
/// static file serving without depending on a real frontend build.
fn create_temp_static_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let dir_path = dir.path().to_path_buf();

    std::fs::write(
        dir.path().join("index.html"),
        r#"<!DOCTYPE html><html><head><title>Test</title></head><body>SPA</body></html>"#,
    )
    .expect("write index.html");

    std::fs::write(dir.path().join("test-asset.txt"), "hello from static")
        .expect("write test-asset.txt");

    (dir, dir_path)
}

/// Helper: build the app with a specific static directory.
fn test_app_with_static_dir(static_dir: PathBuf) -> axum::Router {
    app(AppState::with_static_dir(static_dir))
}

// ---- Acceptance criteria tests ----

#[tokio::test]
async fn test_get_root_returns_index_html() {
    let (_temp, dir_path) = create_temp_static_dir();
    let app = test_app_with_static_dir(dir_path);

    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("valid utf-8");
    assert!(
        html.contains("SPA"),
        "GET / should return index.html content, got: {html}"
    );
}

#[tokio::test]
async fn test_static_file_served_with_correct_content_type() {
    let (_temp, dir_path) = create_temp_static_dir();
    let app = test_app_with_static_dir(dir_path);

    let req = Request::builder()
        .uri("/test-asset.txt")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header missing");
    assert!(
        ct.to_str()
            .expect("content-type not readable")
            .starts_with("text/plain"),
        "text file should have text/plain content-type, got: {ct:?}"
    );
}

#[tokio::test]
async fn test_health_endpoint_still_returns_json() {
    let (_temp, dir_path) = create_temp_static_dir();
    let app = test_app_with_static_dir(dir_path);

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header missing");
    assert!(
        ct.to_str()
            .expect("content-type not readable")
            .starts_with("application/json"),
        "health endpoint should return application/json, got: {ct:?}"
    );
}

#[tokio::test]
async fn test_nonexistent_path_returns_index_html_spa_fallback() {
    let (_temp, dir_path) = create_temp_static_dir();
    let app = test_app_with_static_dir(dir_path);

    let req = Request::builder()
        .uri("/nonexistent-path")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    // SPA fallback: unknown paths should serve index.html
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("valid utf-8");
    assert!(
        html.contains("SPA"),
        "unknown path should fall back to index.html, got: {html}"
    );
}

#[tokio::test]
async fn test_static_dir_env_var_override() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let (_temp, dir_path) = create_temp_static_dir();

    let env_key = "LIBRECHAT_STATIC_DIR";
    let original = std::env::var(env_key).ok();
    // Safety: guarded by ENV_LOCK to serialise parallel test access.
    unsafe {
        std::env::set_var(env_key, dir_path.to_str().expect("path to str"));
    }

    let state = AppState::new();
    let resolved_dir = state.static_dir.clone();

    // Restore env var
    unsafe {
        if let Some(val) = original {
            std::env::set_var(env_key, val);
        } else {
            std::env::remove_var(env_key);
        }
    }

    assert_eq!(
        resolved_dir, dir_path,
        "LIBRECHAT_STATIC_DIR should override the default static directory"
    );
}

#[tokio::test]
async fn test_default_static_dir_is_frontend_dist() {
    let _lock = ENV_LOCK.lock().expect("env lock");

    let env_key = "LIBRECHAT_STATIC_DIR";
    let original = std::env::var(env_key).ok();
    // Safety: guarded by ENV_LOCK to serialise parallel test access.
    unsafe {
        std::env::remove_var(env_key);
    }

    let state = AppState::new();
    let default_dir = state.static_dir.clone();

    // Restore env var
    unsafe {
        if let Some(val) = original {
            std::env::set_var(env_key, val);
        }
    }

    assert!(
        default_dir.ends_with("frontend/dist"),
        "default static_dir should end with 'frontend/dist', got: {default_dir:?}"
    );
}

#[tokio::test]
async fn test_with_static_dir_constructor() {
    let custom_dir = PathBuf::from("/tmp/custom-static");
    let state = AppState::with_static_dir(custom_dir.clone());
    assert_eq!(
        state.static_dir, custom_dir,
        "with_static_dir should set the static_dir field"
    );
}
