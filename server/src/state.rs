//! Shared application state for the Axum server.

use crate::database::{default_database_url, init_pool, run_migrations};
use crate::providers::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, LlmProvider, ModelInfo,
    OpenAiProvider, ProviderError,
};
use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
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
/// Holds the configured LLM provider, the directory path from which static
/// frontend assets are served, and an optional SQLite connection pool.
#[derive(Clone)]
pub struct AppState {
    /// Shared LLM provider used by API handlers.
    pub provider: Arc<dyn LlmProvider>,
    /// Directory containing static frontend files served by `ServeDir`.
    pub static_dir: PathBuf,
    /// SQLite connection pool when persistence is enabled.
    pub db_pool: Option<SqlitePool>,
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

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Err(ProviderError::ConnectionFailed(
            "LLM provider not configured for this AppState".to_string(),
        ))
    }

    fn name(&self) -> &str {
        "NoopProvider"
    }
}

/// Error type for database initialization failures.
#[derive(Debug)]
pub struct DatabaseInitError {
    pub message: String,
}

impl std::fmt::Display for DatabaseInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DatabaseInitError {}

impl AppState {
    /// Creates a new `AppState` with default values **without** a database pool.
    ///
    /// Resolves the static directory from the `LIBRECHAT_STATIC_DIR`
    /// environment variable, falling back to [`DEFAULT_STATIC_DIR`].
    ///
    /// This constructor is used by integration tests that do not need
    /// persistence, as well as by [`Default`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: default_provider(),
            static_dir: resolve_static_dir(),
            db_pool: None,
        }
    }

    /// Creates an `AppState` with a specific static directory.
    ///
    /// Useful for testing where a temporary directory is needed, or for
    /// production deployments that require an absolute path to avoid
    /// CWD-related resolution surprises.  No database pool is created.
    #[must_use]
    pub fn with_static_dir(static_dir: PathBuf) -> Self {
        Self {
            provider: noop_provider(),
            static_dir,
            db_pool: None,
        }
    }

    /// Creates an `AppState` with a specific provider and static directory.
    ///
    /// Intended for tests that need to inject a mock provider while still
    /// exercising the real router and handlers.  No database pool is created.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    pub fn with_provider_and_static_dir(
        provider: Arc<dyn LlmProvider>,
        static_dir: PathBuf,
    ) -> Self {
        Self {
            provider,
            static_dir,
            db_pool: None,
        }
    }

    /// Initialise an `AppState` with the default SQLite database.
    ///
    /// Connects to the database URL resolved by [`default_database_url`],
    /// creates the connection pool, and runs pending migrations.  If either
    /// step fails the error is returned so that the binary can exit with a
    /// clear message at startup.
    ///
    /// # Errors
    ///
    /// Returns [`DatabaseInitError`] when the pool cannot be created or
    /// migrations fail.
    pub async fn init() -> Result<Self, DatabaseInitError> {
        let database_url = default_database_url();
        let pool = init_pool(&database_url)
            .await
            .map_err(|e| DatabaseInitError {
                message: format!("failed to connect to SQLite database at {database_url}: {e}"),
            })?;
        run_migrations(&pool).await.map_err(|e| DatabaseInitError {
            message: format!("failed to run database migrations: {e}"),
        })?;
        let mut state = Self::new();
        state.db_pool = Some(pool);
        Ok(state)
    }

    /// Initialise an `AppState` with a specific database URL.
    ///
    /// Available in tests so that each test can use its own temporary database
    /// file without mutating the global `LIBRECHAT_DATABASE_URL` variable.
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn with_database_url(database_url: &str) -> Result<Self, DatabaseInitError> {
        let pool = init_pool(database_url)
            .await
            .map_err(|e| DatabaseInitError {
                message: format!("failed to connect to SQLite database at {database_url}: {e}"),
            })?;
        run_migrations(&pool).await.map_err(|e| DatabaseInitError {
            message: format!("failed to run database migrations: {e}"),
        })?;
        let mut state = Self::new();
        state.db_pool = Some(pool);
        Ok(state)
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
