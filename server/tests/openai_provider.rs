//! Integration tests for the OpenAI-compatible provider client (Issue #6).

use server::providers::OpenAiProvider;
use server::providers::{
    ChatCompletionRequest, ChatMessage, LlmProvider, MessageRole, ProviderError,
};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::post;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Shared state for the mock server to record received headers.
#[derive(Clone, Default)]
struct MockState {
    /// Captured Authorization header value, if any.
    auth_header: Arc<Mutex<Option<String>>>,
}

/// Wait for a TCP port to become accepting connections.
async fn wait_for_ready(port: u16) {
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("mock server on port {port} never became ready");
}

/// Spins up a mock OpenAI-compatible server on a random port and returns
/// `(base_url, server_handle)` where `base_url` includes the port.
async fn spawn_mock_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let state = MockState::default();

    let app = Router::new()
        .route("/v1/chat/completions", post(mock_chat_completions_handler))
        .with_state(state);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, handle)
}

/// Spins up a mock server that records the Authorization header.
/// Returns `(base_url, auth_header_capture, server_handle)`.
async fn spawn_auth_recording_server() -> (
    String,
    Arc<Mutex<Option<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let state = MockState::default();
    let auth_capture = state.auth_header.clone();

    let app = Router::new()
        .route("/v1/chat/completions", post(mock_chat_completions_handler))
        .with_state(state);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, auth_capture, handle)
}

/// Default mock handler: returns a well-formed ChatCompletionResponse.
/// Records the Authorization header in the shared state.
async fn mock_chat_completions_handler(
    State(state): State<MockState>,
    headers: axum::http::HeaderMap,
    body: axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    // Record the Authorization header value.
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        let val = auth.to_str().unwrap_or("").to_string();
        *state.auth_header.lock().expect("lock") = Some(val);
    }

    // Echo back a response shaped like the OpenAI API.
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("test-model");

    let response = json!({
        "id": "chatcmpl-test123",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello from mock server!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    });

    (StatusCode::OK, axum::Json(response))
}

/// Builds a minimal `ChatCompletionRequest` for tests.
fn test_request(model: &str) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
    }
}

// ── Construction tests ───────────────────────────────────────────────────────

#[test]
fn test_openai_provider_new() {
    let provider = OpenAiProvider::new(
        "http://localhost:11434".to_string(),
        None,
        "llama3".to_string(),
    );
    assert_eq!(provider.name(), "OpenAI-compatible");
    assert_eq!(provider.base_url(), "http://localhost:11434");
    assert_eq!(provider.model(), "llama3");
}

#[test]
fn test_openai_provider_new_trims_trailing_slash() {
    let provider = OpenAiProvider::new(
        "http://localhost:11434/".to_string(),
        None,
        "llama3".to_string(),
    );
    assert_eq!(provider.base_url(), "http://localhost:11434");
}

#[test]
fn test_openai_provider_new_empty_api_key_becomes_none() {
    let provider = OpenAiProvider::new(
        "http://localhost:11434".to_string(),
        Some("".to_string()),
        "llama3".to_string(),
    );
    assert_eq!(provider.api_key(), None);
}

#[test]
fn test_openai_provider_new_with_api_key() {
    let provider = OpenAiProvider::new(
        "https://api.openai.com".to_string(),
        Some("sk-test-key".to_string()),
        "gpt-4o-mini".to_string(),
    );
    assert_eq!(provider.api_key(), Some("sk-test-key"));
}

/// Merged env-var test to avoid parallel races on process-global state.
#[test]
fn test_openai_provider_from_env_defaults_and_custom() {
    // --- Defaults when env vars are unset ---
    // SAFETY: This test owns the env vars exclusively; it is the only test
    // that mutates these variables.
    unsafe {
        std::env::remove_var("LLM_BASE_URL");
        std::env::remove_var("LLM_API_KEY");
        std::env::remove_var("LLM_MODEL");
    }

    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.base_url(), "http://localhost:11434");
    assert_eq!(provider.api_key(), None);
    assert_eq!(provider.model(), "llama3");

    // --- Custom values ---
    // SAFETY: Same rationale — exclusive ownership within this test.
    unsafe {
        std::env::set_var("LLM_BASE_URL", "http://custom:1234");
        std::env::set_var("LLM_API_KEY", "sk-custom-key");
        std::env::set_var("LLM_MODEL", "gpt-4o");
    }

    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.base_url(), "http://custom:1234");
    assert_eq!(provider.api_key(), Some("sk-custom-key"));
    assert_eq!(provider.model(), "gpt-4o");

    // --- Empty API key treated as None ---
    // SAFETY: Same rationale.
    unsafe {
        std::env::set_var("LLM_API_KEY", "");
    }

    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.api_key(), None);

    // --- Cleanup ---
    // SAFETY: Same rationale.
    unsafe {
        std::env::remove_var("LLM_BASE_URL");
        std::env::remove_var("LLM_API_KEY");
        std::env::remove_var("LLM_MODEL");
    }
}

