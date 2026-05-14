//! Integration tests for the streaming chat completion SSE endpoint (Issue #9).

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use server::app;
use server::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChunkChoice,
    ChunkDelta, LlmProvider, MessageRole, ModelInfo, ProviderError,
};
use server::state::AppState;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Mock provider for streaming tests
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct StreamMockProvider {
    chunks: Arc<Mutex<Vec<Result<ChatCompletionChunk, ProviderError>>>>,
    captured_request: Arc<Mutex<Option<ChatCompletionRequest>>>,
    stream_error: Arc<Mutex<Option<ProviderError>>>,
}

impl StreamMockProvider {
    /// Create a mock that yields the given chunks in order, then closes.
    fn with_chunks(chunks: Vec<ChatCompletionChunk>) -> Self {
        let results: Vec<Result<ChatCompletionChunk, ProviderError>> =
            chunks.into_iter().map(Ok).collect();
        Self {
            chunks: Arc::new(Mutex::new(results)),
            captured_request: Arc::new(Mutex::new(None)),
            stream_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a mock that yields some chunks, then errors mid-stream.
    fn with_mid_stream_error(chunks: Vec<ChatCompletionChunk>, error: ProviderError) -> Self {
        let mut results: Vec<Result<ChatCompletionChunk, ProviderError>> =
            chunks.into_iter().map(Ok).collect();
        results.push(Err(error));
        Self {
            chunks: Arc::new(Mutex::new(results)),
            captured_request: Arc::new(Mutex::new(None)),
            stream_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a mock that immediately fails with a provider error
    /// (before starting the stream).
    fn immediate_error(error: ProviderError) -> Self {
        Self {
            chunks: Arc::new(Mutex::new(Vec::new())),
            captured_request: Arc::new(Mutex::new(None)),
            stream_error: Arc::new(Mutex::new(Some(error))),
        }
    }

    fn captured_request_handle(&self) -> Arc<Mutex<Option<ChatCompletionRequest>>> {
        Arc::clone(&self.captured_request)
    }
}

#[async_trait]
impl LlmProvider for StreamMockProvider {
    async fn chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        Err(ProviderError::StreamingNotSupported)
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>, ProviderError> {
        *self.captured_request.lock().expect("lock") = Some(request);

        if let Some(error) = self.stream_error.lock().expect("lock").clone() {
            return Err(error);
        }

        let (tx, rx) = mpsc::channel(32);
        let chunks = self.chunks.lock().expect("lock").clone();

        tokio::spawn(async move {
            for result in chunks {
                if tx.send(result).await.is_err() {
                    return;
                }
            }
            // Sender is dropped here, closing the channel.
        });

        Ok(rx)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![ModelInfo {
            id: "test-model".to_string(),
        }])
    }

    fn name(&self) -> &str {
        "StreamMockProvider"
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_chunk(id: &str, model: &str, content: &str) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: id.to_string(),
        model: model.to_string(),
        choices: vec![ChunkChoice {
            index: 0,
            delta: ChunkDelta {
                role: None,
                content: Some(content.to_string()),
            },
            finish_reason: None,
        }],
    }
}

fn test_stream_request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }],
        temperature: Some(0.2),
        max_tokens: Some(128),
        stream: Some(true),
    }
}

fn test_app(provider: Arc<dyn LlmProvider>) -> (axum::Router, TempDir) {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp_dir.path().join("index.html"),
        "<!doctype html><title>test</title>",
    )
    .expect("write index.html");

    let state = AppState {
        db_pool: None,
        provider,
        static_dir: temp_dir.path().to_path_buf(),
    };
    (app(state), temp_dir)
}

/// Collect the full response body as a String.
async fn response_body_string(response: axum::response::Response) -> String {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    String::from_utf8(body.to_vec()).expect("response body is valid UTF-8")
}

/// Parse SSE text into individual event data lines (ignoring comments and
/// blank lines). Returns only the `data:` content for each event.
fn parse_sse_data_events(raw: &str) -> Vec<String> {
    let mut events = Vec::new();
    let mut current_data = String::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("data: ") {
            if current_data.is_empty() {
                current_data = rest.to_string();
            } else {
                // Multi-line data — append
                current_data.push('\n');
                current_data.push_str(rest);
            }
        } else if line.strip_prefix("event: ").is_some() {
            // Event type lines are parsed by parse_sse_events; ignored here
        } else if line.is_empty() {
            // End of event
            if !current_data.is_empty() {
                events.push(current_data.clone());
                current_data.clear();
            }
        }
    }
    // Trailing event without double newline
    if !current_data.is_empty() {
        events.push(current_data);
    }
    events
}

