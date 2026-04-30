//! Integration tests for the LlmProvider trait and shared chat types (Issue #5).

use async_trait::async_trait;
use server::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Choice,
    ChunkChoice, ChunkDelta, LlmProvider, MessageRole, ModelInfo, ProviderError, Usage,
};
use tokio::sync::mpsc;

struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        Ok(ChatCompletionResponse {
            id: "test-id".to_string(),
            model: "test-model".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: "Hello from mock!".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 10,
                total_tokens: 20,
            },
        })
    }

    async fn chat_completion_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>, ProviderError> {
        let (tx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let chunk = ChatCompletionChunk {
                id: "stream-id".to_string(),
                model: "test-model".to_string(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: ChunkDelta {
                        role: None,
                        content: Some("Hello".to_string()),
                    },
                    finish_reason: None,
                }],
            };
            let _ = tx.send(Ok(chunk)).await;
        });

        Ok(rx)
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

#[tokio::test]
async fn test_mock_provider_chat_completion() {
    let provider = MockProvider;
    let request = ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hi".to_string(),
        }],
        temperature: None,
        max_tokens: None,
        stream: None,
    };

    let response = provider.chat_completion(request).await.unwrap();
    assert_eq!(response.id, "test-id");
    assert_eq!(response.choices[0].message.content, "Hello from mock!");
}

#[tokio::test]
async fn test_mock_provider_chat_completion_stream() {
    let provider = MockProvider;
    let request = ChatCompletionRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hi".to_string(),
        }],
        temperature: None,
        max_tokens: None,
        stream: Some(true),
    };

    let mut rx = provider.chat_completion_stream(request).await.unwrap();
    let chunk = rx.recv().await.unwrap().unwrap();
    assert_eq!(chunk.choices[0].delta.content.as_ref().unwrap(), "Hello");
}

#[tokio::test]
async fn test_mock_provider_list_models() {
    let provider = MockProvider;
    let models = provider.list_models().await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "test-model");
}

#[tokio::test]
async fn test_provider_error_display() {
    let err = ProviderError::ConnectionFailed("timeout".to_string());
    assert_eq!(format!("{err}"), "Connection failed: timeout");

    let err = ProviderError::ApiError {
        status: 404,
        message: "Not Found".to_string(),
    };
    assert_eq!(format!("{err}"), "API error (404): Not Found");

    let err = ProviderError::StreamEnded;
    assert_eq!(format!("{err}"), "Stream ended unexpectedly");

    let err = ProviderError::InvalidResponse("bad json".to_string());
    assert_eq!(format!("{err}"), "Invalid response: bad json");

    let err = ProviderError::StreamingNotSupported;
    assert_eq!(format!("{err}"), "Streaming not supported");
}