// ── Non-streaming chat completion tests ──────────────────────────────────────

#[tokio::test]
async fn test_chat_completion_successful_response() {
    let (base_url, _handle) = spawn_mock_server().await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let response = provider.chat_completion(test_request("test-model")).await;
    assert!(response.is_ok(), "Expected Ok, got {:?}", response);

    let response = response.unwrap();
    assert_eq!(response.id, "chatcmpl-test123");
    assert_eq!(response.model, "test-model");
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.role, MessageRole::Assistant);
    assert_eq!(
        response.choices[0].message.content,
        "Hello from mock server!"
    );
    assert_eq!(response.choices[0].finish_reason.as_ref().unwrap(), "stop");
}

#[tokio::test]
async fn test_chat_completion_sends_authorization_header() {
    let (base_url, auth_capture, _handle) = spawn_auth_recording_server().await;
    let provider = OpenAiProvider::new(
        base_url,
        Some("sk-secret-key".to_string()),
        "test-model".to_string(),
    );

    let result = provider.chat_completion(test_request("test-model")).await;
    assert!(result.is_ok(), "Request with API key should succeed");

    // Verify the Authorization header was actually received by the server.
    let auth = auth_capture.lock().expect("lock").clone();
    assert!(
        auth.as_ref().is_some_and(|v| v.starts_with("Bearer ")),
        "Expected Bearer auth header, got {:?}",
        auth
    );
}

#[tokio::test]
async fn test_chat_completion_no_authorization_without_api_key() {
    let (base_url, auth_capture, _handle) = spawn_auth_recording_server().await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    assert!(result.is_ok());

    // Verify no Authorization header was sent.
    let auth = auth_capture.lock().expect("lock").clone();
    assert!(
        auth.is_none(),
        "Expected no auth header when api_key is None, got {:?}",
        auth
    );
}

#[tokio::test]
async fn test_chat_completion_connection_failed() {
    // Use a port that nothing is listening on.
    let provider = OpenAiProvider::new(
        "http://127.0.0.1:1".to_string(),
        None,
        "test-model".to_string(),
    );

    let result = provider.chat_completion(test_request("test-model")).await;
    assert!(matches!(result, Err(ProviderError::ConnectionFailed(_))));
}

// ── Streaming stub test ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_completion_stream_returns_streaming_not_supported() {
    let provider = OpenAiProvider::new(
        "http://localhost:11434".to_string(),
        None,
        "llama3".to_string(),
    );

    let result = provider
        .chat_completion_stream(test_request("llama3"))
        .await;
    assert!(
        matches!(result, Err(ProviderError::StreamingNotSupported)),
        "Expected StreamingNotSupported, got {:?}",
        result
    );
}

// ── Error mapping tests ──────────────────────────────────────────────────────

async fn spawn_error_server(status_code: u16) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let app = Router::new().route(
        "/v1/chat/completions",
        post(
            move |_state: State<()>, _body: axum::Json<serde_json::Value>| async move {
                (
                    StatusCode::from_u16(status_code).unwrap(),
                    axum::Json(json!({ "error": "test error" })),
                )
            },
        ),
    );

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, handle)
}

#[tokio::test]
async fn test_chat_completion_4xx_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(400).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 400);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_chat_completion_401_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(401).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 401);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_chat_completion_404_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(404).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 404);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_chat_completion_500_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(500).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 500);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_chat_completion_429_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(429).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider.chat_completion(test_request("test-model")).await;
    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 429);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}
