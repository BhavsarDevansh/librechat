# Phase 1 GitHub Issues

> **Already created on GitHub:** #1, #2, #3, #4, #5
> **Below:** Issues #6–#12 (create manually via `gh issue create` or the GitHub UI)

---

## Issue #6: Implement OpenAI-compatible provider client (non-streaming)

**Labels:** `mcp`, `enhancement`

### Summary

Implement the concrete `OpenAiProvider` struct that satisfies the `LlmProvider` trait from issue #5. This issue covers the **non-streaming** chat completion path — sending a `POST /v1/chat/completions` request with `stream: false` and returning the full response.

### Requirements

- Create `server/src/providers/openai.rs` containing the `OpenAiProvider` struct.
- `OpenAiProvider` holds:
  - `client: reqwest::Client` — reuse a single client instance across requests (connection pooling).
  - `base_url: String` — e.g. `http://localhost:11434` for Ollama or `https://api.openai.com` for OpenAI.
  - `api_key: Option<String>` — `None` for Ollama, `Some("sk-...")` for OpenAI.
  - `model: String` — default model to use (e.g. `llama3`, `gpt-4o-mini`).
- Implement `LlmProvider::chat_completion()`:
  - Build the request body from `ChatCompletionRequest`, setting `stream: false`.
  - Send `POST {base_url}/v1/chat/completions` with `Authorization: Bearer {api_key}` header if `api_key` is `Some`.
  - Parse the JSON response into `ChatCompletionResponse`.
  - Map HTTP error statuses to `ProviderError::ApiError { status, message }`.
  - Map connection failures to `ProviderError::ConnectionFailed`.
  - Map deserialization failures to `ProviderError::InvalidResponse`.
- Implement `LlmProvider::name()` returning `"OpenAI-compatible"`.
- Implement `LlmProvider::chat_completion_stream()` as a stub that returns `Err(ProviderError::StreamEnded)` — streaming is a separate issue.
- Add a `OpenAiProvider::new()` constructor and `OpenAiProvider::from_env()` that reads:
  - `LLM_BASE_URL` (default: `http://localhost:11434`)
  - `LLM_API_KEY` (optional)
  - `LLM_MODEL` (default: `llama3`)
- Add `reqwest = { version = "0.12", features = ["json", "stream"] }` to `server/Cargo.toml` if not already present.

### Key Code References (Context7-verified)

Non-streaming request with reqwest:

```rust
let response = self.client
    .post(format!("{}/v1/chat/completions", self.base_url))
    .header("Authorization", format!("Bearer {}", api_key))
    .json(&request_body)
    .send()
    .await
    .map_err(|e| ProviderError::ConnectionFailed(e.to_string()))?;

```

### Acceptance Criteria

- [ ] `cargo build -p server` compiles with the new provider
- [ ] `OpenAiProvider::from_env()` reads environment variables correctly
- [ ] Non-streaming request to a running Ollama instance returns a valid `ChatCompletionResponse`
- [ ] HTTP errors (4xx, 5xx) are mapped to `ProviderError::ApiError`
- [ ] Connection refused is mapped to `ProviderError::ConnectionFailed`
- [ ] `chat_completion_stream()` returns an error (stub for now)

### Notes

- This provider works with both Ollama and OpenAI because both implement the `/v1/chat/completions` endpoint. The only difference is `base_url` and `api_key`.
- Future work: Add an Ollama-native provider (using `/api/chat`) for features not available in the OpenAI-compatible endpoint.

---

## Issue #7: Implement SSE streaming in the OpenAI-compatible provider client

**Labels:** `mcp`, `streaming`, `enhancement`

### Summary

Implement the `chat_completion_stream()` method on `OpenAiProvider`. This streams token-level responses from the LLM provider using Server-Sent Events (SSE), returning chunks via a `tokio::sync::mpsc` channel as they arrive.

### Requirements

- Update `OpenAiProvider::chat_completion_stream()` to:
  - Send `POST {base_url}/v1/chat/completions` with `"stream": true` in the request body.
  - Read the response body as a byte stream using `reqwest::Response::bytes_stream()`.
  - Parse the SSE format: each line starting with `data: ` contains a JSON `ChatCompletionChunk`. Lines starting with `data: [DONE]` signal end of stream.
  - Buffer partial lines (a single SSE event may be split across multiple TCP chunks).
  - Send each parsed `ChatCompletionChunk` through a `tokio::sync::mpsc::Sender` (buffer size 32).
  - Send `Ok(chunk)` for successful chunks, `Err(ProviderError)` for parse errors.
  - Close the channel gracefully when `[DONE]` is received.
  - Close the channel with an error if the stream fails mid-way.
