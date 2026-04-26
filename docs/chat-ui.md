# Chat UI — Technical Documentation

## Architecture & Design

The chat interface is built with Leptos CSR (Client-Side Rendering) components that compose a reactive signal-driven conversation view. The architecture follows a simple unidirectional data flow:

```text
ChatView (owns signal<Vec<ChatMessage>>)
├── MessageList (receives ReadSignal<Vec<ChatMessage>>)
│   └── For each message → <div class="message-bubble message-{role}">
└── ChatInput (receives on_send callback)
    └── textarea + send-btn
```

### Component Hierarchy

| Component   | Responsibility                                          |
| ----------- | -------------------------------------------------------- |
| `ChatView`  | Owns conversation state, composes `MessageList` + `ChatInput` |
| `MessageList` | Renders `Vec<ChatMessage>` with auto-scroll             |
| `ChatInput`  | Textarea + send button, keyboard handling, validation   |

### Data Flow

1. `ChatView` creates a `signal(Vec<ChatMessage>::new())` and an `RwSignal` for ID generation.
2. The `on_send` callback pushes a `User` message and a simulated `Assistant` echo message into the signal.
3. `MessageList` receives a `ReadSignal<Vec<ChatMessage>>` and reactively renders via `<For/>`.
4. `ChatInput` receives the `on_send` callback and calls it when the user presses Enter or clicks Send.

### Why Signals?

Leptos signals (`signal()` and `RwSignal`) provide fine-grained reactivity without virtual DOM diffing. The `Vec<ChatMessage>` signal ensures that only the new message DOM nodes are created when messages are appended, thanks to the keyed `<For/>` component.

## API Reference

### `MessageRole`

```rust
pub enum MessageRole {
    User,
    Assistant,
}
```

Enum representing the role of a chat participant. Used to differentiate visual styling (alignment, color) and will later drive backend request construction.

- Derives: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`

### `ChatMessage`

```rust
pub struct ChatMessage {
    pub id: usize,
    pub role: MessageRole,
    pub content: String,
}
```

A single message in the conversation history.

| Field     | Type            | Description                              |
| --------- | --------------- | ---------------------------------------- |
| `id`      | `usize`         | Unique identifier used as `<For/>` key   |
| `role`    | `MessageRole`   | Whether this is a User or Assistant message |
| `content` | `String`        | The message text                         |

- Derives: `Debug`, `Clone`

### `ChatView`

```rust
#[component]
pub fn ChatView() -> impl IntoView
```

Top-level chat shell. Creates the conversation signal and composes `MessageList` and `ChatInput`. Currently simulates assistant responses with an "Echo: {input}" pattern. This will be replaced by a real backend call.

**Props**: None (stateful — manages its own signals).

### `MessageList`

```rust
#[component]
pub fn MessageList(messages: ReadSignal<Vec<ChatMessage>>) -> impl IntoView
```

Renders a scrollable list of chat messages with auto-scroll-to-bottom behaviour.

| Prop       | Type                        | Description                      |
| ---------- | --------------------------- | -------------------------------- |
| `messages` | `ReadSignal<Vec<ChatMessage>>` | Reactive signal of all messages |

**Auto-scroll**: Uses a `NodeRef<HtmlDivElement>` and an `Effect` that scrolls to `f64::MAX` whenever `messages` updates.

### `ChatInput`

```rust
#[component]
pub fn ChatInput(on_send: impl Fn(String) + Copy + 'static) -> impl IntoView
```

Input area with textarea and send button.

| Prop      | Type                          | Description                     |
| --------- | ----------------------------- | ------------------------------- |
| `on_send` | `impl Fn(String) + Copy + 'static` | Callback invoked with message text |

**Keyboard behaviour**:
- `Enter` → sends the message
- `Shift+Enter` → inserts a newline (default textarea behaviour)

**Validation**:
- Send button is `disabled` when the input text is empty.
- Input field is cleared after a successful send.

## CSS Classes

All chat UI classes are defined in `frontend/style/main.css` (sections 7–8) and reference the project's design tokens.

| Class                | Purpose                                      |
| -------------------- | -------------------------------------------- |
| `.message-list`      | Flex column container for message bubbles    |
| `.message-bubble`    | Base bubble style (max-width, padding, radius) |
| `.message-user`      | Right-aligned, accent background             |
| `.message-assistant` | Left-aligned, secondary background           |
| `.chat-input-area`   | Flex row for textarea + send button          |
| `.chat-textarea`     | Styled input textarea                        |
| `.send-btn`          | Accent-coloured send button with :disabled state |

Responsive breakpoint at `480px` switches the input area to a vertical stack and widens message bubbles to 90%.

## Configuration

No environment variables or feature flags are introduced by this feature. The component tree is wired into `App` via the `components::chat::ChatView` import.

## Testing Guide

Integration tests live in `server/tests/chat_ui.rs` and verify:

- Component file existence (`chat.rs`, `mod.rs`)
- Struct and enum definitions (`ChatMessage`, `MessageRole`)
- Component definitions (`ChatView`, `MessageList`, `ChatInput`)
- Signal usage and `<For/>` iteration
- Keyboard handling and disabled-state logic
- CSS class definitions, alignment rules, responsive breakpoint
- App integration (imports, `ChatView` rendered in `App`)

Run with: `cargo test -p server --test chat_ui`

## Migration / Upgrade Notes

When replacing the simulated echo with a real backend call:

1. Change `on_send` in `ChatView` to call the `/api/chat/completions` endpoint.
2. For streaming, add a pending state signal (`RwSignal<bool>`) and use SSE to append assistant tokens incrementally.
3. The `MessageRole` type mirrors the backend's `server::providers::types::MessageRole` but is kept separate in the frontend to avoid WASM-incompatible server dependencies.
