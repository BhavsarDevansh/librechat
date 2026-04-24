//! Integration tests for the OpenAI-compatible provider client (Issue #6).
//! Issue #7 tests cover SSE streaming behaviour.

use server::providers::OpenAiProvider;
use server::providers::{
    ChatCompletionRequest, ChatMessage, LlmProvider, MessageRole, ProviderError,
};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::net::TcpListener;

use futures_util::StreamExt;

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

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, _handle)
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

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, auth_capture, _handle)
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

/// Merged env-var test to avoid parallel environment-variable races.
#[test]
fn test_openai_provider_from_env_defaults_and_custom() {
    // Save original values so we can restore them after the test.
    let orig_base = std::env::var("LLM_BASE_URL").ok();
    let orig_key = std::env::var("LLM_API_KEY").ok();
    let orig_model = std::env::var("LLM_MODEL").ok();
    let orig_connect = std::env::var("LLM_CONNECT_TIMEOUT_SECS").ok();
    let orig_timeout = std::env::var("LLM_TIMEOUT_SECS").ok();

    // ── Defaults ──────────────────────────────────────────────────────────
    // Safety: env vars are not read from multiple threads concurrently in this test.
    unsafe {
        std::env::remove_var("LLM_BASE_URL");
        std::env::remove_var("LLM_API_KEY");
        std::env::remove_var("LLM_MODEL");
        std::env::remove_var("LLM_CONNECT_TIMEOUT_SECS");
        std::env::remove_var("LLM_TIMEOUT_SECS");
    }

    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.base_url(), "http://localhost:11434");
    assert_eq!(provider.api_key(), None);
    assert_eq!(provider.model(), "llama3");

    // ── Custom values ─────────────────────────────────────────────────────
    // Safety: env vars are not read from multiple threads concurrently in this test.
    unsafe {
        std::env::set_var("LLM_BASE_URL", "http://custom:1234");
        std::env::set_var("LLM_API_KEY", "sk-custom-key");
        std::env::set_var("LLM_MODEL", "gpt-4o");
        std::env::set_var("LLM_CONNECT_TIMEOUT_SECS", "5");
        std::env::set_var("LLM_TIMEOUT_SECS", "60");
    }

    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.base_url(), "http://custom:1234");
    assert_eq!(provider.api_key(), Some("sk-custom-key"));
    assert_eq!(provider.model(), "gpt-4o");

    // ── Empty API key treated as None ─────────────────────────────────────
    // Safety: env vars are not read from multiple threads concurrently in this test.
    unsafe {
        std::env::set_var("LLM_API_KEY", "");
    }
    let provider = OpenAiProvider::from_env();
    assert_eq!(provider.api_key(), None);

    // Restore originals.
    // Safety: env vars are not read from multiple threads concurrently in this test.
    unsafe {
        match orig_base {
            Some(v) => std::env::set_var("LLM_BASE_URL", v),
            None => std::env::remove_var("LLM_BASE_URL"),
        }
        match orig_key {
            Some(v) => std::env::set_var("LLM_API_KEY", v),
            None => std::env::remove_var("LLM_API_KEY"),
        }
        match orig_model {
            Some(v) => std::env::set_var("LLM_MODEL", v),
            None => std::env::remove_var("LLM_MODEL"),
        }
        match orig_connect {
            Some(v) => std::env::set_var("LLM_CONNECT_TIMEOUT_SECS", v),
            None => std::env::remove_var("LLM_CONNECT_TIMEOUT_SECS"),
        }
        match orig_timeout {
            Some(v) => std::env::set_var("LLM_TIMEOUT_SECS", v),
            None => std::env::remove_var("LLM_TIMEOUT_SECS"),
        }
    }
}

// ── Non-streaming chat completion tests ──────────────────────────────────────