- Add `futures-util` to dependencies (for `StreamExt` on the byte stream).
- Add `tokio-stream` to dependencies if needed for stream utilities.

### Key Code References (Context7-verified)

Streaming response with reqwest:

```rust
use futures_util::StreamExt;

let mut stream = response.bytes_stream();
while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|e| ProviderError::ConnectionFailed(e.to_string()))?;
    // accumulate into a line buffer, parse SSE lines
}

```

SSE format (from OpenAI / Ollama):

```

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Hello"}}]}

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":" world"}}]}

data: [DONE]

```

### Acceptance Criteria

- [ ] `chat_completion_stream()` returns a `mpsc::Receiver` that yields `Ok(ChatCompletionChunk)` items
- [ ] Partial SSE lines that span multiple TCP chunks are correctly reassembled
- [ ] `data: [DONE]` closes the channel gracefully
- [ ] Connection errors mid-stream send `Err(ProviderError)` then close the channel
- [ ] Malformed JSON in an SSE line sends `Err(ProviderError::InvalidResponse)` but does NOT terminate the stream
- [ ] `cargo build -p server` compiles without warnings

### Notes

- The SSE parser must handle the `\n\n` delimiter between events and the `data: ` prefix on each line.
- Empty lines between events should be ignored.
- The `mpsc` channel buffer size of 32 is a starting point; it can be tuned based on performance testing.

---

## Issue #8: Add non-streaming chat completion API route

**Labels:** `server`, `enhancement`

### Summary

Create an Axum route `POST /api/chat/completions` that accepts chat messages from the frontend, forwards them to the configured `LlmProvider`, and returns the complete response as JSON.

### Requirements

- Create `server/src/routes/chat.rs` with the handler.
- Define the request body type (can reuse `ChatCompletionRequest` from `providers::types`):
  ```rust
  // The frontend sends a simplified request; the server fills in defaults.
  #[derive(Deserialize)]
  pub struct ChatRequest {
      pub model: Option<String>,  // overrides default if provided
      pub messages: Vec<ChatMessage>,
      pub temperature: Option<f64>,
  }
  ```

- The handler should:
  1. Extract `AppState` via `axum::extract::State`.
  2. Read the `ChatRequest` from the request body.
  3. Build a `ChatCompletionRequest` using the provider's default model (or the override from the request).
  4. Call `provider.chat_completion(request)`.
  5. Return the `ChatCompletionResponse` as JSON with `200 OK`.
  6. On error, return a structured JSON error: `{"error": "message"}` with appropriate HTTP status codes (502 for upstream errors, 400 for bad requests).
- Add the route to the router: `.route("/api/chat/completions", post(chat_completion_handler))`.
- Store the `OpenAiProvider` (as `Box<dyn LlmProvider>`) in `AppState`.

### Key Code References (Context7-verified)

```rust
use axum::{extract::State, Json};
use crate::state::AppState;

async fn chat_completion_handler(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatCompletionResponse>, StatusCode> {
    let provider = &state.provider;
    let request = ChatCompletionRequest {
        model: req.model.unwrap_or_else(|| provider.default_model().to_string()),
        messages: req.messages,
        temperature: req.temperature,
        max_tokens: None,
        stream: Some(false),
    };
    let response = provider.chat_completion(request).await
        .map_err(|e| match e {
            ProviderError::ConnectionFailed(_) => StatusCode::BAD_GATEWAY,
            ProviderError::ApiError { status, .. } => StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Json(response))
}

```

### Acceptance Criteria

- [ ] `POST /api/chat/completions` with valid messages returns `200 OK` with a `ChatCompletionResponse`
- [ ] Missing or malformed request body returns `400 Bad Request`
- [ ] Upstream provider errors return `502 Bad Gateway`
- [ ] The `model` field in the request body correctly overrides the default model
- [ ] `AppState` holds the provider as `Box<dyn LlmProvider>`

### Notes

