//! Integration tests for settings persistence (Issue #29).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use server::app;
use server::database::{init_pool, run_migrations};
use server::providers::{LlmProvider, OpenAiProvider};
use server::state::AppState;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

async fn test_app_with_db(database_url: &str) -> (axum::Router, TempDir) {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp_dir.path().join("index.html"),
        "<!doctype html><title>test</title>",
    )
    .expect("write index.html");

    let pool = init_pool(database_url).await.expect("pool");
    run_migrations(&pool).await.expect("migrations");

    let state = AppState {
        db_pool: Some(pool),
        provider: Arc::new(OpenAiProvider::from_env()) as Arc<dyn LlmProvider>,
        static_dir: temp_dir.path().to_path_buf(),
    };

    (app(state), temp_dir)
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
        .expect("build request")
}

fn put_request(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("PUT")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

async fn read_json_body(response: axum::response::Response) -> Value {
    let status = response.status();
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let body_text = String::from_utf8_lossy(&body_bytes);
    serde_json::from_str(&body_text)
        .unwrap_or_else(|_| panic!("invalid JSON body: status={status}, text={body_text}"))
}

async fn read_body_text(response: axum::response::Response) -> String {
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    String::from_utf8_lossy(&body_bytes).into_owned()
}

// ---- Default settings ----

#[tokio::test]
async fn test_settings_defaults_on_fresh_database() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let response = app
        .oneshot(get_request("/api/settings"))
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = read_json_body(response).await;
    assert_eq!(body["api_endpoint"], "");
    assert_eq!(body["auth_key"], "");
    assert_eq!(body["model"], "llama3");
    assert_eq!(body["sidebar_collapsed"], false);
}

// ---- Update settings ----

#[tokio::test]
async fn test_update_settings_persists_values() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let update = json!({
        "api_endpoint": "http://localhost:11434",
        "auth_key": "sk-test",
        "model": "gpt-4",
        "temperature": 0.7,
        "max_tokens": 2048,
        "sidebar_collapsed": true
    });

    let response = app
        .clone()
        .oneshot(put_request("/api/settings", update))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(get_request("/api/settings"))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    let body = read_json_body(response).await;
    assert_eq!(body["api_endpoint"], "http://localhost:11434");
    assert_eq!(body["auth_key"], "sk-test");
    assert_eq!(body["model"], "gpt-4");
    assert_eq!(body["temperature"], 0.7);
    assert_eq!(body["max_tokens"], 2048);
    assert_eq!(body["sidebar_collapsed"], true);
}

// ---- Partial update ----

#[tokio::test]
async fn test_partial_update_settings() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Set initial values
    let update = json!({
        "api_endpoint": "http://localhost:11434",
        "auth_key": "sk-test",
        "model": "gpt-4",
        "sidebar_collapsed": true
    });
    let response = app
        .clone()
        .oneshot(put_request("/api/settings", update))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    // Partial update: only endpoint and model
    let partial = json!({
        "api_endpoint": "http://localhost:8080",
        "model": "llama3",
    });
    let response = app
        .clone()
        .oneshot(put_request("/api/settings", partial))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(get_request("/api/settings"))
        .await
        .expect("request");
    let body = read_json_body(response).await;
    assert_eq!(body["api_endpoint"], "http://localhost:8080");
    assert_eq!(body["auth_key"], "sk-test");
    assert_eq!(body["model"], "llama3");
    assert_eq!(body["sidebar_collapsed"], true);
}

// ---- Validation failures ----

#[tokio::test]
async fn test_update_settings_rejects_invalid_temperature() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let update = json!({
        "api_endpoint": "http://localhost:11434",
        "temperature": 3.0,
    });

    let response = app
        .oneshot(put_request("/api/settings", update))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_settings_rejects_negative_max_tokens() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let update = json!({
        "api_endpoint": "http://localhost:11434",
        "max_tokens": -1,
    });

    let response = app
        .oneshot(put_request("/api/settings", update))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ---- Security: secrets not exposed in errors ----

#[tokio::test]
async fn test_error_response_does_not_expose_auth_key() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("settings_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Force an error by sending malformed JSON.
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .method("PUT")
                .header("Content-Type", "application/json")
                .body(Body::from("not-json"))
                .expect("build request"),
        )
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body_text = read_body_text(response).await;
    assert!(
        !body_text.contains("sk-"),
        "error response should not contain secrets"
    );
}

// ---- Persistence across new pool ----

#[tokio::test]
async fn test_settings_persist_across_new_app_state() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("persist_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));

    // First app instance
    let (app1, _temp_dir1) = test_app_with_db(&url).await;

    let update = json!({
        "api_endpoint": "http://ollama.local:11434",
        "auth_key": "sk-persist",
        "model": "mistral",
        "temperature": 0.5,
        "max_tokens": 1024,
        "sidebar_collapsed": true
    });
    let response = app1
        .oneshot(put_request("/api/settings", update))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    // Second app instance with same database
    let (app2, _temp_dir2) = test_app_with_db(&url).await;

    let response = app2
        .oneshot(get_request("/api/settings"))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::OK);

    let body = read_json_body(response).await;
    assert_eq!(body["api_endpoint"], "http://ollama.local:11434");
    assert_eq!(body["auth_key"], "sk-persist");
    assert_eq!(body["model"], "mistral");
    assert_eq!(body["temperature"], 0.5);
    assert_eq!(body["max_tokens"], 1024);
    assert_eq!(body["sidebar_collapsed"], true);
}

// ---- No database pool returns 503 ----

#[tokio::test]
async fn test_settings_returns_503_without_database() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp_dir.path().join("index.html"),
        "<!doctype html><title>test</title>",
    )
    .expect("write index.html");

    let state = AppState {
        db_pool: None,
        provider: Arc::new(OpenAiProvider::from_env()) as Arc<dyn LlmProvider>,
        static_dir: temp_dir.path().to_path_buf(),
    };
    let app = server::app(state);

    let response = app
        .oneshot(get_request("/api/settings"))
        .await
        .expect("request");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