#[tokio::test]
async fn test_chat_completion_successful_response() {
    let (base_url, _handle) = spawn_mock_server().await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let response = provider
        .chat_completion(test_request("test-model"))
        .await
        .expect("request should succeed");

    assert_eq!(response.id, "chatcmpl-test123");
    assert_eq!(response.model, "test-model");
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.role, MessageRole::Assistant);
    assert_eq!(
        response.choices[0].message.content,
        "Hello from mock server!"
    );
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("stop"));
    assert_eq!(response.usage.prompt_tokens, 10);
    assert_eq!(response.usage.completion_tokens, 5);
    assert_eq!(response.usage.total_tokens, 15);
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

// ── SSE Streaming tests (Issue #7) ──────────────────────────────────────────

/// Builds a well-formed SSE data line for a given delta content string.
fn sse_data_line(content: &str) -> String {
    let chunk = json!({
        "id": "chatcmpl-stream",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "delta": { "content": content },
            "finish_reason": null
        }]
    });
    format!("data: {chunk}\n\n")
}

/// Builds the terminal SSE line.
fn sse_done_line() -> String {
    "data: [DONE]\n\n".to_string()
}

/// Spins up a mock server that responds with SSE chunks.
async fn spawn_sse_server(chunks: Vec<String>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let body = chunks.join("");

    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |_body: axum::Json<serde_json::Value>| async move {
            (
                StatusCode::OK,
                [
                    ("content-type", "text/event-stream"),
                    ("cache-control", "no-cache"),
                ],
                body,
            )
        }),
    );

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, _handle)
}

/// Spins up a mock SSE server that records the Authorization header.
async fn spawn_sse_auth_recording_server(
    chunks: Vec<String>,
) -> (
    String,
    Arc<Mutex<Option<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let state = MockState::default();
    let auth_capture = state.auth_header.clone();
    let body = chunks.join("");

    let app = Router::new()
        .route(
            "/v1/chat/completions",
            post(
                move |State(state): State<MockState>,
                      headers: axum::http::HeaderMap,
                      _body: axum::Json<serde_json::Value>| async move {
                    if let Some(auth) = headers.get(header::AUTHORIZATION) {
                        let val = auth.to_str().unwrap_or("").to_string();
                        *state.auth_header.lock().expect("lock") = Some(val);
                    }
                    (
                        StatusCode::OK,
                        [
                            ("content-type", "text/event-stream"),
                            ("cache-control", "no-cache"),
                        ],
                        body,
                    )
                },
            ),
        )
        .with_state(state);

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, auth_capture, _handle)
}

/// Spins up a mock SSE server that records whether the request body had `stream: true`.
async fn spawn_sse_stream_flag_recording_server(
    chunks: Vec<String>,
) -> (String, Arc<Mutex<bool>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let stream_flag = Arc::new(Mutex::new(false));
    let stream_flag_clone = stream_flag.clone();
    let body = chunks.join("");

    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |req_body: axum::Json<serde_json::Value>| async move {
            let is_stream = req_body
                .get("stream")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            *stream_flag_clone.lock().expect("lock") = is_stream;
            (
                StatusCode::OK,
                [
                    ("content-type", "text/event-stream"),
                    ("cache-control", "no-cache"),
                ],
                body,
            )
        }),
    );

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, stream_flag, _handle)
}

#[tokio::test]
async fn test_stream_yields_chunks_and_closes_on_done() {
    let chunks = vec![
        sse_data_line("Hello"),
        sse_data_line(" world"),
        sse_done_line(),
    ];
    let (base_url, _handle) = spawn_sse_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    // First chunk: "Hello"
    let chunk1 = rx
        .recv()
        .await
        .expect("should receive chunk 1")
        .expect("chunk 1 should be Ok");
    assert_eq!(chunk1.id, "chatcmpl-stream");
    assert_eq!(chunk1.choices.len(), 1);
    assert_eq!(chunk1.choices[0].delta.content.as_ref().unwrap(), "Hello");

    // Second chunk: " world"
    let chunk2 = rx
        .recv()
        .await
        .expect("should receive chunk 2")
        .expect("chunk 2 should be Ok");
    assert_eq!(chunk2.choices[0].delta.content.as_ref().unwrap(), " world");

    // Channel closes gracefully after [DONE].
    assert!(
        rx.recv().await.is_none(),
        "channel should close after [DONE]"
    );
}

