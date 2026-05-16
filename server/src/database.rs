//! SQLite persistence layer for the LibreChat server.
//!
//! Provides pool creation, migration execution, repository functions for
//! conversations and messages, and helpers for the default database URL.
//!
//! Repository functions use [`sqlx::query_as`](sqlx::query_as) with
//! [`FromRow`](sqlx::FromRow) derive for straightforward CRUD.  The
//! `table_exists` helper demonstrates SQLx compile-time checked queries via
//! [`sqlx::query!`]; to build that path without a live database connection,
//! run `cargo sqlx prepare --workspace` after ensuring migrations are applied
//! to a local database and `DATABASE_URL` is exported.  The generated `.sqlx/`
//! directory is checked into version control so that CI and fresh checkouts
//! can compile offline.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

/// Default SQLite database URL used when `LIBRECHAT_DATABASE_URL` is unset.
///
/// Points to `librechat.db` in the process's current working directory,
/// which is suitable for local development.
pub const DEFAULT_DATABASE_URL: &str = "sqlite:librechat.db";

/// Environment variable key for overriding the database URL.
pub const DATABASE_URL_ENV: &str = "LIBRECHAT_DATABASE_URL";

/// Resolve the database URL.
///
/// Checks `LIBRECHAT_DATABASE_URL` first; if unset, falls back to
/// [`DEFAULT_DATABASE_URL`].
#[must_use]
pub fn default_database_url() -> String {
    std::env::var(DATABASE_URL_ENV).unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// Create a new SQLite connection pool.
///
/// The pool is cheap to clone and intended to be stored in [`AppState`].
/// The database file is created automatically if it does not already exist.
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let max_connections: u32 = std::env::var("LIBRECHAT_DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let connect_timeout_secs: u64 = std::env::var("LIBRECHAT_DB_CONNECT_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let idle_timeout_secs: u64 = std::env::var("LIBRECHAT_DB_IDLE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let max_lifetime_secs: u64 = std::env::var("LIBRECHAT_DB_MAX_LIFETIME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);

    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(connect_timeout_secs))
        .idle_timeout(Some(std::time::Duration::from_secs(idle_timeout_secs)))
        .max_lifetime(Some(std::time::Duration::from_secs(max_lifetime_secs)))
        .connect_with(opts)
        .await
}

/// Run embedded SQLx migrations against the provided pool.
///
/// Migrations live in `server/migrations/` and are embedded at compile time
/// via [`sqlx::migrate!`].
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Verify that a named table exists in the SQLite schema.
///
/// This is a **compile-time checked** query — the SQL is validated against
/// the database schema at build time.  See the module-level docs for the
/// offline-prepare workflow.
pub async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type = 'table' AND name = ?1",
        table_name
    )
    .fetch_one(pool)
    .await?;
    Ok(row.count > 0)
}

// ---------------------------------------------------------------------------
// Conversation & Message repository
// ---------------------------------------------------------------------------

/// Summary of a conversation returned by list endpoints.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ConversationSummary {
    pub id: i64,
    pub title: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A single message belonging to a conversation.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub sequence: i64,
    pub is_error: bool,
    pub created_at: Option<String>,
}

/// Create a new conversation.
///
/// Returns the auto-generated row id.
pub async fn create_conversation(
    pool: &SqlitePool,
    title: Option<&str>,
    model: Option<&str>,
    provider: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result =
        sqlx::query("INSERT INTO conversations (title, model, provider) VALUES (?1, ?2, ?3)")
            .bind(title)
            .bind(model)
            .bind(provider)
            .execute(pool)
            .await?;
    Ok(result.last_insert_rowid())
}

