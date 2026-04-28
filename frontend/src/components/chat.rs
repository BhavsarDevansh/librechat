//! Chat UI components for the LibreChat frontend.
//!
//! Provides [`ChatView`], [`MessageList`], and [`ChatInput`] — the core
//! building blocks of the conversation interface. Messages are stored in a
//! reactive signal (`Vec<ChatMessage>`) inside `ChatView` and flow downward
//! to `MessageList` for rendering.
//!
//! The `ChatView` component connects to the backend via
//! [`crate::api::send_chat_request`] and manages loading and error states
//! reactively.

use crate::api::{self, ApiChatMessage, ApiMessageRole};
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
    /// When `true`, the message content is an error string and the bubble
    /// should be styled with the error colour scheme.
    pub is_error: bool,
}

/// Top-level chat view that manages conversation history, loading state, and
/// composes [`MessageList`] and [`ChatInput`].
///
/// On send, the user message is appended to the signal list, a loading state
/// is set, and [`send_chat_request`] is called. On success the assistant
/// response is appended and loading cleared; on error an error message bubble
/// is appended and loading cleared.
#[component]
pub fn ChatView() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (loading, set_loading) = signal(false);
    let next_id = RwSignal::new(0usize);

    let on_send = move |text: String| {
        let user_id = {
            let prev = next_id.get();
            next_id.update(|id| *id += 1);
            prev
        };
        let user_msg = ChatMessage {
            id: user_id,
            role: MessageRole::User,
            content: text.clone(),
            is_error: false,
        };
        set_messages.update(move |msgs| {
            msgs.push(user_msg);
        });
        set_loading.set(true);

        let api_messages: Vec<ApiChatMessage> = {
            let msgs = messages.get();
            msgs.iter()
                .filter(|m| !(m.role == MessageRole::Assistant && m.is_error))
                .map(|m| ApiChatMessage {
                    role: match m.role {
                        MessageRole::User => ApiMessageRole::User,
                        MessageRole::Assistant => ApiMessageRole::Assistant,
                    },
                    content: m.content.clone(),
                })
                .collect()
        };

        leptos::task::spawn_local(async move {
            let result = api::send_chat_request(&api_messages, api::DEFAULT_MODEL).await;
            set_loading.set(false);

            let assistant_id = next_id.get();
            next_id.update(|id| *id += 1);

            match result {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let assistant_msg = ChatMessage {
                            id: assistant_id,
                            role: MessageRole::Assistant,
                            content: choice.message.content.clone(),
                            is_error: false,
                        };
                        set_messages.update(move |msgs| {
                            msgs.push(assistant_msg);
                        });
                    } else {
                        let error_msg = ChatMessage {
                            id: assistant_id,
                            role: MessageRole::Assistant,
                            content: "(empty response from model)".to_string(),
                            is_error: true,
                        };
                        set_messages.update(move |msgs| {
                            msgs.push(error_msg);
                        });
                    }
                }
                Err(error) => {
                    let error_msg = ChatMessage {
                        id: assistant_id,
                        role: MessageRole::Assistant,
                        content: format!("{error}"),
                        is_error: true,
                    };
                    set_messages.update(move |msgs| {
                        msgs.push(error_msg);
                    });
                }
            }
        });
    };

    view! {
        <div class="flex-column-full">
            <MessageList messages loading />
            <ChatInput on_send disabled=loading />
        </div>
    }
}

/// Renders a scrollable list of chat messages, differentiating visually
/// between User (right-aligned) and Assistant (left-aligned) messages.
///
/// Automatically scrolls to the bottom whenever new messages arrive.
#[component]
pub fn MessageList(
    messages: ReadSignal<Vec<ChatMessage>>,
    /// When `true`, a "Thinking…" indicator is shown and the list scrolls to bottom.
    #[prop(into)]
    loading: Signal<bool>,
) -> impl IntoView {
    let scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Scroll to bottom whenever the message list changes or loading toggles.
    Effect::new(move |_| {
        let _ = messages.get();
        let _ = loading.get();
        if let Some(el) = scroll_ref.get() {
            let opts = web_sys::ScrollToOptions::new();
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
                <div class={move || match (&msg.role, msg.is_error) {
                    (MessageRole::User, _) => "message-bubble message-user",
                    (MessageRole::Assistant, true) => "message-bubble message-assistant message-error",
                    (MessageRole::Assistant, false) => "message-bubble message-assistant",
                }}>
                    {move || msg.content.clone()}
                </div>
            </For>
            {move || loading.get().then(|| view! {
                <div class="message-bubble message-assistant message-loading">
                    "Thinking…"
                </div>
            })}
        </div>
    }
}

/// Input area with a textarea and send button.
///
/// - `Enter` sends the message; `Shift+Enter` inserts a new line.
/// - The send button is disabled when the input is empty **or** when the
///   `disabled` signal is `true` (i.e. a request is in-flight).
/// - The input field is cleared after sending.
#[component]
pub fn ChatInput(
    /// Callback invoked with the message text when the user sends a message.
    on_send: impl Fn(String) + Copy + 'static,
    /// When `true`, both the textarea and send button are disabled.
    #[prop(into)]
    disabled: Signal<bool>,
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
        if ev.key() == "Enter" && !ev.shift_key() && !ev.is_composing() {
            ev.prevent_default();
            handle_send();
        }
    };

    let is_send_disabled = move || disabled.get() || input_text.get().trim().is_empty();

    view! {
        <div class="sticky-input chat-input-area">
            <textarea
                class="chat-textarea"
                placeholder="Type a message…"
                prop:value=move || input_text.get()
                disabled=move || disabled.get()
                on:input:target=move |ev| {
                    set_input_text.set(ev.target().value());
                }
                on:keydown=on_keydown
            ></textarea>
            <button
                class="send-btn"
                disabled=is_send_disabled
                on:click=move |_| handle_send()
            >
                "Send"
            </button>
        </div>
    }
}
