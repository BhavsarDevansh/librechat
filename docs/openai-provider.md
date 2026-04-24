# OpenAI-Compatible Provider Client

## Architecture & Design

The `openai` module (`server/src/providers/openai.rs`) implements the
[`LlmProvider`] trait for any backend that exposes the OpenAI Chat Completions
API. This includes local Ollama instances, OpenAI's hosted API, and any other
compatible server.

### Data Flow — Non-Streaming Request

```text
Client → Axum Handler
           │
           ▼
  OpenAiProvider::chat_completion(request)
           │
           ├─ Serialize ChatCompletionRequest → JSON (stream: false)
           ├─ POST {base_url}/v1/chat/completions
           │   └─ Authorization: Bearer {api_key} (if set)
           │
           ▼
  Provider Server (Ollama / OpenAI / …)
           │
           ▼
  Parse JSON → ChatCompletionResponse
     ─ or ─
  Map error → ProviderError::{ConnectionFailed, ApiError, InvalidResponse}
```

### Data Flow — Streaming Request

```text
Client → Axum Handler
           │
           ▼
  OpenAiProvider::chat_completion_stream(request)
           │
           ├─ Serialize ChatCompletionRequest → JSON (stream: true)
           ├─ POST {base_url}/v1/chat/completions
           │   └─ Authorization: Bearer {api_key} (if set)
           │
           ▼
  Provider Server (Ollama / OpenAI / …)
           │
           ▼  HTTP response (200, Content-Type: text/event-stream)
           │
  reqwest::Response::bytes_stream()
           │
           ▼  For each byte chunk:
           │   ├─ Append to line buffer
           │   ├─ Find "\n\n" delimiters → complete SSE events
           │   ├─ Parse "data: <json>" lines → ChatCompletionChunk
           │   │   ├─ Ok(chunk) → send through mpsc channel
           │   │   └─ Err(e) → send Err(InvalidResponse), continue streaming
           │   ├─ "data: [DONE]" → close channel gracefully
           │   └─ Incomplete lines → buffer for next chunk
           │
           ▼
  mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>
```

### Design Decisions

- **Single `reqwest::Client`**: The struct holds one `reqwest::Client` instance
  that is reused across all requests. This enables HTTP connection pooling,
  which is especially important on resource-constrained hardware (Raspberry Pi)
  where establishing a new TCP+TLS connection per request is expensive.
- **Configurable timeouts**: The client is built with a short connect timeout
  (default 10 s) and a long overall request timeout (default 300 s) so that
  slow LLM generations are not prematurely terminated. Both are configurable
  via environment variables.
- **Environment-driven configuration**: `OpenAiProvider::from_env()` reads
  `LLM_BASE_URL`, `LLM_API_KEY`, and `LLM_MODEL` from the process environment.
  Defaults target an Ollama instance on `localhost:11434` with model `llama3`,
  making zero-config local development the common case.
- **Trailing-slash normalisation**: The constructor strips trailing `/` from
  `base_url` so the URL template `{base_url}/v1/chat/completions` always
  produces a single slash.
- **Empty API key → `None`**: An empty `LLM_API_KEY` environment variable or an
  empty string passed to `new()` is treated as `None`, preventing an empty
  `Bearer` header from being sent.
- **Capped error bodies**: HTTP error response bodies are read up to 4 096
  bytes and truncated with an ellipsis, preventing unbounded memory usage from
  a misbehaving server.
- **SSE line buffering**: The streaming implementation buffers partial SSE
  lines across TCP chunks, reassembling them when a `\n\n` delimiter is
  received. This ensures that token data split across network packets is
  correctly parsed rather than discarded.
- **Resilient malformed-JSON handling**: When an SSE `data:` line contains
  invalid JSON, the error is sent as `Err(InvalidResponse)` through the channel
  but the stream **continues** processing subsequent events. This matches the
  OpenAI API contract where a single bad frame should not kill an entire
  response.
- **Graceful stream termination**: `data: [DONE]` closes the mpsc channel
  without an error. If the connection drops without a `[DONE]` sentinel,
  `Err(StreamEnded)` is sent before closing, so callers can distinguish between
  a complete and an interrupted response.
- **Channel buffer size**: The mpsc channel uses a buffer of 32 items, which
  balances memory usage on Raspberry Pi against backpressure from slow
  consumers.
