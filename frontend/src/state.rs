//! Global application state for the LibreChat frontend.
//!
//! Provides reactive signals for chat threads, settings, and model selection.
//! All state lives in-memory via Leptos context.  Persistence is achieved by
//! synchronising with the backend SQLite store through the history API.

use crate::api;
use crate::components::chat::ChatMessage;
use crate::components::chat::MessageRole;
use crate::history;
use leptos::prelude::*;

/// Unique identifier for a chat thread (local only).
pub type ThreadId = u64;

/// A single chat thread containing its message history and metadata.
#[derive(Debug, Clone)]
pub struct ChatThread {
    pub id: ThreadId,
    pub backend_id: Option<i64>,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    /// Number of messages already persisted to the backend.
    pub persisted_count: usize,
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
    /// Counter for generating unique message IDs.
    pub next_message_id: RwSignal<usize>,
    /// Application settings (API endpoint, auth key).
    pub settings: RwSignal<AppSettings>,
    /// Currently selected model name.
    pub selected_model: RwSignal<String>,
    /// Whether the sidebar is collapsed.
    pub sidebar_collapsed: RwSignal<bool>,
    /// Whether the settings modal is open.
    pub settings_open: RwSignal<bool>,
    /// Available models fetched from the provider.
    pub available_models: RwSignal<Vec<api::ApiModelInfo>>,
    /// Whether models are currently being fetched.
    pub models_loading: RwSignal<bool>,
    /// Error message from model fetch, if any.
    pub models_error: RwSignal<Option<String>>,
    /// Monotonic token to discard stale model-fetch responses.
    pub models_request_id: RwSignal<u64>,
    /// Error message from history operations, if any.
    pub history_error: RwSignal<Option<String>>,
    /// ID of the thread currently being persisted (deduplication guard).
    pub currently_persisting: RwSignal<Option<ThreadId>>,
    /// Thread waiting for a persist after the current one completes.
    pub pending_persist: RwSignal<Option<ThreadId>>,
}

impl AppState {
    /// Initialize and provide the app state via Leptos context.
    pub fn provide() -> Self {
        let state = Self {
            threads: RwSignal::new(Vec::new()),
            active_thread_id: RwSignal::new(None),
            next_thread_id: RwSignal::new(0),
            next_message_id: RwSignal::new(0),
            settings: RwSignal::new(AppSettings::default()),
            selected_model: RwSignal::new(api::DEFAULT_MODEL.to_string()),
            sidebar_collapsed: RwSignal::new(false),
            settings_open: RwSignal::new(false),
            available_models: RwSignal::new(Vec::new()),
            models_loading: RwSignal::new(false),
            models_error: RwSignal::new(None),
            models_request_id: RwSignal::new(0),
            history_error: RwSignal::new(None),
            currently_persisting: RwSignal::new(None),
            pending_persist: RwSignal::new(None),
        };
        provide_context(state);
        state.load_conversations();
        state
    }

    /// Retrieve the app state from Leptos context. Panics if not provided.
    pub fn expect() -> Self {
        expect_context()
    }

    // -----------------------------------------------------------------------
    // History sync
    // -----------------------------------------------------------------------

    /// Load persisted conversations from the backend.
    fn load_conversations(&self) {
        let settings = self.settings.get();
        let endpoint = settings.api_endpoint.clone();
        let auth_key = settings.auth_key.clone();
        let state = *self;

        leptos::task::spawn_local(async move {
            match history::fetch_conversations(&endpoint, &auth_key).await {
                Ok(conversations) => {
                    let mut threads = state.threads.get();
                    let mut max_id = state.next_thread_id.get();
                    for conv in conversations {
                        let local_id = conv.id as u64;
                        if local_id >= max_id {
                            max_id = local_id + 1;
                        }
                        // Update existing thread or create new one
                        if let Some(existing) =
                            threads.iter_mut().find(|t| t.backend_id == Some(conv.id))
                        {
                            existing.title =
                                conv.title.unwrap_or_else(|| format!("Chat {}", conv.id));
                        } else {
                            threads.push(ChatThread {
                                id: local_id,
                                backend_id: Some(conv.id),
                                title: conv.title.unwrap_or_else(|| format!("Chat {}", conv.id)),
                                messages: Vec::new(),
                                persisted_count: 0,
                            });
                        }
                    }
                    state.next_thread_id.set(max_id);
                    state.threads.set(threads);
                    state.history_error.set(None);
                }
                Err(err) => {
                    state.history_error.set(Some(format!("{err}")));
                }
            }
        });
    }

