//! Integration tests for the SQLite persistence foundation (Issue #27).

use server::database::{default_database_url, init_pool, run_migrations, table_exists};
use std::sync::Mutex;

/// Mutex serialising tests that mutate the `LIBRECHAT_DATABASE_URL` environment
/// variable. Cargo runs tests in parallel, so unsynchronised `set_var` /
/// `remove_var` calls would cause data races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ---- Pool initialisation ----

#[tokio::test]
async fn test_database_pool_initialization() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));

    let pool = init_pool(&url)
        .await
        .expect("pool initialization should succeed");

    // Verify the pool is usable with a simple scalar query.
    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("should be able to query pool");
    assert_eq!(row.0, 1);
}

// ---- Migration execution ----

#[tokio::test]
async fn test_migrations_run_on_startup() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));

    let pool = init_pool(&url)
        .await
        .expect("pool initialization should succeed");
    run_migrations(&pool)
        .await
        .expect("migrations should run successfully");
}

// ---- Environment variable override ----

#[tokio::test]
async fn test_database_url_env_var_override() {
    let _lock = ENV_LOCK.lock().expect("env lock");

    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("env_test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));

    let env_key = "LIBRECHAT_DATABASE_URL";
    let original = std::env::var(env_key).ok();
    // Safety: guarded by ENV_LOCK to serialise parallel test access.
    unsafe {
        std::env::set_var(env_key, &url);
    }

    let resolved = default_database_url();

    // Restore env var
    unsafe {
        if let Some(val) = original {
            std::env::set_var(env_key, val);
        } else {
            std::env::remove_var(env_key);
        }
    }

    assert_eq!(
        resolved, url,
        "LIBRECHAT_DATABASE_URL should override the default database URL"
    );
}

// ---- Migrated table existence ----

#[tokio::test]
async fn test_migrated_table_exists() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let db_path = temp.path().join("test.db");
    let url = format!("sqlite:{}", db_path.to_str().expect("path to str"));

    let pool = init_pool(&url)
        .await
        .expect("pool initialization should succeed");
    run_migrations(&pool)
        .await
        .expect("migrations should run successfully");

    // Verify at least one expected table exists using a compile-time checked query.
    let exists = table_exists(&pool, "conversations")
        .await
        .expect("should be able to query sqlite_master");

    assert!(exists, "conversations table should exist after migrations");
}
