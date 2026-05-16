//! Settings modal component for configuring API endpoint, auth key, model,
//! generation parameters, and UI preferences.

use crate::state::AppState;
use leptos::prelude::*;

/// Modal overlay for application settings.
///
/// Allows the user to configure the API endpoint URL, an optional
/// authentication key, the default model, temperature, max tokens, and
/// sidebar collapse preference. Changes are saved to the backend on submit.
#[component]
pub fn SettingsModal() -> impl IntoView {
    let state = AppState::expect();

    // Local form state (staged until save).
    let (endpoint, set_endpoint) = signal(String::new());
    let (auth_key, set_auth_key) = signal(String::new());
    let (show_key, set_show_key) = signal(false);
    let (model, set_model) = signal(String::new());
    let (temperature, set_temperature) = signal(String::new());
    let (max_tokens, set_max_tokens) = signal(String::new());
    let (sidebar_collapsed, set_sidebar_collapsed) = signal(false);

    // Sync local form state from global settings when modal opens.
    Effect::new(move |_| {
        if state.settings_open.get() {
            let s = state.settings.get();
            set_endpoint.set(s.api_endpoint.clone());
            set_auth_key.set(s.auth_key.clone());
            set_model.set(s.model.clone());
            set_temperature.set(s.temperature.map(|t| t.to_string()).unwrap_or_default());
            set_max_tokens.set(s.max_tokens.map(|v| v.to_string()).unwrap_or_default());
            set_sidebar_collapsed.set(s.sidebar_collapsed);
        }
    });

    let close = move || {
        state.settings_open.set(false);
    };

    let on_save = move |_| {
        let temp_val = temperature.get();
        let max_val = max_tokens.get();
        state.settings.update(|s| {
            s.api_endpoint = endpoint.get();
            s.auth_key = auth_key.get();
            s.model = model.get();
            s.temperature = if temp_val.is_empty() {
                None
            } else {
                temp_val.parse().ok()
            };
            s.max_tokens = if max_val.is_empty() {
                None
            } else {
                max_val.parse().ok()
            };
            s.sidebar_collapsed = sidebar_collapsed.get();
        });
        state.selected_model.set(state.settings.get().model.clone());
        state
            .sidebar_collapsed
            .set(state.settings.get().sidebar_collapsed);
        state.save_settings();
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

                        <div class="form-group">
                            <label class="form-label" for="settings-model">
                                "Default Model"
                            </label>
                            <input
                                id="settings-model"
                                class="form-input"
                                type="text"
                                placeholder="llama3"
                                prop:value=move || model.get()
                                on:input:target=move |ev| set_model.set(ev.target().value())
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="temperature">
                                "Temperature"
                                <span class="form-optional">"(optional)"</span>
                            </label>
                            <input
                                id="temperature"
                                class="form-input"
                                type="number"
                                step="0.1"
                                min="0"
                                max="2"
                                placeholder="0.7"
                                prop:value=move || temperature.get()
                                on:input:target=move |ev| set_temperature.set(ev.target().value())
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label" for="max-tokens">
                                "Max Tokens"
                                <span class="form-optional">"(optional)"</span>
                            </label>
                            <input
                                id="max-tokens"
                                class="form-input"
                                type="number"
                                min="1"
                                placeholder="2048"
                                prop:value=move || max_tokens.get()
                                on:input:target=move |ev| set_max_tokens.set(ev.target().value())
                            />
                        </div>

                        <div class="form-group">
                            <label class="form-label">
                                <input
                                    type="checkbox"
                                    prop:checked=move || sidebar_collapsed.get()
                                    on:change:target=move |ev| set_sidebar_collapsed.set(ev.target().checked())
                                />
                                " Collapse sidebar by default"
                            </label>
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