    /// Activate a thread, loading its messages from the backend if needed.
    pub fn activate_thread(&self, thread_id: ThreadId) {
        self.active_thread_id.set(Some(thread_id));

        let needs_load = self.threads.with(|threads| {
            let thread = threads.iter().find(|t| t.id == thread_id)?;
            thread.backend_id.map(|bid| (bid, thread.messages.len()))
        });

        if let Some((bid, _msg_count)) = needs_load {
            let settings = self.settings.get();
            let endpoint = settings.api_endpoint.clone();
            let auth_key = settings.auth_key.clone();
            let state = *self;
            leptos::task::spawn_local(async move {
                match history::fetch_conversation(&endpoint, &auth_key, bid).await {
                    Ok(detail) => {
                        let mut next_id = state.next_message_id.get();
                        let fetched: Vec<ChatMessage> = detail
                            .messages
                            .into_iter()
                            .map(|m| {
                                let id = next_id;
                                next_id += 1;
                                ChatMessage {
                                    id,
                                    role: match m.role.as_str() {
                                        "user" => MessageRole::User,
                                        _ => MessageRole::Assistant,
                                    },
                                    content: m.content,
                                    is_error: m.is_error,
                                }
                            })
                            .collect();
                        state.threads.update(|threads| {
                            if let Some(t) = threads.iter_mut().find(|t| t.id == thread_id) {
                                let tail = t
                                    .messages
                                    .split_off(t.persisted_count.min(t.messages.len()));
                                t.messages = fetched;
                                let tail_len = tail.len();
                                t.messages.extend(tail);
                                t.persisted_count = t.messages.len() - tail_len;
                            }
                        });
                        let final_next_id = state.next_message_id.get().max(next_id);
                        state.next_message_id.set(final_next_id);
                        state.history_error.set(None);
                    }
                    Err(err) => {
                        state.history_error.set(Some(format!("{err}")));
                    }
                }
            });
        }
    }

    /// Create a new thread, add it to the threads list, and set it as active.
    pub fn create_thread(&self) {
        let id = self.next_thread_id.get();
        self.next_thread_id.update(|n| *n += 1);

        let thread = ChatThread {
            id,
            backend_id: None,
            title: format!("Chat {}", id + 1),
            messages: Vec::new(),
            persisted_count: 0,
        };

        self.threads.update(|threads| threads.push(thread));
        self.active_thread_id.set(Some(id));

        // Optimistically create on backend.
        let settings = self.settings.get();
        let endpoint = settings.api_endpoint.clone();
        let auth_key = settings.auth_key.clone();
        let state = *self;
        leptos::task::spawn_local(async move {
            let req = history::ApiCreateConversationRequest {
                title: Some(format!("Chat {}", id + 1)),
                model: Some(state.selected_model.get()),
                provider: None,
            };
            match history::create_conversation(&endpoint, &auth_key, &req).await {
                Ok(conv) => {
                    let title_to_sync = state.threads.with(|threads| {
                        threads.iter().find(|t| t.id == id).and_then(|t| {
                            if t.backend_id.is_none() && !t.title.is_empty() {
                                Some(t.title.clone())
                            } else {
                                None
                            }
                        })
                    });
                    state.threads.update(|threads| {
                        if let Some(t) = threads.iter_mut().find(|t| t.id == id) {
                            t.backend_id = Some(conv.id);
                        }
                    });
                    if let Some(title) = title_to_sync {
                        let settings = state.settings.get();
                        let endpoint = settings.api_endpoint.clone();
                        let auth_key = settings.auth_key.clone();
                        let state = state;
                        leptos::task::spawn_local(async move {
                            let req = history::ApiUpdateConversationRequest {
                                title: Some(title),
                                model: None,
                                provider: None,
                            };
                            if let Err(err) =
                                history::update_conversation(&endpoint, &auth_key, conv.id, &req)
                                    .await
                            {
                                state.history_error.set(Some(format!("{err}")));
                            }
                        });
                    }
                }
                Err(err) => {
                    state.history_error.set(Some(format!("{err}")));
                }
            }
        });
    }

