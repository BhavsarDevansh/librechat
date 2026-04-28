//! Settings modal component for configuring API endpoint and auth key.

use crate::state::AppState;
use leptos::prelude::*;

/// Modal overlay for application settings.
///
/// Allows the user to configure the API endpoint URL and an optional
/// authentication key. Changes are saved to the global `AppState` on submit.
#[component]
pub fn SettingsModal() -> impl IntoView {
    let state = AppState::expect();

    // Local form state (staged until save).
    let (endpoint, set_endpoint) = signal(String::new());
    let (auth_key, set_auth_key) = signal(String::new());
    let (show_key, set_show_key) = signal(false);

    // Sync local form state from global settings when modal opens.
    Effect::new(move |_| {
        if state.settings_open.get() {
            let s = state.settings.get();
            set_endpoint.set(s.api_endpoint.clone());
            set_auth_key.set(s.auth_key.clone());
        }
    });

    let close = move || {
        state.settings_open.set(false);
    };

    let on_save = move |_| {
        state.settings.update(|s| {
            s.api_endpoint = endpoint.get();
            s.auth_key = auth_key.get();
        });
        close();
    };

    let on_backdrop_click = move |ev: web_sys::MouseEvent| {
        if ev.target() == ev.current_target() {
            close();
        }
    };

    let on_escape = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            close();
        }
    };

    view! {
        <Show when=move || state.settings_open.get()>
            <div
                class="modal-backdrop"
                on:click=on_backdrop_click
                on:keydown=on_escape
                tabindex="-1"
                role="dialog"
                aria-modal="true"
                aria-label="Settings"
            >
                <div class="modal-content" on:keydown=on_escape>
                    <div class="modal-header">
                        <h2 class="modal-title">"Settings"</h2>
                        <button class="modal-close" on:click=move |_| close() aria-label="Close settings">
                            "×"
                        </button>
                    </div>

                    <div class="modal-body">
                        <div class="form-group">
                            <label class="form-label" for="api-endpoint">
                                "API Endpoint"
                            </label>
                            <input
                                id="api-endpoint"
                                class="form-input"
                                type="url"
                                placeholder="http://localhost:11434"
                                prop:value=move || endpoint.get()
                                on:input:target=move |ev| set_endpoint.set(ev.target().value())
                            />
                            <span class="form-hint">"Leave empty to use the current origin."</span>
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="auth-key">
                                "Auth Key"
                                <span class="form-optional">"(optional)"</span>
                            </label>
                            <div class="input-row">
                                <input
                                    id="auth-key"
                                    class="form-input"
                                    type=move || if show_key.get() { "text" } else { "password" }
                                    placeholder="sk-..."
                                    prop:value=move || auth_key.get()
                                    on:input:target=move |ev| set_auth_key.set(ev.target().value())
                                />
                                <button
                                    class="toggle-visibility-btn"
                                    on:click=move |_| set_show_key.update(|v| *v = !*v)
                                    aria-label=move || if show_key.get() { "Hide key" } else { "Show key" }
                                >
                                    {move || if show_key.get() { "🙈" } else { "👁" }}
                                </button>
                            </div>
                            <span class="form-hint">"Required for OpenAI, Anthropic, and other cloud providers."</span>
                        </div>
                    </div>

                    <div class="modal-footer">
                        <button class="btn-secondary" on:click=move |_| close()>
                            "Cancel"
                        </button>
                        <button class="btn-primary" on:click=on_save>
                            "Save"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
