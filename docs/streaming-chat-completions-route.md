# Streaming Chat Completions Route (SSE)

## Overview

Issue `#9` adds a streaming HTTP endpoint at `POST /api/chat/completions/stream`.
The route accepts a JSON `ChatCompletionRequest`, forwards it to the configured
LLM provider's streaming interface, and returns the response as a
Server-Sent Events (SSE) stream. Each `ChatCompletionChunk` from the provider
is serialised as a `data:` event; the stream terminates with `data: [DONE]`.
Mid-stream errors are reported as `event: error` SSE messages.

The route lives in `server/src/routes/chat_stream.rs` and is mounted by the
shared router builder in `server/src/lib.rs`. Shared error-mapping logic was
extracted into `server/src/routes/error.rs` to avoid duplication with the
non-streaming route.

## Architecture And Design

The request flow is:

1. Axum matches `POST /api/chat/completions/stream`.
2. The `chat_completion_stream` handler extracts:
   - `AppState` via `State<AppState>`
   - The request body via `Json<ChatCompletionRequest>`
3. The handler calls `state.provider.chat_completion_stream(request).await`,
   which returns an `mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>`.
4. The receiver is converted into an SSE stream using
   `futures_util::stream::unfold` with a state machine (`SseStreamState`):
   - **`Receiving`**: Reads chunks from the channel, yielding
     `data: {json}` events for `Ok` chunks.
   - **On `Err`**: Yields `event: error` + `data: {message}`, then transitions
     to `Done`.
   - **On channel close (`None`)**: Yields `data: [DONE]`, then transitions to
     `Done`.
   - **`Done`**: The stream terminates (returns `None` from `unfold`).
5. The stream is wrapped in `axum::response::sse::Sse` with
   `KeepAlive::default()`.

### State Machine Diagram

```text
          ┌──────────────────────────────────────────────────────┐
          │               SseStreamState::Receiving              │
          │  ┌─────────────────────────────────────────────────┐ │
          │  │  mpsc::Receiver<Result<Chunk, ProviderError>>    │ │
          │  └─────────────────────────────────────────────────┘ │
          │    │              │                │                   │
          │  Ok(chunk)     Err(error)       None (closed)         │
          │    │              │                │                   │
          │  yield data:{json}  yield event:error  yield data:[DONE]
          │    │              │                │                   │
          │  stay Receiving  └───────┬────────┘                   │
          │                         │                            │
          └─────────────────────────┼────────────────────────────┘
                                    ▼
                          SseStreamState::Done
                          (stream terminates)
```

### Design Decisions

- **`futures_util::stream::unfold` over `tokio-stream`**: Avoided adding
  `tokio-stream` as a dependency since `futures-util` is already in the
  workspace. The `unfold` combinator provides the same functionality with a
  state machine that naturally models the "error → stop" and "closed → [DONE]"
  transitions.
- **`BoxStream` return type**: The `build_sse_stream` function returns
  `BoxStream<'static, Result<Event, Infallible>>` to erase the complex
  `Unfold` type, making the handler signature cleaner.
- **`Infallible` as the stream error type**: Since all provider errors are
  converted into `Ok(Event)` (with `event: error`), the stream itself never
  produces an error. Using `Infallible` communicates this at the type level.
- **Shared `error` module**: `map_provider_error` and `ErrorResponse` were
  duplicated between `chat.rs` and `chat_stream.rs`. They were extracted into
  `server/src/routes/error.rs` to follow DRY principles.
- **`ProviderError` now implements `Clone`**: Required for the mock provider
  in integration tests to clone error values from shared state.

## API Reference

### Endpoint

- Method: `POST`
- Path: `/api/chat/completions/stream`
- Request content type: `application/json`
- Success response content type: `text/event-stream`
- Error response content type: `application/json` (pre-stream errors only)

### Request Type

Same as the non-streaming route — `ChatCompletionRequest`:

```rust
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}
```

### SSE Event Types

| Event type | Data format | When emitted |
|---|---|---|
| (default) | JSON `ChatCompletionChunk` | Each successful chunk from the provider |
| `error` | JSON envelope: `{"error":{"message":"..."}}` | Mid-stream provider error |
| (default) | `[DONE]` | Provider channel closed cleanly |

### SSE Event Format

```text
data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: [DONE]

```

Mid-stream error:

```text
data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

event: error
data: {"error":{"message":"Connection failed: upstream disconnected"}}

```

### Response Types

```rust
pub struct ChatCompletionChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

pub struct ChunkDelta {
    pub role: Option<MessageRole>,
    pub content: Option<String>,
}
```

## Error Mapping

Pre-stream errors (before SSE starts) return JSON error responses identical to
the non-streaming route:

- `ProviderError::ApiError { status, message }` → mapped upstream status
- `ProviderError::ConnectionFailed(_)` → `502 Bad Gateway`
- `ProviderError::InvalidResponse(_)` → `502 Bad Gateway`
- `ProviderError::StreamEnded` → `500 Internal Server Error`
- `ProviderError::StreamingNotSupported` → `500 Internal Server Error`
- JSON extraction failure → `400 Bad Request`

Mid-stream errors are reported as SSE `event: error` messages and the stream
closes immediately after.

## Logging

The handler emits `tracing` records for:

- Request start with `model` and `message_count`
- Pre-stream failure with mapped HTTP status and provider error

Mid-stream errors are logged at the route level in `build_sse_stream`
(`server/src/routes/chat_stream.rs`) using `warn!`, including the provider
error details. This makes mid-stream failures observable without relying on
provider-side logging.

## Testing Guide

Run the targeted streaming route tests:

```bash
cargo test -p server --test chat_completions_stream
```

Run the full server verification suite:

```bash
cargo test -p server
cargo clippy -p server --all-targets -- -D warnings
cargo fmt --all -- --check
```

The integration tests in `server/tests/chat_completions_stream.rs` verify:

- `200 OK` with `text/event-stream` content type
- Tokens streamed as `data: {json}` events
- Stream terminates with `data: [DONE]`
- Mid-stream errors reported as `event: error` SSE events
- `400 Bad Request` for malformed JSON
- `502 Bad Gateway` for connection failures
- Upstream `ApiError` status passthrough (e.g. 429 → 429)

## Configuration

No new environment variables were added for this issue. The route uses the
provider already configured in `AppState`, which defaults to
`OpenAiProvider::from_env()`.

## Migration Notes

- `ProviderError` now derives `Clone` (all fields were already `Clone`-able).
  This should be a non-breaking change for consumers.
- The `routes::error` module is `pub(crate)` — it is not part of the public
  API and does not affect downstream consumers.
- The non-streaming route (`chat.rs`) was refactored to use the shared
  `error` module. No functional changes were made to its behaviour.
