//! Chat UI components for the LibreChat frontend.
//!
//! Provides [`ChatView`], [`MessageList`], [`ChatInput`], and [`ModelSelector`]
//! — the core building blocks of the conversation interface. Messages are
//! stored per-thread in the global [`AppState`] and the active thread is
//! switched via the sidebar.

use crate::api::{self, ApiChatMessage, ApiMessageRole};
use crate::state::AppState;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

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

/// Model selector dropdown displayed in the chat header.
///
/// Fetches the available model list from the configured provider and allows
/// the user to select a model or enter a custom model name. The selected
/// model is stored in global state and applies to all new chat requests
/// unless changed.
#[component]
pub fn ModelSelector() -> impl IntoView {
    let state = AppState::expect();
    let (custom_input, set_custom_input) = signal(false);

    // Refresh models on mount.
    // Note: Effect::new with no tracked signal reads runs exactly once on mount,
    // so state.refresh_models() is invoked only at component creation, not on every render.
    Effect::new(move |_| {
        state.refresh_models();
    });

    let on_select_change = move |ev: web_sys::Event| {
        let target: web_sys::HtmlSelectElement = ev.target().unwrap().unchecked_into();
        let value = target.value();
        if value == "__custom" {
            set_custom_input.set(true);
        } else {
            set_custom_input.set(false);
            state.selected_model.set(value);
        }
    };

    let on_custom_change = move |ev: web_sys::Event| {
        let target: web_sys::HtmlInputElement = ev.target().unwrap().unchecked_into();
        let value = target.value();
        state.selected_model.set(value);
    };

    view! {
        <div class="model-selector">
            <Show
                when=move || !custom_input.get()
                fallback=move || view! {
                    <div class="model-custom-input">
                        <input
                            class="form-input model-input"
                            type="text"
                            placeholder="Model name…"
                            prop:value=move || state.selected_model.get()
                            on:input=on_custom_change
                        />
                        <button
                            class="model-back-btn"
                            on:click=move |_| set_custom_input.set(false)
                            aria-label="Back to presets"
                        >
                            "←"
                        </button>
                    </div>
                }
            >
                <select
                    class="model-select"
                    prop:value=move || state.selected_model.get()
                    on:change=on_select_change
                    aria-label="Select model"
                >
                    <Show when=move || state.models_loading.get() fallback=|| ()>
                        <option value="" disabled=true>"Loading models…"</option>
                    </Show>
                    <For
                        each=move || state.available_models.get()
                        key=|model| model.id.clone()
                        let(model)
                    >
                        {
                            let id = model.id.clone();
                            let id_display = id.clone();
                            let id_selected = model.id.clone();
                            view! {
                                <option value=move || id.clone() selected=move || state.selected_model.get() == id_selected>
                                    {move || id_display.clone()}
                                </option>
                            }
                        }
                    </For>
                    <option value="__custom">"Custom…"</option>
                </select>
            </Show>
            <Show when=move || state.models_error.get().is_some()>
                <span class="model-error-hint" title=move || state.models_error.get().unwrap_or_default()>
                    "⚠"
                </span>
            </Show>
        </div>
    }
}

/// Top-level chat view that manages the active thread's conversation,
/// loading state, and composes [`ModelSelector`], [`MessageList`] and
/// [`ChatInput`].
///
/// On send, the user message is appended to the active thread, a loading
/// state is set, and [`send_chat_request`] is called. On success the
/// assistant response is appended and loading cleared; on error an error
/// message bubble is appended and loading cleared.
///
/// If no thread is active, a welcome screen is shown with a prompt to
/// start a chat.
#[component]
pub fn ChatView() -> impl IntoView {
    let state = AppState::expect();
    let (loading, set_loading) = signal(false);
    let next_id = RwSignal::new(0usize);

    let active_messages = move || {
        state.active_messages()
    };

    let on_send = move |text: String| {
        if loading.get() {
            return;
        }

        // Ensure there is an active thread.
        if state.active_thread_id.get().is_none() {
            state.create_thread();
        }

        let active_id = state.active_thread_id.get().unwrap();

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

        // Update the thread's first message as the title.
        let is_first_message = {
            let threads = state.threads.get();
            let thread = threads.iter().find(|t| t.id == active_id);
            thread.is_none_or(|t| t.messages.is_empty())
        };

        state.threads.update(|threads| {
            if let Some(thread) = threads.iter_mut().find(|t| t.id == active_id) {
                thread.messages.push(user_msg);
                if is_first_message {
                    let title = if text.chars().count() > 30 {
                        let truncated: String = text.chars().take(30).collect();
                        format!("{truncated}…")
                    } else {
                        text.clone()
                    };
                    thread.title = title;
                }
            }
        });

        set_loading.set(true);

        let api_messages: Vec<ApiChatMessage> = {
            let threads = state.threads.get();
            let thread = threads.iter().find(|t| t.id == active_id);
            thread
                .map(|t| {
                    t.messages
                        .iter()
                        .filter(|m| !(m.role == MessageRole::Assistant && m.is_error))
                        .map(|m| ApiChatMessage {
                            role: match m.role {
                                MessageRole::User => ApiMessageRole::User,
                                MessageRole::Assistant => ApiMessageRole::Assistant,
                            },
                            content: m.content.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        let model = state.selected_model.get();
        let settings = state.settings.get();
        let endpoint = settings.api_endpoint.clone();
        let auth_key = settings.auth_key.clone();

        leptos::task::spawn_local(async move {
            let result = api::send_chat_request(&api_messages, &model, &endpoint, &auth_key).await;
            set_loading.set(false);

            let assistant_id = next_id.get();
            next_id.update(|id| *id += 1);

            let assistant_msg = match result {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        ChatMessage {
                            id: assistant_id,
                            role: MessageRole::Assistant,
                            content: choice.message.content.clone(),
                            is_error: false,
                        }
                    } else {
                        ChatMessage {
                            id: assistant_id,
                            role: MessageRole::Assistant,
                            content: "(empty response from model)".to_string(),
                            is_error: true,
                        }
                    }
                }
                Err(error) => ChatMessage {
                    id: assistant_id,
                    role: MessageRole::Assistant,
                    content: format!("{error}"),
                    is_error: true,
                },
            };

            state.threads.update(|threads| {
                if let Some(thread) = threads.iter_mut().find(|t| t.id == active_id) {
                    thread.messages.push(assistant_msg);
                }
            });
        });
    };

    view! {
        <div class="flex-column-full">
            <div class="chat-header">
                <ModelSelector />
            </div>
            <Show
                when=move || state.active_thread_id.get().is_some()
                fallback=|| view! {
                    <div class="chat-welcome">
                        <h1 class="welcome-title">"LibreChat"</h1>
                        <p class="welcome-subtitle">"Start a conversation or select a thread from the sidebar."</p>
                    </div>
                }
            >
                <MessageList messages=active_messages loading />
                <ChatInput on_send disabled=loading />
            </Show>
        </div>
    }
}

/// Renders a scrollable list of chat messages, differentiating visually
/// between User (right-aligned) and Assistant (left-aligned) messages.
///
/// Automatically scrolls to the bottom whenever new messages arrive.
#[component]
pub fn MessageList(
    messages: impl Fn() -> Vec<ChatMessage> + Copy + Send + 'static,
    #[prop(into)] loading: Signal<bool>,
) -> impl IntoView {
    let scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Scroll to bottom whenever the message list changes or loading toggles.
    Effect::new(move |_| {
        let _ = messages();
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
                each=messages
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
