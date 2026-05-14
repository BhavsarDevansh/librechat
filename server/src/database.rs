//! SQLite persistence layer for the LibreChat server.
//!
//! Provides pool creation, migration execution, and helpers for the
//! default database URL.  The module is intentionally small so that later
//! issues can introduce repository functions rather than issuing ad-hoc SQL
//! from route handlers.
//!
//! # Compile-time checked queries
//!
//! The module demonstrates SQLx compile-time checked queries via
//! [`sqlx::query!`].  To build without a live database connection, run
//! `cargo sqlx prepare --workspace` after ensuring migrations are applied to
//! a local database and `DATABASE_URL` is exported.  The generated `.sqlx/`
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

    let opts = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true).foreign_keys(true);

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
