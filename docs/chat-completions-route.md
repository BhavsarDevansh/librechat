# Chat Completions Route

## Overview

Issue `#8` adds a non-streaming HTTP endpoint at `POST /api/chat/completions`.
The route accepts a JSON `ChatCompletionRequest`, forwards it to the configured
LLM provider, and returns the provider's `ChatCompletionResponse` as JSON.

The route lives in `server/src/routes/chat.rs` and is mounted by the shared
router builder in `server/src/lib.rs`.

## Architecture And Design

The request flow is:

1. Axum matches `POST /api/chat/completions`.
2. The `chat_completion` handler extracts:
   - `AppState` via `State<AppState>`
   - The request body via `Json<ChatCompletionRequest>`
3. The handler calls `state.provider.chat_completion(request).await`.
4. On success, the handler returns `(StatusCode::OK, Json(response))`.
5. On failure, the handler maps `ProviderError` into an HTTP status and a JSON
   body shaped as `{"error":"..."}`.

`AppState` now owns:

- `provider: Arc<dyn LlmProvider>`
- `static_dir: PathBuf`

Using `Arc<dyn LlmProvider>` keeps the state cloneable for Axum while allowing
tests to inject a mock provider without changing production routing.

## API Reference

### Endpoint

- Method: `POST`
- Path: `/api/chat/completions`
- Request content type: `application/json`
- Success response content type: `application/json`
- Error response content type: `application/json`

### Request Type

The handler currently accepts the shared provider request type directly:

```rust
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}
```

This matches the GitHub issue body, which required extracting a
`ChatCompletionRequest` from the JSON body and forwarding it to the provider.

### Response Type

Successful responses return `ChatCompletionResponse` unchanged from the
provider:

```rust
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}
```

Errors return:

```json
{"error":"..."}
```

## Error Mapping

Provider errors are translated as follows:

- `ProviderError::ApiError { status, message }`:
  - HTTP status: the upstream provider status when it is valid
  - body: `{"error":"<message>"}`
- `ProviderError::ConnectionFailed(_)`:
  - HTTP status: `502 Bad Gateway`
  - body: `{"error":"Connection failed: ..."}`
- `ProviderError::InvalidResponse(_)`:
  - HTTP status: `502 Bad Gateway`
  - body: `{"error":"Invalid response: ..."}`
- Any other provider error:
  - HTTP status: `500 Internal Server Error`
  - body: `{"error":"..."}`
- JSON extraction failure:
  - HTTP status: `400 Bad Request`
  - body: `{"error":"Failed to parse JSON request: ..."}`

If an upstream `ApiError` contains a non-standard status code that cannot be
converted into `StatusCode`, the handler falls back to `502 Bad Gateway`.

## Logging

The handler emits `tracing` records for:

- Request start with `model` and `message_count`
- Success with `model` and `choice_count`
- Failure with the mapped HTTP status and provider error

This complements the existing `TraceLayer` request/response logging on the
router.

## Testing Guide

Run the targeted route tests:

```bash
cargo test -p server --test chat_completions
```

Run the full server verification suite:

```bash
cargo test -p server
cargo clippy -p server --all-targets -- -D warnings
cargo fmt --all -- --check
```

The integration tests in `server/tests/chat_completions.rs` verify:

- `200 OK` with provider JSON passthrough
- JSON request parsing into `ChatCompletionRequest`
- `400 Bad Request` for malformed JSON
- Upstream `ApiError` status passthrough
- `502 Bad Gateway` for connection and invalid-response failures
- `500 Internal Server Error` for other provider failures

## Configuration

No new environment variables were added for this issue. The route uses the
provider already configured in `AppState`, which defaults to
`OpenAiProvider::from_env()`.

## Migration Notes

`AppState` now requires a provider in addition to the static directory. Existing
production code can continue using `AppState::new()` or
`AppState::with_static_dir(...)`, both of which create an `OpenAiProvider` from
environment variables automatically.
