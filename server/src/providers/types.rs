//! Shared types for LLM provider request/response handling.
//!
//! Types mirror the OpenAI Chat Completions API format since both Ollama and
//! OpenAI use it. See `ROADMAP.md` Phase 1 for design rationale.

use serde::{Deserialize, Serialize};

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Role of a participant in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Request sent to an LLM provider (mirrors OpenAI Chat Completions format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Non-streaming response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

/// A single completion choice in a non-streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage statistics for a completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single chunk in a streaming response (SSE format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

/// A single choice within a streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

/// Delta content within a streaming chunk choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Errors that can occur when interacting with an LLM provider.
#[derive(Debug)]
pub enum ProviderError {
    /// Failed to establish a connection to the provider.
    ConnectionFailed(String),
    /// The provider returned a non-success HTTP status.
    ApiError { status: u16, message: String },
    /// The streaming response ended unexpectedly.
    StreamEnded,
    /// Streaming is not supported by this provider implementation.
    StreamingNotSupported,
    /// The provider returned a response that could not be parsed.
    InvalidResponse(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            Self::ApiError { status, message } => write!(f, "API error ({status}): {message}"),
            Self::StreamEnded => write!(f, "Stream ended unexpectedly"),
            Self::StreamingNotSupported => write!(f, "Streaming not supported"),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {msg}"),
        }
    }
}

impl std::error::Error for ProviderError {}
