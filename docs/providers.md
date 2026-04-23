# LLM Provider Abstraction Layer

## Architecture & Design

The `providers` module (`server/src/providers/`) defines the core abstraction
layer for LLM backends. All provider implementations share a single trait
(`LlmProvider`) and a common set of request/response types, making it
straightforward to swap or add backends without changing downstream code.

### Data Flow

```
Client Request
     │
     ▼
Axum Route Handler
     │
     ▼
LlmProvider::chat_completion / chat_completion_stream
     │
     ▼
Provider Implementation (Ollama, OpenAI, …)
     │
     ▼
ChatCompletionResponse | mpsc::Receiver<ChatCompletionChunk>
```

### Design Decisions

- **OpenAI-compatible types**: `ChatCompletionRequest`, `ChatCompletionResponse`,
  and `ChatCompletionChunk` mirror the OpenAI Chat Completions API format
  because both Ollama and OpenAI use it. This avoids format-mapping overhead
  at the trait boundary.
- **`mpsc` channel for streaming**: Streaming uses
  `tokio::sync::mpsc::Receiver` rather than returning a `Stream` directly.
  This simplifies integration with Axum's SSE handler and avoids requiring
  consumers to implement `Stream` traits.
- **`async_trait`**: The `LlmProvider` trait uses `async_trait` to support
  async methods in the trait object, consistent with the project's Axum-based
  async architecture.

## API Reference

### Trait: `LlmProvider`

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError>;

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>,
        ProviderError,
    >;

    fn name(&self) -> &str;
}
```

| Method | Description |
|--------|-------------|
| `chat_completion` | Non-streaming request. Returns a full response. |
| `chat_completion_stream` | Streaming request. Returns a channel receiver that yields chunks. |
| `name` | Human-readable provider name (e.g. `"Ollama"`, `"OpenAI"`). |

### Shared Types

| Type | Module | Description |
|------|--------|-------------|
| `ChatMessage` | `providers::types` | A single message with `role` and `content`. |
| `MessageRole` | `providers::types` | Enum: `System`, `User`, `Assistant`. Serialized as lowercase. |
| `ChatCompletionRequest` | `providers::types` | Request body: `model`, `messages`, optional `temperature`/`max_tokens`/`stream`. |
| `ChatCompletionResponse` | `providers::types` | Non-streaming response: `id`, `model`, `choices`, `usage`. |
| `Choice` | `providers::types` | A single completion choice with `index`, `message`, `finish_reason`. |
| `Usage` | `providers::types` | Token usage: `prompt_tokens`, `completion_tokens`, `total_tokens`. |
| `ChatCompletionChunk` | `providers::types` | A streaming chunk: `id`, `model`, `choices`. |
| `ChunkChoice` | `providers::types` | A choice within a streaming chunk. |
| `ChunkDelta` | `providers::types` | Delta content: optional `role` and `content`. |

### Error Type: `ProviderError`

```rust
pub enum ProviderError {
    ConnectionFailed(String),
    ApiError { status: u16, message: String },
    StreamEnded,
    InvalidResponse(String),
}
```

| Variant | `Display` output |
|---------|-----------------|
| `ConnectionFailed(msg)` | `Connection failed: {msg}` |
| `ApiError { status, message }` | `API error ({status}): {message}` |
| `StreamEnded` | `Stream ended unexpectedly` |
| `InvalidResponse(msg)` | `Invalid response: {msg}` |

Implements `std::fmt::Display` and `std::error::Error`.

## Configuration

No new environment variables or feature flags are introduced by this module.
The `async-trait` crate is added as a workspace dependency.

## Testing Guide

Run all provider tests:

```sh
cargo test -p server --test provider_trait_tests
```

| Test | Validates |
|------|-----------|
| `test_mock_provider_chat_completion` | Non-streaming trait method returns correct types. |
| `test_mock_provider_chat_completion_stream` | Streaming trait method returns chunks via channel. |
| `test_provider_error_display` | `Display` impl for all `ProviderError` variants. |

To add a new provider, implement `LlmProvider` for your struct and add
integration tests following the `MockProvider` pattern in
`server/tests/provider_trait_tests.rs`.

## Migration / Upgrade Notes

- This is a **new module** — no breaking changes to existing code.
- The `providers` module is publicly exported from `server::providers`.
- The `state` module is now also publicly exported from `server::state`.
