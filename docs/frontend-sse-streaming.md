# Frontend SSE Streaming

## Overview

Issue `#12` replaces the non-streaming chat integration in the Leptos frontend
with real-time SSE streaming. Tokens arrive from the LLM provider as they are
generated and are appended to the assistant message bubble character-by-character.

The implementation introduces three new frontend modules:

- `frontend/src/sse.rs` — custom SSE parser that buffers raw HTTP chunks and
  emits complete `SseEvent` values.
- `frontend/src/api.rs` — `stream_chat_request` async function that sends
  `POST /api/chat/completions/stream`, reads the response body via
  `web_sys::ReadableStream`, and calls an `on_chunk` callback for each
  `ApiChatCompletionChunk`.
- `frontend/src/components/chat.rs` — updated `ChatView` that creates an empty
  assistant placeholder, streams tokens into it via `stream_chat_request`, and
  cleans up on stream end or error.

## Architecture And Design

### Data Flow

```text
User sends message
  → ChatView appends User message to thread
  → ChatView creates empty Assistant placeholder (id = assistant_id)
  → ChatView sets loading = true
  → spawn_local {
        stream_chat_request(
            messages,
            on_chunk = |chunk| {
                if let Some(text) = chunk.choices[0].delta.content {
                    append text to assistant placeholder
                }
            }
        )
        → on success: loading = false
        → on error:   replace placeholder content with error text,
                       is_error = true, loading = false
    }
```

### SSE Parser (`SseParser`)

`SseParser` is a small, zero-allocation-overhead state machine that processes
raw text chunks from the `ReadableStream`:

1. **Buffering**: Incoming text is appended to an internal `String` buffer.
2. **Line splitting**: The buffer is split on `\n` (and `\r\n`).
3. **Field accumulation**:
   - `data: <value>` lines accumulate into the current event's data payload.
   - Multiple `data:` lines are joined with `\n`.
   - `event: <value>` sets the event type (default `"message"`).
   - Lines starting with `:` are comments and ignored.
4. **Event emission**: A blank line signals the end of an event; the accumulated
   `SseEvent` is returned.
5. **Finalisation**: `finalize()` drains any trailing data that lacked a
   terminating blank line (e.g. when the stream closes).

The parser is target-agnostic — it works on `&str` and requires no browser APIs,
so it is fully unit-testable on the host target.

### Streaming API Client (`stream_chat_request`)

`stream_chat_request` uses `gloo-net` for the initial HTTP request and then
accesses the underlying `web_sys::Response` body via `Response::body()`:

1. Build `POST /api/chat/completions/stream` with `stream: true`.
2. Send via `gloo-net` and check HTTP status. Non-2xx responses are converted
   to `ApiError::Http` immediately.
3. Obtain the `ReadableStream` from the response body.
4. Create a `ReadableStreamDefaultReader` and read chunks in a loop using
   `wasm_bindgen_futures::JsFuture`.
5. Each chunk is a `Uint8Array` → converted to `Vec<u8>` → decoded as UTF-8
   and fed into `SseParser`.
6. For each emitted `SseEvent`:
   - `data: [DONE]` → stream ends successfully.
   - Any other `data:` → deserialise as `ApiChatCompletionChunk` and invoke
     `on_chunk`.
7. On stream close, `finalize()` handles any trailing event.

### `ChatView` Updates

The `on_send` closure was refactored:

- **Before**: Called `send_chat_request`, waited for the full response, then
  appended a complete Assistant message.
- **After**: Appends an empty Assistant placeholder first, then calls
  `stream_chat_request`. The `on_chunk` callback appends delta content directly
  to the placeholder message in the reactive signal. On stream end, `loading` is
  set to `false`. On error, the placeholder is converted to an error bubble.

### Design Decisions

- **Custom SSE parser instead of a library**: The only existing WASM SSE crate
  (`reqwasm`) pulls in many dependencies. A custom parser (~100 lines) avoids
  extra binary bloat, which is important for the Raspberry Pi target.
- **Direct `ReadableStream` access**: `gloo-net` does not provide a streaming
  text reader, but it exposes the raw `web_sys::ReadableStream`. Using
  `ReadableStreamDefaultReader` directly avoids adding another dependency.
