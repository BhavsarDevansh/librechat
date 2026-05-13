//! Integration tests for the non-streaming chat completion API route (Issue #8).

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use server::app;
use server::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Choice,
    LlmProvider, MessageRole, ModelInfo, ProviderError, Usage,
};
use server::state::AppState;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tower::ServiceExt;

#[derive(Clone)]
struct MockProvider {
    response: Arc<MockProviderResponse>,
    captured_request: Arc<Mutex<Option<ChatCompletionRequest>>>,
}

#[derive(Clone)]
enum MockProviderResponse {
    Success(ChatCompletionResponse),
    Failure { status: u16, message: String },
    ConnectionFailed(String),
    InvalidResponse(String),
    StreamEnded,
    StreamingNotSupported,
}

impl MockProvider {
    fn success(response: ChatCompletionResponse) -> Self {
        Self {
            response: Arc::new(MockProviderResponse::Success(response)),
            captured_request: Arc::new(Mutex::new(None)),
        }
    }

    fn failure(error: ProviderError) -> Self {
        let response = match error {
            ProviderError::ApiError { status, message } => {
                MockProviderResponse::Failure { status, message }
            }
            ProviderError::ConnectionFailed(message) => {
                MockProviderResponse::ConnectionFailed(message)
            }
            ProviderError::InvalidResponse(message) => {
                MockProviderResponse::InvalidResponse(message)
            }
            ProviderError::StreamEnded => MockProviderResponse::StreamEnded,
            ProviderError::StreamingNotSupported => MockProviderResponse::StreamingNotSupported,
        };

        Self {
            response: Arc::new(response),
            captured_request: Arc::new(Mutex::new(None)),
        }
    }

    fn captured_request(&self) -> Option<ChatCompletionRequest> {
        self.captured_request.lock().expect("lock").clone()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        *self.captured_request.lock().expect("lock") = Some(request);
        match self.response.as_ref() {
            MockProviderResponse::Success(response) => Ok(response.clone()),
            MockProviderResponse::Failure { status, message } => Err(ProviderError::ApiError {
                status: *status,
                message: message.clone(),
            }),
            MockProviderResponse::ConnectionFailed(message) => {
                Err(ProviderError::ConnectionFailed(message.clone()))
            }
            MockProviderResponse::InvalidResponse(message) => {
                Err(ProviderError::InvalidResponse(message.clone()))
            }
            MockProviderResponse::StreamEnded => Err(ProviderError::StreamEnded),
            MockProviderResponse::StreamingNotSupported => {
                Err(ProviderError::StreamingNotSupported)
            }
        }
    }

    async fn chat_completion_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>, ProviderError> {
        Err(ProviderError::StreamingNotSupported)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![ModelInfo {
            id: "test-model".to_string(),
        }])
    }

    fn name(&self) -> &str {
        "MockProvider"
    }
}

fn test_response() -> ChatCompletionResponse {
    ChatCompletionResponse {
        id: "chatcmpl-test".to_string(),
        model: "test-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: MessageRole::Assistant,
                content: "Hello from mock provider".to_string(),
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage {
            prompt_tokens: 4,
            completion_tokens: 5,
            total_tokens: 9,
        },
    }
}

fn test_request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello".to_string(),
        }],
        temperature: Some(0.2),
        max_tokens: Some(128),
        stream: Some(false),
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
        provider,
        static_dir: temp_dir.path().to_path_buf(),
    };
    (app(state), temp_dir)
}

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    serde_json::from_slice(&body).expect("parse json response")
}

#[tokio::test]
async fn test_chat_completions_returns_200_and_provider_response_json() {
    let provider = Arc::new(MockProvider::success(test_response()));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let expected_request = test_request();
    let (app, _temp_dir) = test_app(provider.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type");
    assert!(
        content_type
            .to_str()
            .expect("content-type string")
            .starts_with("application/json"),
        "content-type should be application/json, got {content_type:?}"
    );

    let body = response_body_json(response).await;
    assert_eq!(body["id"], "chatcmpl-test");
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Hello from mock provider"
    );
    let captured_request = provider
        .captured_request()
        .expect("expected request to be passed to provider");
    assert_eq!(captured_request.model, expected_request.model);
    assert_eq!(
        captured_request.messages.len(),
        expected_request.messages.len()
    );
    assert_eq!(
        captured_request.messages[0].content,
        expected_request.messages[0].content
    );
    assert_eq!(captured_request.temperature, expected_request.temperature);
    assert_eq!(captured_request.max_tokens, expected_request.max_tokens);
    assert_eq!(captured_request.stream, expected_request.stream);
}

#[tokio::test]
async fn test_chat_completions_returns_400_for_malformed_json() {
    let provider = Arc::new(MockProvider::success(test_response()));
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"model\":"))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_body_json(response).await;
    assert!(
        body["error"]
            .as_str()
            .expect("error string")
            .contains("Failed to parse JSON"),
        "expected parse error response, got {body}"
    );
}

#[tokio::test]
async fn test_chat_completions_maps_api_error_status_code() {
    let provider = Arc::new(MockProvider::failure(ProviderError::ApiError {
        status: 429,
        message: "rate limited".to_string(),
    }));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "rate limited");
}

#[tokio::test]
async fn test_chat_completions_maps_non_error_api_status_to_502() {
    let provider = Arc::new(MockProvider::failure(ProviderError::ApiError {
        status: 200,
        message: "unexpected upstream status".to_string(),
    }));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "unexpected upstream status");
}

#[tokio::test]
async fn test_chat_completions_maps_connection_failed_to_502() {
    let provider = Arc::new(MockProvider::failure(ProviderError::ConnectionFailed(
        "connection refused".to_string(),
    )));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "Connection failed: connection refused");
}

#[tokio::test]
async fn test_chat_completions_maps_invalid_response_to_502() {
    let provider = Arc::new(MockProvider::failure(ProviderError::InvalidResponse(
        "bad upstream payload".to_string(),
    )));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "Invalid response: bad upstream payload");
}

#[tokio::test]
async fn test_chat_completions_maps_stream_ended_to_500() {
    let provider = Arc::new(MockProvider::failure(ProviderError::StreamEnded));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "Stream ended unexpectedly");
}

#[tokio::test]
async fn test_chat_completions_maps_streaming_not_supported_to_500() {
    let provider = Arc::new(MockProvider::failure(ProviderError::StreamingNotSupported));
    let request_body = serde_json::to_vec(&test_request()).expect("serialize request");
    let (app, _temp_dir) = test_app(provider);

    let request = Request::builder()
        .method("POST")
        .uri("/api/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body))
        .expect("build request");

    let response = app.oneshot(request).await.expect("oneshot");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response_body_json(response).await;
    assert_eq!(body["error"], "Streaming not supported");
}
