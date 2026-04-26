//! Chat UI components for the LibreChat frontend.
//!
//! Provides [`ChatView`], [`MessageList`], and [`ChatInput`] — the core
//! building blocks of the conversation interface. Messages are stored in a
//! reactive signal (`Vec<ChatMessage>`) inside `ChatView` and flow downward
//! to `MessageList` for rendering.

use leptos::prelude::*;

/// Role of a participant in a chat conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageRole {
    User,
    Assistant,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: usize,
    pub role: MessageRole,
    pub content: String,
}

/// Top-level chat view that manages conversation history and composes
/// [`MessageList`] and [`ChatInput`].
///
/// For now, sending a user message immediately appends a simulated
/// assistant echo response. This will be replaced with a real backend
/// call once the streaming endpoint is wired in.
#[component]
pub fn ChatView() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let next_id = RwSignal::new(0usize);

    let on_send = move |text: String| {
        let get_next = move || {
            let prev = next_id.get();
            next_id.update(|id| *id += 1);
            prev
        };
        let user_id = get_next();
        let assistant_id = get_next();
        let user_msg = ChatMessage {
            id: user_id,
            role: MessageRole::User,
            content: text.clone(),
        };
        let assistant_msg = ChatMessage {
            id: assistant_id,
            role: MessageRole::Assistant,
            content: format!("Echo: {text}"),
        };
        set_messages.update(move |msgs| {
            msgs.push(user_msg);
            msgs.push(assistant_msg);
        });
    };

    view! {
        <div class="flex-column-full">
            <MessageList messages />
            <ChatInput on_send />
        </div>
    }
}

/// Renders a scrollable list of chat messages, differentiating visually
/// between User (right-aligned) and Assistant (left-aligned) messages.
///
/// Automatically scrolls to the bottom whenever new messages arrive.
#[component]
pub fn MessageList(messages: ReadSignal<Vec<ChatMessage>>) -> impl IntoView {
    let scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Scroll to bottom whenever the message list changes.
    Effect::new(move |_| {
        let _ = messages.get();
        if let Some(el) = scroll_ref.get() {
            let mut opts = web_sys::ScrollToOptions::new();
            opts.set_top(el.scroll_height() as f64);
            el.scroll_to_with_scroll_to_options(&opts);
        }
    });

    view! {
        <div class="scroll-area message-list" node_ref=scroll_ref>
            <For
                each=move || messages.get()
                key=|msg: &ChatMessage| msg.id
                let(msg)
            >
                <div class={move || match msg.role {
                    MessageRole::User => "message-bubble message-user",
                    MessageRole::Assistant => "message-bubble message-assistant",
                }}>
                    {move || msg.content.clone()}
                </div>
            </For>
        </div>
    }
}

/// Input area with a textarea and send button.
///
/// - `Enter` sends the message; `Shift+Enter` inserts a new line.
/// - The send button is disabled when the input is empty.
/// - The input field is cleared after sending.
#[component]
pub fn ChatInput(
    /// Callback invoked with the message text when the user sends a message.
    on_send: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let (input_text, set_input_text) = signal(String::new());

    let handle_send = move || {
        let text = input_text.get();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            on_send(trimmed.to_string());
            set_input_text.set(String::new());
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            handle_send();
        }
    };

    let is_disabled = move || input_text.get().trim().is_empty();

    view! {
        <div class="sticky-input chat-input-area">
            <textarea
                class="chat-textarea"
                placeholder="Type a message…"
                prop:value=move || input_text.get()
                on:input:target=move |ev| {
                    set_input_text.set(ev.target().value());
                }
                on:keydown=on_keydown
            ></textarea>
            <button
                class="send-btn"
                disabled=is_disabled
                on:click=move |_| handle_send()
            >
                "Send"
            </button>
        </div>
    }
}
