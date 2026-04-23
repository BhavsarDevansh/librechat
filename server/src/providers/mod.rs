//! LLM provider abstraction layer.
//!
//! Defines the [`LlmProvider`] trait that all provider backends must implement,
//! and re-exports the shared request/response types from [`types`].

mod types;

pub use types::*;

use async_trait::async_trait;

/// A trait that all LLM provider backends must implement.
///
/// Provides both non-streaming and streaming chat completion interfaces.
/// Streaming uses a [`tokio::sync::mpsc`] channel receiver rather than
/// returning a `Stream` directly, which is simpler to integrate with Axum SSE.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a non-streaming chat completion request.
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError>;

    /// Send a streaming chat completion request. Returns a channel receiver
    /// that yields [`ChatCompletionChunk`] values as they arrive from the
    /// provider.
    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>,
        ProviderError,
    >;

    /// Human-readable name for this provider (e.g., "Ollama", "OpenAI").
    fn name(&self) -> &str;
}