    /// Delete a thread by ID. If the deleted thread was active, switch to
    /// the most recent remaining thread (or None if empty).
    pub fn delete_thread(&self, thread_id: ThreadId) {
        let backend_id = self.threads.with(|threads| {
            threads
                .iter()
                .find(|t| t.id == thread_id)
                .and_then(|t| t.backend_id)
        });

        // If there is a backend conversation, attempt deletion first; only
        // remove locally on success so the user can retry on failure.
        if let Some(bid) = backend_id {
            let settings = self.settings.get();
            let endpoint = settings.api_endpoint.clone();
            let auth_key = settings.auth_key.clone();
            let state = *self;
            leptos::task::spawn_local(async move {
                match history::delete_conversation(&endpoint, &auth_key, bid).await {
                    Ok(_) => {
                        state.threads.update(|threads| {
                            threads.retain(|t| t.id != thread_id);
                            let current = state.active_thread_id.get();
                            if current == Some(thread_id) {
                                state.active_thread_id.set(threads.last().map(|t| t.id));
                            }
                        });
                        state.history_error.set(None);
                    }
                    Err(err) => {
                        state.history_error.set(Some(format!("{err}")));
                    }
                }
            });
        } else {
            // Local-only thread: remove immediately.
            self.threads.update(|threads| {
                threads.retain(|t| t.id != thread_id);
                let current = self.active_thread_id.get();
                if current == Some(thread_id) {
                    self.active_thread_id.set(threads.last().map(|t| t.id));
                }
            });
        }
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
    pub fn active_messages(&self) -> Vec<ChatMessage> {
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

    /// Persist messages for a specific thread that have not yet been saved.
    pub fn persist_thread(&self, thread_id: ThreadId) {
        if self.currently_persisting.get() == Some(thread_id) {
            self.pending_persist.set(Some(thread_id));
            return;
        }
        self.currently_persisting.set(Some(thread_id));

        let settings = self.settings.get();
        let endpoint = settings.api_endpoint.clone();
        let auth_key = settings.auth_key.clone();
        let state = *self;

        leptos::task::spawn_local(async move {
            let (backend_id, new_msgs) = match state.threads.with(|threads| {
                let thread = threads.iter().find(|t| t.id == thread_id)?;
                let backend_id = thread.backend_id?;
                let start = thread.persisted_count;
                let msgs: Vec<history::ApiAppendMessage> = thread
                    .messages
                    .iter()
                    .enumerate()
                    .skip(start)
                    .map(|(idx, m)| history::ApiAppendMessage {
                        role: match m.role {
                            MessageRole::User => "user".to_string(),
                            MessageRole::Assistant => "assistant".to_string(),
                        },
                        content: m.content.clone(),
                        sequence: idx as i64,
                        is_error: m.is_error,
                    })
                    .collect();
                Some((backend_id, msgs))
            }) {
                Some(v) => v,
                None => {
                    state.currently_persisting.set(None);
                    return;
                }
            };

            if new_msgs.is_empty() {
                state.currently_persisting.set(None);
                return;
            }

            let count = new_msgs.len();
            let req = history::ApiAppendMessagesRequest { messages: new_msgs };
            match history::append_messages(&endpoint, &auth_key, backend_id, &req).await {
                Ok(_) => {
                    state.threads.update(|threads| {
                        if let Some(t) = threads.iter_mut().find(|t| t.id == thread_id) {
                            t.persisted_count += count;
                        }
                    });
                    state.history_error.set(None);
                }
                Err(err) => {
                    state.history_error.set(Some(format!("{err}")));
                }
            }
            state.currently_persisting.set(None);
            if state.pending_persist.get() == Some(thread_id) {
                state.pending_persist.set(None);
                state.persist_thread(thread_id);
            }
        });
    }

    /// Persist any messages in the active thread that have not yet been saved.
    #[allow(dead_code)]
    /// Update the title of a thread on the backend.
    pub fn update_thread_title(&self, thread_id: ThreadId, title: String) {
        let backend_id = self.threads.with(|threads| {
            threads
                .iter()
                .find(|t| t.id == thread_id)
                .and_then(|t| t.backend_id)
        });

        if let Some(bid) = backend_id {
            let settings = self.settings.get();
            let endpoint = settings.api_endpoint.clone();
            let auth_key = settings.auth_key.clone();
            let state = *self;
            leptos::task::spawn_local(async move {
                let req = history::ApiUpdateConversationRequest {
                    title: Some(title),
                    model: None,
                    provider: None,
                };
                if let Err(err) =
                    history::update_conversation(&endpoint, &auth_key, bid, &req).await
                {
                    state.history_error.set(Some(format!("{err}")));
                }
            });
        }
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