- **Error granularity**: HTTP error responses are mapped to
  `ProviderError::ApiError { status, message }` preserving the exact status
  code and (truncated) body. Connection failures map to `ConnectionFailed`, and
  malformed JSON maps to `InvalidResponse`.

## API Reference

### Struct: `OpenAiProvider`

```rust
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `client` | `reqwest::Client` | Reusable HTTP client with connection pooling. |
| `base_url` | `String` | API base URL, trailing `/` stripped (e.g. `http://localhost:11434`). |
| `api_key` | `Option<String>` | Bearer token; `None` for Ollama or if empty. |
| `model` | `String` | Default model name (e.g. `llama3`, `gpt-4o-mini`). |

### Constructors

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(base_url: String, api_key: Option<String>, model: String) → Self` | Create with explicit configuration. Strips trailing `/`, treats empty API key as `None`. |
| `from_env` | `() → Self` | Create from environment variables (see Configuration). |

### Accessors

| Method | Return type | Description |
|--------|------------|-------------|
| `base_url()` | `&str` | The configured base URL. |
| `api_key()` | `Option<&str>` | The API key, if set. |
| `model()` | `&str` | The default model name. |

### `LlmProvider` Implementation

| Method | Behaviour |
|--------|-----------|
| `chat_completion` | Serialises request as JSON with `stream: false`, sends `POST {base_url}/v1/chat/completions`, adds `Authorization: Bearer` header if `api_key` is `Some`. Maps errors per table below. |
| `chat_completion_stream` | Serialises request as JSON with `stream: true`, sends `POST {base_url}/v1/chat/completions`, adds `Authorization: Bearer` header if `api_key` is `Some`. Reads the response as a byte stream, parses SSE `data:` lines, sends `Ok(ChatCompletionChunk)` for each parsed chunk through a buffered mpsc channel (buffer size 32). Closes gracefully on `data: [DONE]`, sends `Err(InvalidResponse)` for malformed JSON without terminating, sends `Err(ConnectionFailed)` on byte-stream errors, and sends `Err(StreamEnded)` if the connection drops without `[DONE]`. |
| `name` | Returns `"OpenAI-compatible"`. |

### Streaming Error Mapping

| Condition | `ProviderError` variant |
|-----------|------------------------|
| Connection / timeout error on initial request | `ConnectionFailed(message)` |
| HTTP 4xx / 5xx on initial request | `ApiError { status, message }` (body capped at 4 KiB) |
| Byte-stream read error mid-stream | `ConnectionFailed(message)` — sent then channel closes |
| Invalid UTF-8 in byte chunk | `InvalidResponse(message)` — sent then channel closes |
| Malformed JSON in SSE `data:` line | `InvalidResponse(message)` — sent but stream **continues** |
| `data: [DONE]` received | Channel closes gracefully (no error sent) |
| Connection closes without `[DONE]` | `StreamEnded` — sent then channel closes |

### Helper Function: `process_sse_event`

```rust
async fn process_sse_event(
    event: &str,
    tx: &mpsc::Sender<Result<ChatCompletionChunk, ProviderError>>,
) -> Result<bool, ()>
```

Parses all `data:` lines within a single SSE event string and sends results
through the channel. Returns `Ok(true)` if `[DONE]` was encountered, `Ok(false)`
otherwise, or `Err(())` if the receiver was dropped.

This helper eliminates duplication between the main loop (processing
`\n\n`-delimited events) and the final buffer drain (processing remaining data
after the byte stream ends).

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_BASE_URL` | `http://localhost:11434` | Base URL of the OpenAI-compatible API. |
| `LLM_API_KEY` | *(unset)* | Bearer token for authentication. Empty string treated as unset. Leave unset for Ollama. |
| `LLM_MODEL` | `llama3` | Default model identifier sent in request bodies. |
| `LLM_CONNECT_TIMEOUT_SECS` | `10` | TCP connect timeout in seconds. |
| `LLM_TIMEOUT_SECS` | `300` | Overall request timeout in seconds (long for LLM generation). |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.12 | HTTP client with `json` and `stream` features for byte-stream support |
| `futures-util` | 0.3 | `StreamExt` trait for iterating over `bytes_stream()` |
| `tokio` | 1 | Async runtime, `mpsc` channel for streaming chunks |
| `async-trait` | 0.1 | Async trait support for `LlmProvider` |
| `serde` / `serde_json` | 1 | Serialisation of request/response types |

