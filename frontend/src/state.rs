//! Global application state for the LibreChat frontend.
//!
//! Provides reactive signals for chat threads, settings, and model selection.
//! All state lives in-memory via Leptos context — no persistence layer yet.

use crate::api::{self, ApiModelInfo};
use leptos::prelude::*;

/// Unique identifier for a chat thread.
pub type ThreadId = usize;

/// A single chat thread containing its message history and metadata.
#[derive(Debug, Clone)]
pub struct ChatThread {
    pub id: ThreadId,
    pub title: String,
    pub messages: Vec<super::components::chat::ChatMessage>,
}

/// Application-wide settings (in-memory, not persisted).
#[derive(Debug, Clone, Default)]
pub struct AppSettings {
    /// API endpoint URL (e.g. "http://localhost:11434" for Ollama).
    pub api_endpoint: String,
    /// Optional authentication key for the API.
    pub auth_key: String,
}

/// Provides the global application state via Leptos context.
/// Call once at the app root.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
    /// All chat threads.
    pub threads: RwSignal<Vec<ChatThread>>,
    /// ID of the currently active thread.
    pub active_thread_id: RwSignal<Option<ThreadId>>,
    /// Counter for generating unique thread IDs.
    pub next_thread_id: RwSignal<ThreadId>,
    /// Application settings (API endpoint, auth key).
    pub settings: RwSignal<AppSettings>,
    /// Currently selected model name.
    pub selected_model: RwSignal<String>,
    /// Whether the sidebar is collapsed.
    pub sidebar_collapsed: RwSignal<bool>,
    /// Whether the settings modal is open.
    pub settings_open: RwSignal<bool>,
    /// Available models fetched from the provider.
    pub available_models: RwSignal<Vec<ApiModelInfo>>,
    /// Whether models are currently being fetched.
    pub models_loading: RwSignal<bool>,
    /// Error message from model fetch, if any.
    pub models_error: RwSignal<Option<String>>,
    /// Monotonic token to discard stale model-fetch responses.
    pub models_request_id: RwSignal<u64>,
}

impl AppState {
    /// Initialize and provide the app state via Leptos context.
    pub fn provide() -> Self {
        let state = Self {
            threads: RwSignal::new(Vec::new()),
            active_thread_id: RwSignal::new(None),
            next_thread_id: RwSignal::new(0),
            settings: RwSignal::new(AppSettings::default()),
            selected_model: RwSignal::new(api::DEFAULT_MODEL.to_string()),
            sidebar_collapsed: RwSignal::new(false),
            settings_open: RwSignal::new(false),
            available_models: RwSignal::new(Vec::new()),
            models_loading: RwSignal::new(false),
            models_error: RwSignal::new(None),
            models_request_id: RwSignal::new(0),
        };
        provide_context(state);
        state
    }

    /// Retrieve the app state from Leptos context. Panics if not provided.
    pub fn expect() -> Self {
        expect_context()
    }

    /// Create a new thread, add it to the threads list, and set it as active.
    pub fn create_thread(&self) {
        let id = self.next_thread_id.get();
        self.next_thread_id.update(|n| *n += 1);

        let thread = ChatThread {
            id,
            title: format!("Chat {}", id + 1),
            messages: Vec::new(),
        };

        self.threads.update(|threads| threads.push(thread));
        self.active_thread_id.set(Some(id));
    }

    /// Delete a thread by ID. If the deleted thread was active, switch to
    /// the most recent remaining thread (or None if empty).
    pub fn delete_thread(&self, thread_id: ThreadId) {
        self.threads.update(|threads| {
            threads.retain(|t| t.id != thread_id);
            let current = self.active_thread_id.get();
            if current == Some(thread_id) {
                self.active_thread_id.set(threads.last().map(|t| t.id));
            }
        });
    }

    /// Get the active thread by value (clones the entire thread including messages).
    /// Returns None if no thread is active.
    #[allow(dead_code)]
    pub fn active_thread(&self) -> Option<ChatThread> {
        let active_id = self.active_thread_id.get()?;
        self.threads
            .with(|threads| threads.iter().find(|t| t.id == active_id).cloned())
    }

    /// Get the active thread's messages directly, avoiding cloning the full ChatThread.
    /// Returns an empty Vec if no thread is active.
    pub fn active_messages(&self) -> Vec<super::components::chat::ChatMessage> {
        let active_id = match self.active_thread_id.get() {
            Some(id) => id,
            None => return Vec::new(),
        };
        self.threads.with(|threads| {
            threads
                .iter()
                .find(|t| t.id == active_id)
                .map(|t| t.messages.clone())
                .unwrap_or_default()
        })
    }

    /// Fetch the list of available models from the configured provider.
    pub fn refresh_models(&self) {
        let endpoint = self.settings.get().api_endpoint.clone();
        let auth_key = self.settings.get().auth_key.clone();
        self.models_loading.set(true);
        self.models_error.set(None);
        let request_id = self.models_request_id.get() + 1;
        self.models_request_id.set(request_id);

        let state = *self;
        leptos::task::spawn_local(async move {
            match api::fetch_models(&endpoint, &auth_key).await {
                Ok(models) => {
                    if state.models_request_id.get() == request_id {
                        state.available_models.set(models);
                        state.models_loading.set(false);
                    }
                }
                Err(error) => {
                    if state.models_request_id.get() == request_id {
                        state.models_error.set(Some(error.to_string()));
                        state.available_models.set(Vec::new());
                        state.models_loading.set(false);
                    }
                }
            }
        });
    }
}