/// List conversations ordered by most recently updated first.
pub async fn list_conversations(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<ConversationSummary>, sqlx::Error> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    sqlx::query_as::<_, ConversationSummary>(
        "SELECT id, title, model, provider, created_at, updated_at
         FROM conversations
         ORDER BY updated_at DESC, id DESC
         LIMIT ?1 OFFSET ?2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Fetch a single conversation by id.
pub async fn get_conversation(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<ConversationSummary>, sqlx::Error> {
    sqlx::query_as::<_, ConversationSummary>(
        "SELECT id, title, model, provider, created_at, updated_at
         FROM conversations
         WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Update conversation metadata (title, model, provider).
///
/// `COALESCE` is used for each field: a `Some` value (including an empty
/// string) overwrites the existing column, while `None` leaves it unchanged.
///
/// Returns `true` if a row was updated.
pub async fn update_conversation(
    pool: &SqlitePool,
    id: i64,
    title: Option<&str>,
    model: Option<&str>,
    provider: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query(
        "UPDATE conversations
         SET title = COALESCE(?1, title),
             model = COALESCE(?2, model),
             provider = COALESCE(?3, provider)
         WHERE id = ?4",
    )
    .bind(title)
    .bind(model)
    .bind(provider)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows > 0)
}

/// Delete a conversation (cascades to messages via foreign key).
///
/// Returns `true` if a row was deleted.
pub async fn delete_conversation(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query("DELETE FROM conversations WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

/// Insert one or more messages into a conversation.
pub async fn insert_messages(
    pool: &SqlitePool,
    conversation_id: i64,
    messages: &[(String, String, i64, bool)], // (role, content, sequence, is_error)
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for (role, content, sequence, is_error) in messages {
        sqlx::query(
            "INSERT INTO messages (conversation_id, role, content, sequence, is_error)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(sequence)
        .bind(*is_error)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Fetch ordered messages for a conversation.
pub async fn get_messages(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        "SELECT id, conversation_id, role, content, sequence, is_error, created_at
         FROM messages
         WHERE conversation_id = ?1
         ORDER BY sequence ASC, id ASC",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
}

// ---------------------------------------------------------------------------
// Application settings repository (single-row table, id = 1)
// ---------------------------------------------------------------------------

/// Persisted application settings.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AppSettingsRow {
    pub api_endpoint: String,
    pub auth_key: String,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub sidebar_collapsed: i64,
}

/// Default settings used when the database row has not been created yet.
impl Default for AppSettingsRow {
    fn default() -> Self {
        Self {
            api_endpoint: String::new(),
            auth_key: String::new(),
            model: "llama3".to_string(),
            temperature: None,
            max_tokens: None,
            sidebar_collapsed: 0,
        }
    }
}

/// Fetch the single settings row, returning defaults if none exists.
pub async fn get_settings(pool: &SqlitePool) -> Result<AppSettingsRow, sqlx::Error> {
    let row = sqlx::query_as::<_, AppSettingsRow>(
        "SELECT api_endpoint, auth_key, model, temperature, max_tokens, sidebar_collapsed
         FROM app_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or_default())
}

/// Upsert the single settings row.
///
/// Uses `INSERT ... ON CONFLICT(id) DO UPDATE` so that repeated calls
/// always replace the existing row.
pub async fn upsert_settings(
    pool: &SqlitePool,
    api_endpoint: &str,
    auth_key: &str,
    model: &str,
    temperature: Option<f64>,
    max_tokens: Option<i64>,
    sidebar_collapsed: bool,
) -> Result<(), sqlx::Error> {
    let sidebar_int = if sidebar_collapsed { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO app_settings (id, api_endpoint, auth_key, model, temperature, max_tokens, sidebar_collapsed)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             api_endpoint = excluded.api_endpoint,
             auth_key = excluded.auth_key,
             model = excluded.model,
             temperature = excluded.temperature,
             max_tokens = excluded.max_tokens,
             sidebar_collapsed = excluded.sidebar_collapsed",
    )
    .bind(api_endpoint)
    .bind(auth_key)
    .bind(model)
    .bind(temperature)
    .bind(max_tokens)
    .bind(sidebar_int)
    .execute(pool)
    .await?;
    Ok(())
}
