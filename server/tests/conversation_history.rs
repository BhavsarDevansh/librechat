//! Integration tests for persistent chat history APIs (Issue #28).

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
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

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("parse json")
}

// ---- GET /api/conversations ----

#[tokio::test]
async fn test_list_conversations_empty() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/conversations")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    let conversations = body.as_array().expect("array");
    assert!(conversations.is_empty(), "should return empty list");
}

// ---- POST /api/conversations ----

#[tokio::test]
async fn test_create_conversation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let payload = serde_json::json!({
        "title": "New Chat",
        "model": "llama3",
        "provider": "ollama"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_json(response).await;
    assert!(body.get("id").is_some(), "should return id");
    assert_eq!(body["title"], "New Chat");
    assert_eq!(body["model"], "llama3");
    assert_eq!(body["provider"], "ollama");
}

// ---- GET /api/conversations/{id} ----

#[tokio::test]
async fn test_fetch_conversation_with_messages() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Create conversation
    let create_payload = serde_json::json!({ "title": "Test Chat" });
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build request");
    let create_response = app.clone().oneshot(create_request).await.expect("oneshot");
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = response_body_json(create_response).await;
    let conv_id = create_body["id"].as_i64().expect("id");

    // Append messages
    let msg_payload = serde_json::json!({
        "messages": [
            { "role": "user", "content": "Hello", "sequence": 0 },
            { "role": "assistant", "content": "Hi there", "sequence": 1 }
        ]
    });
    let msg_request = Request::builder()
        .method("POST")
        .uri(format!("/api/conversations/{conv_id}/messages"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(msg_payload.to_string()))
        .expect("build request");
    let msg_response = app.clone().oneshot(msg_request).await.expect("oneshot");
    assert_eq!(msg_response.status(), StatusCode::OK);

    // Fetch conversation
    let fetch_request = Request::builder()
        .method("GET")
        .uri(format!("/api/conversations/{conv_id}"))
        .body(Body::empty())
        .expect("build request");
    let fetch_response = app.clone().oneshot(fetch_request).await.expect("oneshot");
    assert_eq!(fetch_response.status(), StatusCode::OK);

    let fetch_body = response_body_json(fetch_response).await;
    assert_eq!(fetch_body["id"], conv_id);
    assert_eq!(fetch_body["title"], "Test Chat");
    let messages = fetch_body["messages"].as_array().expect("messages array");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "Hello");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "Hi there");
}

// ---- PATCH /api/conversations/{id} ----

#[tokio::test]
async fn test_update_conversation_title() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Create conversation
    let create_payload = serde_json::json!({ "title": "Old Title" });
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build request");
    let create_response = app.clone().oneshot(create_request).await.expect("oneshot");
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = response_body_json(create_response).await;
    let conv_id = create_body["id"].as_i64().expect("id");

    // Update title
    let patch_payload = serde_json::json!({ "title": "New Title" });
    let patch_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/conversations/{conv_id}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(patch_payload.to_string()))
        .expect("build request");
    let patch_response = app.clone().oneshot(patch_request).await.expect("oneshot");
    assert_eq!(patch_response.status(), StatusCode::OK);

    let patch_body = response_body_json(patch_response).await;
    assert_eq!(patch_body["title"], "New Title");
}

// ---- DELETE /api/conversations/{id} ----

#[tokio::test]
async fn test_delete_conversation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Create conversation
    let create_payload = serde_json::json!({ "title": "To Delete" });
    let create_request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(create_payload.to_string()))
        .expect("build request");
    let create_response = app.clone().oneshot(create_request).await.expect("oneshot");
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = response_body_json(create_response).await;
    let conv_id = create_body["id"].as_i64().expect("id");

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/conversations/{conv_id}"))
        .body(Body::empty())
        .expect("build request");
    let delete_response = app.clone().oneshot(delete_request).await.expect("oneshot");
    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify deleted
    let fetch_request = Request::builder()
        .method("GET")
        .uri(format!("/api/conversations/{conv_id}"))
        .body(Body::empty())
        .expect("build request");
    let fetch_response = app.oneshot(fetch_request).await.expect("oneshot");
    assert_eq!(fetch_response.status(), StatusCode::NOT_FOUND);
}

// ---- List ordering ----

#[tokio::test]
async fn test_list_conversations_ordered_by_updated_desc() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    // Create first
    let c1_payload = serde_json::json!({ "title": "First" });
    let c1_request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(c1_payload.to_string()))
        .expect("build request");
    let c1_response = app.clone().oneshot(c1_request).await.expect("oneshot");
    assert_eq!(c1_response.status(), StatusCode::OK);
    let c1_body = response_body_json(c1_response).await;
    let id1 = c1_body["id"].as_i64().expect("id");

    // Create second
    let c2_payload = serde_json::json!({ "title": "Second" });
    let c2_request = Request::builder()
        .method("POST")
        .uri("/api/conversations")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(c2_payload.to_string()))
        .expect("build request");
    let c2_response = app.clone().oneshot(c2_request).await.expect("oneshot");
    assert_eq!(c2_response.status(), StatusCode::OK);
    let c2_body = response_body_json(c2_response).await;
    let _id2 = c2_body["id"].as_i64().expect("id");

    // Sleep to cross a second boundary so updated_at changes
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // Sleep to cross a second boundary so updated_at changes
    let patch_payload = serde_json::json!({ "title": "First Updated" });
    let patch_request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/conversations/{id1}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(patch_payload.to_string()))
        .expect("build request");
    let patch_response = app.clone().oneshot(patch_request).await.expect("oneshot");
    assert_eq!(patch_response.status(), StatusCode::OK);

    // List should have First Updated first, then Second
    let list_request = Request::builder()
        .method("GET")
        .uri("/api/conversations")
        .body(Body::empty())
        .expect("build request");
    let list_response = app.oneshot(list_request).await.expect("oneshot");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = response_body_json(list_response).await;
    let list = list_body.as_array().expect("array");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["title"], "First Updated");
    assert_eq!(list[1]["title"], "Second");
}

// ---- Not found handling ----

#[tokio::test]
async fn test_fetch_nonexistent_conversation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));
    let (app, _temp_dir) = test_app_with_db(&url).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/conversations/9999")
        .body(Body::empty())
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