- **Zero-copy where possible**: The SSE parser works on `&str` slices. UTF-8
  decoding uses `String::from_utf8_lossy` (returns `Cow<str>`), but since the
  parsed data is immediately fed to `SseParser::feed`, the allocation is
  unavoidable for the chunk buffer itself.
- **Keep `send_chat_request`**: The non-streaming function is preserved for
  backwards compatibility and potential fallback use (e.g. providers that do
  not support streaming).

## API Reference

### `frontend/src/sse.rs`

#### `SseEvent`

```rust
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
}
```

A single parsed SSE event. `event_type` defaults to `"message"` when no
`event:` field is present.

#### `SseParser`

```rust
pub struct SseParser;

impl SseParser {
    pub fn new() -> Self;
    pub fn feed(&mut self, chunk: &str) -> Vec<SseEvent>;
    pub fn finalize(self) -> Option<SseEvent>;
}
```

- `feed`: Append a raw chunk and return all complete events found so far.
- `finalize`: Consume the parser and return any trailing event that was not
  terminated by a blank line.

### `frontend/src/api.rs`

#### `ApiChatCompletionChunk`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChatCompletionChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<ApiChunkChoice>,
}
```

Mirrors the server-side `ChatCompletionChunk`.

#### `ApiChunkChoice`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChunkChoice {
    pub index: u32,
    pub delta: ApiChunkDelta,
    pub finish_reason: Option<String>,
}
```

#### `ApiChunkDelta`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ApiChunkDelta {
    pub role: Option<ApiMessageRole>,
    pub content: Option<String>,
}
```

#### `stream_chat_request`

```rust
pub async fn stream_chat_request(
    messages: &[ApiChatMessage],
    model: &str,
    endpoint: &str,
    auth_key: &str,
    on_chunk: impl FnMut(ApiChatCompletionChunk),
) -> Result<(), ApiError>
```

Sends a streaming chat completion request. For each received chunk, `on_chunk`
is called synchronously. The function returns `Ok(())` on successful stream
termination (`data: [DONE]`) or `Err` on network/HTTP/parse errors.

### `frontend/src/components/chat.rs`

#### `ChatView` (updated)

- Creates an empty `ChatMessage` placeholder with `role: Assistant` and
  `content: ""` before starting the stream.
- Uses `stream_chat_request` instead of `send_chat_request`.
- The `on_chunk` closure updates the placeholder's `content` signal in-place.
- On error, the placeholder is updated with `is_error: true` and the error text.
- If the stream ends with no content, the placeholder is updated to
  `"(empty response from model)"` with `is_error: true`.

## Configuration

### New dependencies

Added to `frontend/Cargo.toml`:

- `wasm-bindgen-futures = "0.4"` — required to await JS Promises from
  `ReadableStreamDefaultReader::read`.

### New `web-sys` features

Added to `frontend/Cargo.toml`:

- `ReadableStream` — exposes `Response::body()` as `ReadableStream`.
- `ReadableStreamDefaultReader` — allows creating a reader and calling `read()`.

## Testing Guide

Run the frontend unit tests:

```bash
cargo test -p frontend
```

Key tests:

- `sse::tests::*` — parser correctness (single event, multi-event, CRLF,
  comments, custom event type, chunk boundaries, finalisation).
- `api::tests::test_chat_completion_chunk_deserialisation` — verifies JSON
  mapping for streaming chunks.
- `api::tests::test_stream_request_payload_serialisation` — verifies the
  request body sets `stream: true`.

Run structural/integration tests:

```bash
cargo test -p server --test frontend_api_integration
```

Run linting and formatting:

```bash
cargo clippy --all-targets
cargo fmt --all -- --check
```

## Migration / Upgrade Notes

- `send_chat_request` is no longer called from `ChatView`, but the function is
  preserved in `api.rs` for backwards compatibility.
- `ChatView` now creates an assistant placeholder message immediately on send.
  Any code that inspects `state.threads` immediately after `on_send` may see
  an empty assistant message.
- The `loading` signal remains `true` for the entire duration of the stream and
  is set to `false` only on stream end or error.