## Testing Guide

Run the OpenAI provider integration tests:

```sh
cargo test -p server --test openai_provider
```

### Non-Streaming Tests

| Test | Validates |
|------|-----------|
| `test_openai_provider_new` | Constructor sets fields correctly; `name()` returns `"OpenAI-compatible"`. |
| `test_openai_provider_new_trims_trailing_slash` | Trailing `/` is stripped from `base_url`. |
| `test_openai_provider_new_empty_api_key_becomes_none` | Empty API key string is treated as `None`. |
| `test_openai_provider_new_with_api_key` | Non-empty API key is stored and accessible. |
| `test_openai_provider_from_env_defaults_and_custom` | `from_env()` returns defaults when vars unset; reads custom values; empty `LLM_API_KEY` → `None`. |
| `test_chat_completion_successful_response` | Non-streaming request returns a well-formed `ChatCompletionResponse`. |
| `test_chat_completion_sends_authorization_header` | Bearer token is sent and recorded by the mock server. |
| `test_chat_completion_no_authorization_without_api_key` | No `Authorization` header when `api_key` is `None`. |
| `test_chat_completion_connection_failed` | Connection refused maps to `ConnectionFailed`. |
| `test_chat_completion_4xx_maps_to_api_error` | 400 → `ApiError { status: 400, … }`. |
| `test_chat_completion_401_maps_to_api_error` | 401 → `ApiError { status: 401, … }`. |
| `test_chat_completion_404_maps_to_api_error` | 404 → `ApiError { status: 404, … }`. |
| `test_chat_completion_500_maps_to_api_error` | 500 → `ApiError { status: 500, … }`. |
| `test_chat_completion_429_maps_to_api_error` | 429 → `ApiError { status: 429, … }`. |

### Streaming Tests

| Test | Validates |
|------|-----------|
| `test_stream_yields_chunks_and_closes_on_done` | Chunks are yielded via the channel and it closes gracefully after `data: [DONE]`. |
| `test_stream_sends_stream_true_in_request_body` | The request body contains `"stream": true`. |
| `test_stream_sends_authorization_header_when_key_set` | Bearer token is sent in the streaming request when `api_key` is set. |
| `test_stream_no_authorization_without_api_key` | No `Authorization` header when `api_key` is `None`. |
| `test_stream_connection_failed` | Connection refused on initial request → `ConnectionFailed`. |
| `test_stream_http_error_maps_to_api_error` | HTTP 500 on initial request → `ApiError { status: 500, … }`. |
| `test_stream_malformed_json_sends_error_without_terminating` | Invalid JSON in an SSE line sends `Err(InvalidResponse)` but the stream continues and subsequent valid chunks are still delivered. |
| `test_stream_partial_sse_lines_reassembled` | Multiple SSE events in a single response body are correctly parsed and yielded. |
| `test_stream_connection_error_mid_stream_sends_err_then_closes` | When the server drops mid-stream, the channel closes (possibly after an `Err`). |
| `test_chat_completion_stream_returns_ok_with_sse_server` | `chat_completion_stream` returns `Ok(Receiver)` (no longer `StreamingNotSupported`). |

Tests spin up ephemeral `axum` mock servers on random ports and use TCP
readiness probes instead of fixed sleeps, ensuring deterministic startup.

## Migration / Upgrade Notes

- `chat_completion_stream()` now **fully implements** SSE streaming via
  `tokio::sync::mpsc`. Code that previously matched on
  `Err(ProviderError::StreamingNotSupported)` should be updated to handle the
  new `Ok(Receiver)` return type.
- The `futures-util` crate has been added as a workspace dependency. It
  provides the `StreamExt` trait needed to iterate over `reqwest`'s
  `bytes_stream()`.
- A new `ProviderError::StreamEnded` variant exists (it was defined in the
  types module previously but not used). Code that matches exhaustively on
  `ProviderError` should include an arm for `StreamEnded`.
- The `StreamingNotSupported` variant still exists on the enum but is no
  longer returned by `OpenAiProvider`. Other provider implementations may still
  use it.
