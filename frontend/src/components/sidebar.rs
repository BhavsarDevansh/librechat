//! Collapsible sidebar component showing chat threads and a settings button.

use crate::state::AppState;
use leptos::prelude::*;

/// Collapsible sidebar panel with thread list and settings trigger.
#[component]
pub fn Sidebar() -> impl IntoView {
    let state = AppState::expect();

    let toggle_sidebar = move |_| {
        state.sidebar_collapsed.update(|c| *c = !*c);
        state.settings.update(|s| {
            s.sidebar_collapsed = state.sidebar_collapsed.get();
        });
        state.save_settings();
    };

    let new_chat = move |_| {
        state.create_thread();
    };

    let open_settings = move |_| {
        state.settings_open.set(true);
    };

    view! {
        <nav
            class="sidebar"
            class:sidebar-collapsed=move || state.sidebar_collapsed.get()
            aria-label="Chat navigation"
        >
            <div class="sidebar-header">
                <button
                    class="sidebar-toggle"
                    on:click=toggle_sidebar
                    aria-label=move || if state.sidebar_collapsed.get() { "Expand sidebar" } else { "Collapse sidebar" }
                >
                    {move || if state.sidebar_collapsed.get() { "☰" } else { "✕" }}
                </button>
                <Show when=move || !state.sidebar_collapsed.get()>
                    <span class="sidebar-title">"Chats"</span>
                </Show>
            </div>

            <Show when=move || !state.sidebar_collapsed.get()>
                <button class="new-chat-btn" on:click=new_chat>
                    "+ New Chat"
                </button>

                <div class="thread-list" role="list">
                    <For
                        each=move || state.threads.get()
                        key=|thread| thread.id
                        let(thread)
                    >
                        <div
                            class="thread-item"
                            class:thread-active=move || state.active_thread_id.get() == Some(thread.id)
                            role="button"
                            aria-selected=move || state.active_thread_id.get() == Some(thread.id)
                            tabindex="0"
                            on:click=move |_| {
                                state.activate_thread(thread.id);
                            }
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" || ev.key() == " " {
                                    ev.prevent_default();
                                    state.activate_thread(thread.id);
                                }
                            }
                        >
                            {
                                let title = thread.title.clone();
                                let title_aria = thread.title.clone();
                                view! {
                                    <span class="thread-title">{move || title.clone()}</span>
                                    <button
                                        class="thread-delete"
                                        aria-label=move || format!("Delete {}", title_aria)
                                        on:click=move |ev| {
                                           ev.stop_propagation();
                                           state.delete_thread(thread.id);
                                       }
                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                            if ev.key() == "Enter" || ev.key() == " " {
                                                ev.stop_propagation();
                                                ev.prevent_default();
                                                state.delete_thread(thread.id);
                                            }
                                        }
                                    >
                                        "×"
                                    </button>
                                }
                            }
                        </div>
                    </For>
                </div>
            </Show>

            <div class="sidebar-footer">
                <button class="settings-btn" on:click=open_settings aria-label="Open settings">
                    <Show when=move || !state.sidebar_collapsed.get() fallback=|| view! { "⚙" }>
                        "⚙ Settings"
                    </Show>
                </button>
            </div>
        </nav>
    }
}
