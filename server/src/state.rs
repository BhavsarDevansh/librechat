//! Shared application state for the Axum server.

use crate::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, LlmProvider,
    OpenAiProvider, ProviderError,
};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Default static directory as a relative path resolved against the process's
/// current working directory at runtime.
const DEFAULT_STATIC_DIR: &str = "frontend/dist";

/// Environment variable key for overriding the static directory.
const STATIC_DIR_ENV: &str = "LIBRECHAT_STATIC_DIR";

/// Application state shared across all request handlers via Axum's
/// [`State`](axum::extract::State) extractor.
///
/// Holds the configured LLM provider and the directory path from which static
/// frontend assets are served.
/// The directory defaults to the relative path `frontend/dist`, resolved
/// against the process's current working directory (CWD) at runtime via
/// [`resolve_static_dir`]. This only matches when the server is launched from the
/// workspace root (e.g. via `cargo run` from the workspace root).
///
/// Override the default by setting the `LIBRECHAT_STATIC_DIR` environment
/// variable or by calling [`AppState::with_static_dir`] with an absolute path
/// at startup.
#[derive(Clone)]
pub struct AppState {
    /// Shared LLM provider used by API handlers.
    pub provider: Arc<dyn LlmProvider>,
    /// Directory containing static frontend files served by `ServeDir`.
    pub static_dir: PathBuf,
}

struct NoopProvider;

#[async_trait]
impl LlmProvider for NoopProvider {
    async fn chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        Err(ProviderError::ConnectionFailed(
            "LLM provider not configured for this AppState".to_string(),
        ))
    }

    async fn chat_completion_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<mpsc::Receiver<Result<ChatCompletionChunk, ProviderError>>, ProviderError> {
        Err(ProviderError::ConnectionFailed(
            "LLM provider not configured for this AppState".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "NoopProvider"
    }
}

impl AppState {
    /// Creates a new `AppState` with default values.
    ///
    /// Resolves the static directory from the `LIBRECHAT_STATIC_DIR`
    /// environment variable, falling back to [`DEFAULT_STATIC_DIR`]. Both
    /// `new()` and the [`Default`] impl delegate to [`resolve_static_dir`],
    /// which resolves the relative default against the CWD at runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: default_provider(),
            static_dir: resolve_static_dir(),
        }
    }

    /// Creates an `AppState` with a specific static directory.
    ///
    /// Useful for testing where a temporary directory is needed, or for
    /// production deployments that require an absolute path to avoid
    /// CWD-related resolution surprises.
    #[must_use]
    pub fn with_static_dir(static_dir: PathBuf) -> Self {
        Self {
            provider: noop_provider(),
            static_dir,
        }
    }

    /// Creates an `AppState` with a specific provider and static directory.
    ///
    /// Intended for tests that need to inject a mock provider while still
    /// exercising the real router and handlers.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    pub fn with_provider_and_static_dir(
        provider: Arc<dyn LlmProvider>,
        static_dir: PathBuf,
    ) -> Self {
        Self {
            provider,
            static_dir,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the static directory path.
///
/// Checks the `LIBRECHAT_STATIC_DIR` environment variable first;
/// if unset, returns the default relative path `frontend/dist` resolved
/// against the process's current working directory.
fn resolve_static_dir() -> PathBuf {
    std::env::var(STATIC_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_STATIC_DIR))
}

fn default_provider() -> Arc<dyn LlmProvider> {
    Arc::new(OpenAiProvider::from_env())
}

fn noop_provider() -> Arc<dyn LlmProvider> {
    Arc::new(NoopProvider)
}