- This issue covers the non-streaming route only. The streaming route is a separate issue.
- CORS must already be configured (issue #3) so the frontend can call this endpoint.

---

## Issue #9: Add streaming chat completion API route (SSE endpoint)

**Labels:** `server`, `streaming`, `enhancement`

### Summary

Create an Axum route `POST /api/chat/completions/stream` that accepts chat messages and returns an SSE stream of `ChatCompletionChunk` objects, enabling real-time token delivery to the frontend.

### Requirements

- Create `server/src/routes/chat_stream.rs` with the handler.
- Use Axum's SSE support (`axum::response::sse::Sse`) to stream `ChatCompletionChunk` objects.
- The handler should:
  1. Extract `AppState` via `State`.
  2. Read the `ChatRequest` from the request body.
  3. Call `provider.chat_completion_stream(request)` to get an `mpsc::Receiver`.
  4. Convert the receiver into an Axum SSE stream by mapping each `Result<ChatCompletionChunk, ProviderError>` to an SSE event:
     - `Ok(chunk)` → `Event::default().data(serde_json::to_string(&chunk).unwrap())`
     - `Err(e)` → `Event::default().event("error").data(e.to_string())`
  5. Return `Sse<impl Stream>` with `keep_alive(Interval::from_secs(15))` to prevent connection timeouts.
- Add the route: `.route("/api/chat/completions/stream", post(chat_stream_handler))`.
- Ensure the SSE response has headers: `Content-Type: text/event-stream`, `Cache-Control: no-cache`, `Connection: keep-alive`. (Axum's `Sse` sets these automatically.)

### Key Code References (Context7-verified)

```rust
use axum::response::sse::{Event, Sse};
use futures_util::stream::Stream;

async fn chat_stream_handler(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let receiver = state.provider.chat_completion_stream(request).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let stream = tokio_stream::wrappers::ReceiverStream::new(receiver)
        .map(|result| match result {
            Ok(chunk) => Event::default().data(serde_json::to_string(&chunk).unwrap()),
            Err(e) => Event::default().event("error").data(e.to_string()),
        })
        .map(Ok);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

```

### Acceptance Criteria

- [ ] `POST /api/chat/completions/stream` returns `200 OK` with `Content-Type: text/event-stream`
- [ ] Each SSE event is a JSON-encoded `ChatCompletionChunk`
- [ ] The stream ends gracefully when the provider sends `[DONE]`
- [ ] A keep-alive ping is sent every 15 seconds during idle periods
- [ ] Error events are sent with `event: error` type
- [ ] Connection errors return `502 Bad Gateway`

### Notes

- The non-streaming `POST /api/chat/completions` route must already exist (issue #8).
- The frontend SSE integration is a separate issue.

---

## Issue #10: Build Leptos chat UI — message list and input components

**Labels:** `ui`, `enhancement`

### Summary

Build the visual components for the chat interface using Leptos CSR and traditional CSS: a scrollable message list area and a text input with a send button. This issue covers the UI only — API integration is a separate issue.

### Requirements

- Create the following Leptos components in `frontend/src/`:
  - `app.rs` — Root `App` component that composes `ChatView` and holds the application state.
  - `chat_view.rs` — Main chat container with flex column layout.
  - `message_list.rs` — Scrollable container that renders a list of `Message` components. Auto-scrolls to the bottom on new messages.
  - `message.rs` — Single message bubble. Shows role icon (user vs assistant) and content text. User messages right-aligned with accent background, assistant messages left-aligned with secondary background.
  - `chat_input.rs` — Fixed-bottom input bar with a `<textarea>` (not `<input>`, for multiline support) and a "Send" button. Submit on Enter (without Shift), newline on Shift+Enter.
- Define the `Message` signal type:
  ```rust
  #[derive(Clone, Debug)]
  pub struct Message {
      pub role: MessageRole,
      pub content: String,
  }
  ```

- The `App` component should hold `messages: RwSignal<Vec<Message>>` and `input_text: RwSignal<String>`.
- Clicking "Send" (or pressing Enter) should:
  1. Append the input text as a `Message { role: MessageRole::User, content: input_text }` to the messages signal.
  2. Clear the input text.
  3. Append a placeholder `Message { role: MessageRole::Assistant, content: String::new() }` (this will be filled by streaming later).
- For now, the assistant placeholder message should display a "Thinking..." animation (CSS-only pulse/spinner).
- Create `frontend/style/chat.css` with styles for:
  - `.chat-view`: flex column, full viewport height
  - `.message-list`: flex-grow, overflow-y auto, padding
  - `.message`: message bubble with role-dependent alignment and colors using CSS custom properties from `main.css`
  - `.chat-input`: sticky bottom, flex row, textarea + button
  - `.message--user` and `.message--assistant`: distinct backgrounds using `--color-bg-secondary` and `--color-accent`
  - `@keyframes pulse` for the "Thinking..." animation
- Import `chat.css` in `main.css` via `@import "chat.css";`.
- The textarea should auto-resize based on content (up to a max height).

### Acceptance Criteria

- [ ] `trunk serve` displays a full-viewport chat layout with dark theme
- [ ] Typing a message and clicking Send (or pressing Enter) appends a user message bubble
- [ ] User messages appear right-aligned, assistant placeholders left-aligned
- [ ] "Thinking..." animation shows on the assistant placeholder
- [ ] Textarea supports multiline input (Shift+Enter for newlines)
- [ ] Message list auto-scrolls to the bottom on new messages
- [ ] All styles use CSS custom properties from `main.css` — no Tailwind

### Notes

- This issue is UI-only. The assistant response will always show "Thinking..." until API integration is added in issues #11 and #12.
- Do NOT use Tailwind CSS. All styling is in traditional CSS stylesheets.
- The auto-resize textarea can use a simple `input` event handler that sets `el.style.height = "auto"; el.style.height = el.scrollHeight + "px"`.

---

## Issue #11: Connect Leptos frontend to non-streaming chat API

**Labels:** `ui`, `mcp`, `enhancement`

### Summary

Wire the Leptos chat UI to the non-streaming `POST /api/chat/completions` endpoint. When the user sends a message, the frontend sends the full conversation history to the backend and displays the assistant's response.

### Requirements

- Add `gloo-net` (or `reqwasm`) to `frontend/Cargo.toml` as a dependency for making HTTP requests from WASM.
- Add `serde` and `serde_json` to `frontend/Cargo.toml` for request/response serialization.
- Create `frontend/src/api.rs` with functions:
  ```rust
  pub async fn send_chat_request(
      messages: Vec<ChatMessage>,
      model: Option<String>,
  ) -> Result<ChatCompletionResponse, String>
  ```

  - Builds the request body matching the server's `ChatRequest` schema.
  - Sends `POST /api/chat/completions` using `gloo_net::http::Request`.
  - Deserializes the JSON response into `ChatCompletionResponse`.
  - Maps HTTP errors to human-readable `String` error messages.
- Define shared types in `frontend/src/types.rs` matching the server-side types:
  - `ChatMessage { role: String, content: String }`
  - `ChatCompletionResponse { id, model, choices, usage }`
  - `Choice { index, message, finish_reason }`
- Update `ChatInput`'s send handler to:
  1. Set a `loading: RwSignal<bool>` to `true`.
  2. Call `send_chat_request(messages, model)`.
  3. On success: update the last assistant `Message` content with the response text. Set `loading` to `false`.
  4. On error: update the last assistant `Message` content with an error message. Set `loading` to `false`.
- Disable the send button and textarea while `loading` is `true`.
- The API base URL should default to `""` (same origin) and be overridable via a `window.__LIBRECHAT_API_URL__` JS global or similar mechanism.

### Acceptance Criteria

- [ ] Sending a message in the UI calls `POST /api/chat/completions`
- [ ] The assistant message bubble updates with the full response text
- [ ] The send button and textarea are disabled while the request is in-flight
- [ ] Network errors display an error message in the assistant bubble
- [ ] Works with Ollama running locally (`http://localhost:11434`) via the backend proxy
- [ ] `cargo build -p frontend --target wasm32-unknown-unknown` compiles without errors

### Notes

- This is the simpler integration path. Streaming (issue #12) will replace this for a better UX.
- The `model` field can be hardcoded to `"llama3"` for now — model selection UI comes in Phase 2.

---

## Issue #12: Implement SSE streaming from Leptos frontend

**Labels:** `ui`, `streaming`, `enhancement`

### Summary

Replace the non-streaming chat integration with SSE streaming so that tokens appear in real-time as they are generated by the LLM. The frontend connects to `POST /api/chat/completions/stream` and updates the assistant message incrementally.

### Requirements

- Add `gloo-net` (or `reqwasm`) streaming support — ensure the `http` feature is enabled for response body streaming.
- Create `frontend/src/api.rs` function:
  ```rust
  pub async fn stream_chat_request(
      messages: Vec<ChatMessage>,
      model: Option<String>,
      on_chunk: impl Fn(ChatCompletionChunk),
  ) -> Result<(), String>
  ```

- The streaming function should:
  1. Send `POST /api/chat/completions/stream` with `Content-Type: application/json`.
  2. Read the response body as an SSE stream.
  3. Parse each `data: {json}` line into a `ChatCompletionChunk`.
  4. Call `on_chunk(chunk)` for each parsed chunk.
  5. Stop when `data: [DONE]` is received.
  6. Handle `event: error` lines by calling `on_chunk` with an error representation or returning an error.
- Create a custom SSE parser in `frontend/src/sse.rs`:
  - Buffer incoming text from the response body.
  - Split on `\n\n` boundaries.
  - For each event block, extract lines starting with `data: `.
  - Skip empty lines and comments (`: ...`).
  - Return parsed events as they complete.
- Update `ChatView` to:
  1. On send, call `stream_chat_request` instead of `send_chat_request`.
  2. For each chunk, append `chunk.choices[0].delta.content` to the assistant message's content signal.
  3. On stream end, set `loading` to `false`.
  4. On error, append error text to the assistant message.
- The assistant message content should update character-by-character as chunks arrive, creating a "typing" effect.
- Remove or deprecate the non-streaming `send_chat_request` function (keep it for fallback if desired).

### Acceptance Criteria

- [ ] Sending a message streams tokens in real-time to the UI
- [ ] The assistant message updates character-by-character as chunks arrive
- [ ] The "Thinking..." animation is replaced by the streamed text immediately
- [ ] `data: [DONE]` terminates the stream cleanly
- [ ] Network errors during streaming display an error in the assistant bubble
- [ ] The send button is re-enabled after the stream completes or errors
- [ ] No memory leaks from unclosed streams on rapid message sending (cancel previous stream if new message is sent)

### Notes

- `gloo-net` supports streaming response bodies via `Response::body()` which returns a `web_sys::ReadableStream`. The SSE parsing must be done in JS/WASM since there's no native SSE client in Rust for the browser.
- Consider using `web_sys::EventSource` as an alternative — but it only supports `GET` requests. Since our endpoint is `POST`, we must parse the SSE stream manually from the fetch response body.
- This issue supersedes the non-streaming integration from issue #11 for the primary UX, but the non-streaming endpoint remains available as a fallback.

---

## Labels Reference

The following custom labels need to exist on the repo (already created ✅):

| Label | Color | Description |
|---|---|---|
| `server` | `#0e8a16` | Backend Axum server, routes, and middleware |
| `ui` | `#bfeb2c` | Frontend Leptos WASM interface |
| `mcp` | `#5319e7` | LLM provider integration and API clients |
| `streaming` | `#1d76db` | SSE/WebSocket streaming of LLM responses |
| `scaffolding` | `#bfdadc` | Project structure, build pipeline, and configuration |
| `testing` | `#fbca04` | Test coverage and CI |

Plus the existing GitHub defaults: `bug`, `documentation`, `enhancement`, `good first issue`, `help wanted`.

---

## Summary: All 12 Phase 1 Issues

| # | Title | Labels | Already on GitHub? |
|---|---|---|---|
| 1 | Scaffold Cargo workspace with Axum server crate | `scaffolding`, `enhancement` | ✅ |
| 2 | Set up traditional CSS stylesheets and design system for Leptos frontend | `scaffolding`, `ui`, `enhancement` | ✅ |
| 3 | Implement minimal Axum server with health check and CORS | `server`, `enhancement` | ✅ |
| 4 | Serve Leptos WASM frontend from Axum via static file middleware | `server`, `ui`, `enhancement` | ✅ |
| 5 | Define LlmProvider trait and shared chat types | `mcp`, `enhancement` | ✅ |
| 6 | Implement OpenAI-compatible provider client (non-streaming) | `mcp`, `enhancement` | ❌ Create manually |
| 7 | Implement SSE streaming in the OpenAI-compatible provider client | `mcp`, `streaming`, `enhancement` | ❌ Create manually |
| 8 | Add non-streaming chat completion API route | `server`, `enhancement` | ❌ Create manually |
| 9 | Add streaming chat completion API route (SSE endpoint) | `server`, `streaming`, `enhancement` | ❌ Create manually |
| 10 | Build Leptos chat UI — message list and input components | `ui`, `enhancement` | ❌ Create manually |
| 11 | Connect Leptos frontend to non-streaming chat API | `ui`, `mcp`, `enhancement` | ❌ Create manually |
| 12 | Implement SSE streaming from Leptos frontend | `ui`, `streaming`, `enhancement` | ❌ Create manually |
