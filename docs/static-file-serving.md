# Static File Serving (Leptos WASM Frontend)

## Architecture & Design

The server serves the compiled Leptos WASM frontend as static files, enabling a
single-binary deployment model where the same Axum process handles both the API
and the UI.

### Component Interaction

```
main.rs
  └─ resolves port from LIBRECHAT_PORT env var (default 3000)
  └─ resolves static dir from LIBRECHAT_STATIC_DIR env var (default ../frontend/dist)
  └─ initialises tracing_subscriber with env-filter
  └─ binds TcpListener to 0.0.0.0:{port}
  └─ calls app(AppState::new()) to build the Router
  └─ serves via axum::serve

lib.rs
  └─ app(state) → Router
       ├─ route: GET /health → routes::health::health
       ├─ fallback_service: ServeDir (static files from state.static_dir)
       │    ├─ append_index_html_on_directories(true)
       │    └─ fallback: ServeFile("{static_dir}/index.html")
       ├─ layer: TraceLayer (tower-http)
       ├─ layer: CorsLayer allowlist (tower-http)
       └─ with_state(AppState)
```

### Design Decisions

- **`fallback_service` instead of nested routes**: Axum's `Router::fallback_service`
  registers the `ServeDir` as a catch-all. All API routes are registered first
  and take priority; any unmatched path falls through to the static file service.

- **SPA fallback via `ServeFile`**: The `ServeDir` is configured with a `.fallback()`
  that serves `index.html` for any path not matching a static file. This enables
  client-side routing in the Leptos WASM app — navigating to `/chat/history` will
  return `index.html`, and the Leptos router handles the route in the browser.

- **`append_index_html_on_directories(true)`**: Requests to directory paths (e.g.
  `GET /`) automatically append `index.html`, ensuring the root path serves the
  app entry point.

- **Configurable static directory**: The directory defaults to the relative path
  `../frontend/dist`, resolved against the process's current working directory
  (CWD) at runtime via `PathBuf::from`. This only matches the binary's directory
  when the server is launched from the workspace root (e.g. via `cargo run`).
  Operators can override it with the `LIBRECHAT_STATIC_DIR` environment variable
  or supply an absolute path via `AppState::with_static_dir()` at startup to avoid
  CWD-related surprises.

- **`AppState` holds `static_dir`**: The static directory path is stored in
  `AppState::static_dir` as a `PathBuf`, resolved once at startup. This avoids
  repeated env var lookups and makes the path testable.

- **Static-only state avoids provider setup**: `AppState::with_static_dir()`
  now uses a noop provider so static-file tests and callers do not construct
  the real HTTP provider or read its environment settings unnecessarily.

## API Reference

### `GET /health`

Returns the server health status. (Unchanged — see `docs/server-health-check.md`.)

### `GET /` (and all non-API paths)

Serves the Leptos WASM frontend. The static file service:

- Serves files from the configured directory (default: `../frontend/dist/`)
- Appends `index.html` for directory requests
- Falls back to `index.html` for any path not matching a file (SPA routing)
- Returns correct MIME types (`.js` → `application/javascript`, `.wasm` →
  `application/wasm`, `.html` → `text/html`, etc.)

### `pub fn app(state: AppState) -> Router`

Builds the Axum `Router` with API routes, static file fallback, and middleware.
The route priority is:

1. `GET /health` — API handler
2. All other paths — `ServeDir` fallback (static files)

### `pub struct AppState` (in `server::state`)

Shared application state with a `static_dir: PathBuf` field.

```rust
let state = AppState::new();                // resolves from env / default
let state = AppState::with_static_dir(path); // explicit override (for tests)
```

The `Default` impl and `AppState::new()` both resolve the static directory
using `LIBRECHAT_STATIC_DIR`, falling back to the relative path
`../frontend/dist` (resolved against the CWD at runtime).

## Configuration

| Variable               | Default            | Description                                      |
| ---------------------- | ------------------ | ------------------------------------------------ |
| `LIBRECHAT_PORT`       | `3000`             | TCP port the server binds to                      |
| `LIBRECHAT_STATIC_DIR` | `../frontend/dist` | Directory containing built frontend static files |
| `LIBRECHAT_ALLOWED_ORIGINS` | localhost allowlist | Comma-separated CORS allowlist for browser requests |
| `RUST_LOG`             | `server=info`     | Tracing filter (env-filter syntax)               |

**Note**: `LIBRECHAT_STATIC_DIR` defaults to a relative path resolved against
the process's current working directory. For production deployments, set it to
an absolute path to avoid CWD-related issues.

## Testing Guide

Run all server tests:

```bash
cargo test -p server
```

Run only the static file serving integration tests:

```bash
cargo test -p server --test static_file_serving
```

### Test Descriptions

| Test                                         | What it validates                                    |
| -------------------------------------------- | ---------------------------------------------------- |
| `test_get_root_returns_index_html`           | `GET /` returns `index.html` (200 OK)                |
| `test_static_file_served_with_correct_content_type` | Static files get proper MIME types              |
| `test_health_endpoint_still_returns_json`    | `/health` returns JSON, not `index.html`              |
| `test_nonexistent_path_returns_index_html_spa_fallback` | SPA fallback: unknown paths serve `index.html` |
| `test_static_dir_env_var_override`           | `LIBRECHAT_STATIC_DIR` overrides the default dir     |
| `test_default_static_dir_is_frontend_dist`   | Default `static_dir` ends with `frontend/dist`       |
| `test_with_static_dir_constructor`            | `AppState::with_static_dir()` sets the field correctly |

### Extending

Add new static file tests by importing `app` and `AppState` from the `server`
crate. Use `AppState::with_static_dir(temp_dir)` to avoid depending on a real
frontend build:

```rust
let app = app(AppState::with_static_dir(PathBuf::from("/tmp/test-static")));
```

## Migration / Upgrade Notes

- **v0.3.0 → v0.4.0**: `AppState` now has a `static_dir: PathBuf` field. The
  `Default` implementation resolves the static directory from the environment
  (falling back to `../frontend/dist`). Any code constructing `AppState` directly
  should use `AppState::new()` or `AppState::with_static_dir(path)`.

- **New dependency**: `tower-http` already included the `fs` feature; no new crate
  was added. `ServeDir` and `ServeFile` are re-exported from
  `tower_http::services`.

- **Future work**: A future issue will embed static assets into the binary using
  `include_dir!` or `rust-embed`, replacing runtime file serving for production.