#[tokio::test]
async fn test_stream_sends_stream_true_in_request_body() {
    let chunks = vec![sse_data_line("hi"), sse_done_line()];
    let (base_url, stream_flag, _handle) = spawn_sse_stream_flag_recording_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    // Drain the channel.
    while rx.recv().await.is_some() {}

    let was_stream = *stream_flag.lock().expect("lock");
    assert!(was_stream, "request body should have stream: true");
}

#[tokio::test]
async fn test_stream_sends_authorization_header_when_key_set() {
    let chunks = vec![sse_data_line("hi"), sse_done_line()];
    let (base_url, auth_capture, _handle) = spawn_sse_auth_recording_server(chunks).await;
    let provider = OpenAiProvider::new(
        base_url,
        Some("sk-stream-key".to_string()),
        "test-model".to_string(),
    );

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    while rx.recv().await.is_some() {}

    let auth = auth_capture.lock().expect("lock").clone();
    assert!(
        auth.as_ref().is_some_and(|v| v.starts_with("Bearer ")),
        "Expected Bearer auth header, got {:?}",
        auth
    );
}

#[tokio::test]
async fn test_stream_no_authorization_without_api_key() {
    let chunks = vec![sse_data_line("hi"), sse_done_line()];
    let (base_url, auth_capture, _handle) = spawn_sse_auth_recording_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    while rx.recv().await.is_some() {}

    let auth = auth_capture.lock().expect("lock").clone();
    assert!(auth.is_none(), "Expected no auth header, got {:?}", auth);
}

#[tokio::test]
async fn test_stream_connection_failed() {
    let provider = OpenAiProvider::new(
        "http://127.0.0.1:1".to_string(),
        None,
        "test-model".to_string(),
    );

    let result = provider
        .chat_completion_stream(test_request("test-model"))
        .await;

    assert!(
        matches!(result, Err(ProviderError::ConnectionFailed(_))),
        "Expected ConnectionFailed for unreachable server, got {:?}",
        result
    );
}

#[tokio::test]
async fn test_stream_http_error_maps_to_api_error() {
    let (base_url, _handle) = spawn_error_server(500).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider
        .chat_completion_stream(test_request("test-model"))
        .await;

    match result {
        Err(ProviderError::ApiError { status, message }) => {
            assert_eq!(status, 500);
            assert!(!message.is_empty());
        }
        other => panic!("Expected ApiError, got {:?}", other),
    }
}

#[tokio::test]
async fn test_stream_malformed_json_sends_error_without_terminating() {
    // One valid chunk, one malformed, one more valid, then [DONE].
    let valid_chunk = json!({
        "id": "chatcmpl-malformed",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "delta": { "content": "good" },
            "finish_reason": null
        }]
    });
    let chunks = vec![
        format!("data: {valid_chunk}\n\n"),
        "data: {not valid json}\n\n".to_string(),
        sse_data_line("after"),
        sse_done_line(),
    ];
    let (base_url, _handle) = spawn_sse_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    // First chunk is valid.
    let item1 = rx.recv().await.expect("should receive chunk 1");
    assert!(item1.is_ok(), "first chunk should be Ok");
    assert_eq!(
        item1.unwrap().choices[0].delta.content.as_deref().unwrap(),
        "good"
    );

    // Second chunk is malformed -> Err(InvalidResponse).
    let item2 = rx.recv().await.expect("should receive error item");
    match item2 {
        Err(ProviderError::InvalidResponse(_)) => {}
        other => panic!(
            "Expected InvalidResponse for malformed JSON, got {:?}",
            other
        ),
    }

    // Third chunk is valid -- stream continues.
    let item3 = rx.recv().await.expect("should receive chunk 3");
    assert!(
        item3.is_ok(),
        "third chunk should be Ok after malformed one"
    );
    assert_eq!(
        item3.unwrap().choices[0].delta.content.as_deref().unwrap(),
        "after"
    );

    // Channel closes gracefully after [DONE].
    assert!(
        rx.recv().await.is_none(),
        "channel should close after [DONE]"
    );
}