/// Parse SSE text and return (event_type, data) pairs.
fn parse_sse_events(raw: &str) -> Vec<(Option<String>, String)> {
    let mut events = Vec::new();
    let mut current_event_type: Option<String> = None;
    let mut current_data = String::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("event: ") {
            current_event_type = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("data: ") {
            if current_data.is_empty() {
                current_data = rest.to_string();
            } else {
                current_data.push('\n');
                current_data.push_str(rest);
            }
        } else if line.is_empty() {
            if !current_data.is_empty() {
                events.push((current_event_type.take(), current_data.clone()));
                current_data.clear();
            } else {
                current_event_type = None;
            }
        }
    }
    if !current_data.is_empty() {
        events.push((current_event_type, current_data));
    }
    events
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_stream_endpoint_returns_event_stream_content_type() {
    let provider = Arc::new(StreamMockProvider::with_chunks(vec![test_chunk(
        "chatcmpl-1",
        "test-model",
        "Hello",
    )]));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header present")
        .to_str()
        .expect("content-type is valid UTF-8");
    assert!(
        content_type.starts_with("text/event-stream"),
        "expected text/event-stream, got {content_type}"
    );
}

#[tokio::test]
async fn test_stream_endpoint_streams_tokens_as_data_events() {
    let chunk1 = test_chunk("chatcmpl-1", "test-model", "Hello");
    let chunk2 = test_chunk("chatcmpl-1", "test-model", " world");
    let provider = Arc::new(StreamMockProvider::with_chunks(vec![chunk1, chunk2]));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_string(response).await;
    let events = parse_sse_data_events(&body);

    // Should have 2 token events + 1 [DONE] event = 3 events
    assert_eq!(events.len(), 3, "expected 3 events, got: {events:?}");

    // First event should be a valid ChatCompletionChunk JSON
    let first: serde_json::Value =
        serde_json::from_str(&events[0]).expect("first event should be valid JSON");
    assert_eq!(first["id"], "chatcmpl-1");
    assert_eq!(first["choices"][0]["delta"]["content"], "Hello");

    // Second event should also be a valid chunk
    let second: serde_json::Value =
        serde_json::from_str(&events[1]).expect("second event should be valid JSON");
    assert_eq!(second["choices"][0]["delta"]["content"], " world");

    // Third event should be [DONE]
    assert_eq!(events[2], "[DONE]");
}

#[tokio::test]
async fn test_stream_endpoint_ends_with_done_message() {
    let provider = Arc::new(StreamMockProvider::with_chunks(vec![test_chunk(
        "chatcmpl-1",
        "test-model",
        "Hi",
    )]));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_string(response).await;
    let events = parse_sse_data_events(&body);

    let last = events.last().expect("should have at least one event");
    assert_eq!(last, "[DONE]", "stream should end with [DONE]");
}

#[tokio::test]
async fn test_stream_endpoint_reports_errors_via_sse_event() {
    let chunks = vec![test_chunk("chatcmpl-1", "test-model", "Hello")];
    let provider = Arc::new(StreamMockProvider::with_mid_stream_error(
        chunks,
        ProviderError::ConnectionFailed("upstream disconnected".to_string()),
    ));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_body_string(response).await;
    let events = parse_sse_events(&body);

    // Should have: 1 chunk event + 1 error event (no [DONE] after error)
    assert!(
        events.len() >= 2,
        "expected at least 2 events, got: {events:?}"
    );

    // There should be an error event
    let error_events: Vec<_> = events
        .iter()
        .filter(|(event_type, _)| event_type.as_deref() == Some("error"))
        .collect();
    assert!(
        !error_events.is_empty(),
        "expected at least one error event, got: {events:?}"
    );

    // The error event data should be a JSON envelope with the expected shape
    let error_data = &error_events[0].1;
    let error_json: serde_json::Value =
        serde_json::from_str(error_data).expect("error data should be valid JSON");
    assert!(
        error_json.get("error").is_some(),
        "error envelope should have top-level 'error' field, got: {error_json}"
    );
    assert!(
        error_json["error"].get("message").is_some(),
        "error envelope should have 'error.message' field, got: {error_json}"
    );
    assert!(
        error_json["error"]["message"].is_string(),
        "error.message should be a string, got: {error_json}"
    );
}

#[tokio::test]
async fn test_stream_endpoint_returns_bad_request_for_invalid_json() {
    let provider = Arc::new(StreamMockProvider::with_chunks(vec![]));
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(b"this is not json".to_vec()))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_stream_endpoint_returns_502_on_connection_failed() {
    let provider = Arc::new(StreamMockProvider::immediate_error(
        ProviderError::ConnectionFailed("cannot connect".to_string()),
    ));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn test_stream_endpoint_returns_429_on_api_error() {
    let provider = Arc::new(StreamMockProvider::immediate_error(
        ProviderError::ApiError {
            status: 429,
            message: "rate limited".to_string(),
        },
    ));
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_stream_endpoint_forwards_request_to_provider() {
    let provider = Arc::new(StreamMockProvider::with_chunks(vec![test_chunk(
        "chatcmpl-1",
        "test-model",
        "Hello",
    )]));
    let captured = provider.captured_request_handle();
    let request_body = serde_json::to_vec(&test_stream_request()).expect("serialize request");
    let (app_router, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions/stream")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app_router.oneshot(request).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);

    let captured_req = captured
        .lock()
        .expect("lock")
        .clone()
        .expect("request was captured");
    assert_eq!(captured_req.model, "test-model");
    assert_eq!(captured_req.messages.len(), 1);
    assert_eq!(captured_req.messages[0].role, MessageRole::User);
    assert_eq!(captured_req.messages[0].content, "Hello");
    assert_eq!(captured_req.stream, Some(true));
}
