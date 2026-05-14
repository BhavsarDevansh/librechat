# SQLite Persistence Foundation

**Issue:** [#27](https://github.com/BhavsarDevansh/librechat/issues/27)  
**Status:** Implemented  
**Scope:** Backend (`server` crate)

---

## Architecture & Design

### Goal

Introduce a lightweight, local-first persistence layer for LibreChat using
SQLite and SQLx.  This is the **foundation** for Phase 2; later issues will
add repository functions, chat-history APIs, and UI integration.

### Data Flow

```text
┌─────────────┐     ┌──────────────┐     ┌────────────────┐
│   Binary    │────>│  AppState    │────>│  SqlitePool    │
│  (main.rs)  │     │  (state.rs)  │     │ (database.rs)  │
└─────────────┘     └──────────────┘     └────────────────┘
                                                    │
                                                    ▼
                                           ┌────────────────┐
                                           │  migrations/   │
                                           │  (embedded)    │
                                           └────────────────┘
```

1. On startup `main.rs` calls `AppState::init().await`.
2. `AppState::init()` resolves the database URL, creates a [`SqlitePool`], and
   runs pending migrations via [`sqlx::migrate!`].
3. The pool is stored in `AppState.db_pool` as `Option<SqlitePool>`.
4. Route handlers receive `AppState` via Axum's `State` extractor and can
   use `state.db_pool` when persistence logic is added in future issues.

### Design Decisions

| Decision | Rationale |
|----------|-----------|
| **SQLite (not PostgreSQL / MySQL)** | Targets Raspberry Pi and legacy hardware; zero external daemon, minimal binary overhead, single-file portability. |
| **SQLx (not Diesel / sea-orm)** | Async-first, compile-time query checking, lightweight, and already fits the Tokio / Axum stack. |
| **Optional pool in `AppState`** | Existing integration tests construct `AppState` directly and must continue to compile without a database.  The binary path always initialises the pool; test paths leave it `None`. |
| **Migrations embedded at compile time** | `sqlx::migrate!("./migrations")` means migrations ship with the binary; no external migration runner is needed. |
| **Single-row tables for settings** | `user_settings` and `model_preferences` use `PRIMARY KEY CHECK (id = 1)` so there is exactly one global configuration row, avoiding multi-user complexity that is out of scope for this issue. |

---

## API Reference

### `server::database`

```rust
pub const DEFAULT_DATABASE_URL: &str = "sqlite:librechat.db";
pub const DATABASE_URL_ENV: &str = "LIBRECHAT_DATABASE_URL";

pub fn default_database_url() -> String;
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error>;
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError>;
pub async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool, sqlx::Error>;
```

#### `default_database_url`

Reads `LIBRECHAT_DATABASE_URL`; falls back to `sqlite:librechat.db`.

#### `init_pool`

Builds a [`SqlitePool`] with `create_if_missing(true)` so the database file
is created automatically when it does not yet exist.

#### `run_migrations`

Executes all `.sql` files in `server/migrations/` in lexicographic order.
Migration state is tracked in a `_sqlx_migrations` table managed by SQLx.

#### `table_exists`

Compile-time checked query (`sqlx::query!`) returning `true` when the
named table is present in `sqlite_master`.

### `server::state::AppState`

```rust
pub struct AppState {
    pub provider: Arc<dyn LlmProvider>,
    pub static_dir: PathBuf,
    pub db_pool: Option<SqlitePool>,
}

impl AppState {
    pub fn new() -> Self;                       // no database
    pub fn with_static_dir(PathBuf) -> Self;  // no database
    pub async fn init() -> Result<Self, DatabaseInitError>;
}
```

`AppState::init()` is the **production** constructor.  It:
1. Resolves the default database URL.
2. Creates the pool.
3. Runs migrations.
4. Returns an `AppState` with `db_pool: Some(...)`.

If any step fails, a [`DatabaseInitError`] is returned with a clear message so
that the binary can log the problem and exit cleanly.

---

## Configuration

| Environment Variable | Default | Description |
|----------------------|---------|-------------|
| `LIBRECHAT_DATABASE_URL` | `sqlite:librechat.db` | SQLite connection URL.  Absolute paths are supported (`sqlite:/var/lib/librechat.db`). |

No additional environment variables or feature flags are introduced.

---

## Testing Guide

### Running the database tests

```bash
cargo test -p server --test database_persistence
```

Tests are deterministic and use **temporary SQLite files** via `tempfile`;
no real provider credentials or external network calls are required.

### Test coverage

| Test | What it validates |
|------|-------------------|
| `test_database_pool_initialization` | Pool can be created and queried. |
| `test_migrations_run_on_startup` | Migrations execute without error. |
| `test_database_url_env_var_override` | `LIBRECHAT_DATABASE_URL` overrides the default. |
| `test_migrated_table_exists` | A migrated table (`conversations`) is present after startup, verified with a compile-time checked query. |

### Extending the test suite

When adding new repository functions:
1. Create a temporary database with `tempfile::tempdir()`.
2. Call `database::init_pool(&url).await`.
3. Call `database::run_migrations(&pool).await`.
4. Exercise the new function and assert on returned data.

### Offline compilation (CI / fresh checkout)

Compile-time checked queries require schema metadata at build time.  To
enable building without a live database:

```bash
# 1. Ensure migrations are applied to a local database.
export DATABASE_URL="sqlite:librechat.db"
cargo run -p server   # or sqlx migrate run

# 2. Generate offline query metadata.
cargo sqlx prepare --workspace

# 3. Check in the generated .sqlx/ directory.
git add .sqlx/
```

The `.sqlx/` directory is already committed for this issue; future PRs that
add `query!` / `query_as!` calls must re-run `cargo sqlx prepare` and include
the updated metadata.

---

## Migration / Upgrade Notes

- **No breaking changes** to existing HTTP routes or `AppState` constructors
  used by tests (`new()`, `with_static_dir()`, `with_provider_and_static_dir()`).
- **New field** `db_pool: Option<SqlitePool>` added to `AppState`.  Code that
  constructs `AppState` with a struct literal must add `db_pool: None`.
- The binary entry point (`main.rs`) now calls `AppState::init().await` and
  exits with a clear error if the database cannot be initialised.
