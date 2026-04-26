# Frontend API Integration — Technical Documentation

## Architecture & Design

This feature connects the Leptos CSR frontend to the backend non-streaming chat completions endpoint (`POST /api/chat/completions`). The architecture adds an API client layer and reactive loading/error states to the existing `ChatView` component.

### Data Flow

```text
User types message → ChatInput::on_send
  → ChatView appends User message to signal
  → ChatView sets loading = true
  → spawn_local { send_chat_request(messages) }
      → on success: append Assistant message, loading = false
      → on error:   append error message (is_error = true), loading = false
```

### Component Hierarchy (Updated)

```text
ChatView (owns signal<Vec<ChatMessage>>, signal<bool> loading)
├── MessageList (receives ReadSignal<Vec<ChatMessage>>)
│   └── For each message → <div class="message-bubble message-{role}[-message-error]">
├── Loading indicator (conditional on loading signal)
└── ChatInput (receives on_send + disabled signal)
    └── textarea + send-btn (both disabled when loading = true)
```

### Module Layout

```text
frontend/src/
├── lib.rs           — App root, declares mod api
├── api.rs           — HTTP client, API types, send_chat_request
└── components/
    ├── mod.rs        — Re-exports chat module
    └── chat.rs       — ChatView, MessageList, ChatInput, ChatMessage, MessageRole
```

## API Reference

### `api` Module (`frontend/src/api.rs`)

#### `send_chat_request`

```rust
pub async fn send_chat_request(
    messages: &[ApiChatMessage],
    model: &str,
) -> Result<ApiChatCompletionResponse, ApiError>
```

Sends a `POST /api/chat/completions` request with the given conversation history. When `model` is empty, it defaults to [`DEFAULT_MODEL`] (`"llama3"`). Returns the full non-streaming response or an [`ApiError`].

#### `ApiMessageRole`

```rust
pub enum ApiMessageRole {
    System,
    User,
    Assistant,
}
```

Serialised as lowercase strings via `#[serde(rename_all = "lowercase")]`. Mirrors the server's `MessageRole` but lives in the frontend crate to avoid WASM-incompatible server dependencies.

#### `ApiChatCompletionRequest`

```rust
pub struct ApiChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ApiChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}
```

Serialises with `skip_serializing_if = "Option::is_none"` for optional fields. The `stream` field is always set to `Some(false)` by `send_chat_request`.

#### `ApiChatCompletionResponse`

```rust
pub struct ApiChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ApiChoice>,
    pub usage: ApiUsage,
}
```

Deserialised from the backend JSON response. The `choices[0].message.content` field is used as the assistant reply text.

#### `ApiError`

```rust
pub enum ApiError {
    Network(String),       // Network-level failure (CORS, DNS, etc.)
    Http { status: u16, body: String },  // Non-2xx HTTP response
    Parse(String),         // JSON deserialisation failure
}
```

Implements `Display` for user-facing error messages displayed in the chat UI.

#### `DEFAULT_MODEL`

```rust
pub const DEFAULT_MODEL: &str = "llama3";
```

The model identifier used when no model is explicitly specified. Change this constant to target a different Ollama model.

#### `api_base_url`

```rust
fn api_base_url() -> String
```

Reads `window.__LIBRECHAT_API_URL__` from the JavaScript global scope. Returns an empty string when unset, which results in relative URLs (correct for same-origin deployments).

### `components::chat` Module (Updated)

#### `ChatMessage` (Updated)

```rust
pub struct ChatMessage {
    pub id: usize,
    pub role: MessageRole,
    pub content: String,
    pub is_error: bool,
}
```

Added `is_error: bool` field. When `true`, the message bubble is styled with the `.message-error` CSS class instead of `.message-assistant`.

#### `ChatView` (Updated)

- Added `loading` signal (`signal(false)`) to track in-flight requests.
- On send: appends User message, sets `loading = true`, spawns `send_chat_request` via `leptos::task::spawn_local`.
- On response success: appends Assistant message, sets `loading = false`.
- On response error: appends error message with `is_error = true`, sets `loading = false`.
- Renders a "Thinking…" loading indicator conditionally between `MessageList` and `ChatInput`.

#### `ChatInput` (Updated)

```rust
pub fn ChatInput(
    on_send: impl Fn(String) + Copy + 'static,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView
```

Added `disabled: Signal<bool>` prop. When `true` (during in-flight requests), both the `<textarea>` and `<button>` are disabled. The send button is also disabled when the input text is empty.

## CSS Classes (New)

| Class               | Purpose                                        |
| ------------------- | ---------------------------------------------- |
| `.message-loading`  | Pulsing "Thinking…" indicator while awaiting response |
| `.message-error`    | Error-styled assistant bubble (red background) |
| `.chat-textarea:disabled` | Reduced opacity and cursor for in-flight state |

The `.message-loading` class uses a `pulse-opacity` CSS keyframe animation (1.4s ease-in-out infinite) that fades between full and 40% opacity.

## Configuration

| Setting                       | Type   | Default | Description                                       |
| ----------------------------- | ------ | ------- | ------------------------------------------------- |
| `window.__LIBRECHAT_API_URL__` | `string` | `""`    | Override API base URL (empty = relative, same-origin) |
| `DEFAULT_MODEL` (constant)    | `&str` | `"llama3"` | Model identifier sent in chat completion requests |

## Testing Guide

Structural tests live in `server/tests/frontend_api_integration.rs` and verify:

- `frontend/src/api.rs` exists and defines expected types and functions
- `frontend/src/lib.rs` declares `mod api`
- `ChatMessage` has `is_error: bool` field
- `ChatView` uses `signal(false)` for loading, calls `send_chat_request`, and shows "Thinking…"
- `ChatInput` accepts a `disabled` prop and binds it to textarea/button
- CSS defines `.message-error` and `.message-loading` classes
- `frontend/Cargo.toml` includes `gloo-net`, `serde`, and `js-sys`

Unit tests in `frontend/src/api.rs::tests` cover:
- `ApiError::Display` implementations
- `ApiMessageRole` serialisation
- `ApiChatCompletionRequest` serialisation (skips `None` fields)
- `DEFAULT_MODEL` value

Run all tests: `cargo test`

## Migration / Upgrade Notes

- The `is_error` field was added to `ChatMessage`. Any code constructing `ChatMessage` must set `is_error: false` for normal messages.
- The `ChatInput` component now requires a `disabled` prop. Callers must pass a `Signal<bool>`.
- The previous "Echo" simulation has been replaced with real backend API calls. To revert, remove the `spawn_local` block and restore the echo logic.
- For streaming support, a future change would replace `send_chat_request` with an SSE-based streaming client and render tokens incrementally.