#[tokio::test]
async fn test_stream_partial_sse_lines_reassembled() {
    // This test exercises line reassembly: the mock server sends the SSE
    // body as a streaming HTTP response using Body::from_stream, with two
    // separate data frames that split a data: line mid-JSON. This forces
    // the provider to buffer partial bytes and reassemble across recv
    // boundaries. We embed a multibyte UTF-8 character to ensure proper
    // handling when the split happens within a codepoint.
    let multibyte_content = "split\u{1F680}emoji";
    let chunk_json = json!({
        "id": "chatcmpl-partial",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "delta": { "content": multibyte_content },
            "finish_reason": null
        }]
    });
    let full_body = format!("data: {chunk_json}\n\ndata: [DONE]\n\n");
    let mid = full_body.len() / 2;
    let part1 = full_body.as_bytes()[..mid].to_vec();
    let part2 = full_body.as_bytes()[mid..].to_vec();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{port}");

    let p2 = part2.clone();
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |_body: axum::Json<serde_json::Value>| async move {
            let stream = futures_util::stream::once(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(p2))
            });
            // First frame is immediate, second frame is delayed.
            let combined = futures_util::stream::iter(vec![Ok::<_, std::convert::Infallible>(
                axum::body::Bytes::from(part1),
            )])
            .chain(stream);
            let body = Body::from_stream(combined);
            let mut response = Response::new(body);
            response.headers_mut().insert(
                "content-type",
                "text/event-stream".parse().expect("header value"),
            );
            response
                .headers_mut()
                .insert("cache-control", "no-cache".parse().expect("header value"));
            response
        }),
    );

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    let item = rx
        .recv()
        .await
        .expect("should receive chunk")
        .expect("chunk should be Ok");
    assert_eq!(
        item.choices[0].delta.content.as_deref().unwrap(),
        multibyte_content,
        "reassembled content should match original multibyte string"
    );

    // Channel closes after [DONE].
    assert!(
        rx.recv().await.is_none(),
        "channel should close after [DONE]"
    );
}

#[tokio::test]
async fn test_stream_connection_error_mid_stream_sends_err_then_closes() {
    // Server sends one valid chunk then closes the connection (no [DONE]).
    let chunks = vec![sse_data_line("before-drop")];
    let (base_url, _handle) = spawn_sse_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let mut rx = provider
        .chat_completion_stream(test_request("test-model"))
        .await
        .expect("stream should return Ok");

    // First chunk is valid.
    let item1 = rx.recv().await.expect("should receive chunk");
    assert!(item1.is_ok(), "first chunk should be Ok");

    // After the server closes without [DONE], chat_completion_stream must
    // emit ProviderError::StreamEnded before closing the channel.
    let mut stream_ended_seen = false;
    loop {
        match rx.recv().await {
            None => {
                assert!(
                    stream_ended_seen,
                    "channel closed without receiving StreamEnded error"
                );
                break;
            }
            Some(Err(e)) => {
                assert!(
                    matches!(e, ProviderError::StreamEnded),
                    "expected StreamEnded error, got {:?}",
                    e
                );
                stream_ended_seen = true;
                // Channel should close next.
                assert!(
                    rx.recv().await.is_none(),
                    "channel should close after StreamEnded"
                );
                break;
            }
            Some(Ok(_)) => {
                // Could receive more data before the connection fully drops.
                continue;
            }
        }
    }
}

// ── Streaming method returns Ok (no longer stub) ─────────────────────────────

#[tokio::test]
async fn test_chat_completion_stream_returns_ok_with_sse_server() {
    // Streaming is now implemented — verify the method returns Ok(Receiver)
    // when connected to a real (mock) SSE server.
    let chunks = vec![sse_data_line("hello"), sse_done_line()];
    let (base_url, _handle) = spawn_sse_server(chunks).await;
    let provider = OpenAiProvider::new(base_url, None, "test-model".to_string());

    let result = provider
        .chat_completion_stream(test_request("test-model"))
        .await;
    assert!(
        result.is_ok(),
        "Expected Ok(Receiver) from streaming, got {:?}",
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

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server error");
    });

    wait_for_ready(port).await;

    (base_url, _handle)
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
