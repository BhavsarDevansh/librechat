# Minimal Axum Server with Health Check

## Architecture & Design

The server follows a modular architecture where the binary entry point (`main.rs`)
is separated from the library crate (`lib.rs`). This split enables integration
testing without requiring a running server — tests call `app()` directly via
`tower::ServiceExt::oneshot`.

### Component Interaction

```
main.rs
  └─ resolves port from LIBRECHAT_PORT env var (default 3000)
  └─ initialises tracing_subscriber with env-filter
  └─ binds TcpListener to 127.0.0.1:{port}
  └─ calls app(AppState::new()) to build the Router
  └─ serves via axum::serve

lib.rs
  └─ app(state) → Router
       ├─ route: GET /health → routes::health::health
       ├─ layer: TraceLayer (tower-http)
       ├─ layer: CorsLayer allowlist (tower-http)
       └─ with_state(AppState)
```

### Design Decisions

- **lib/bin split**: The `app()` constructor and `AppState` live in the library
  crate so integration tests can import them directly. `main.rs` is a thin
  orchestrator that wires up tracing, resolves the port, binds the listener,
  and calls `app()`.

- **Allowlisted CORS**: `CorsLayer` is configured from
  `LIBRECHAT_ALLOWED_ORIGINS` when set, and otherwise falls back to a small set
  of common localhost development origins. Credentials are disabled explicitly.

- **Zero-copy where possible**: The health handler returns `axum::Json` which
  serialises directly into the response body, avoiding intermediate allocations.

## API Reference

### `GET /health`

Returns the server health status.

**Response**: `200 OK`

```json
{"status": "ok"}
```

**Headers**:
- `Content-Type: application/json`
- `Access-Control-Allow-Origin: <allowed origin>` when the request origin is on
  the configured allowlist

### `pub fn app(state: AppState) -> Router`

Builds the Axum `Router` with all routes and middleware. Receives `AppState`
and returns `Router<()>` (fully resolved state).

### `pub fn resolve_port() -> u16`

Reads `LIBRECHAT_PORT` from the environment. Returns the parsed value, or `3000`
if unset or invalid.

### `pub struct AppState` (in `server::state`)

Shared state holding the configured provider and static directory. `new()`
builds the default provider from environment variables; `with_static_dir()`
uses a lightweight noop provider for static-only scenarios.

## Configuration

| Variable          | Default | Description                          |
| ----------------- | ------- | ------------------------------------ |
| `LIBRECHAT_PORT`  | `3000`  | TCP port the server binds to          |
| `LIBRECHAT_ALLOWED_ORIGINS` | localhost allowlist | Comma-separated CORS allowlist |
| `RUST_LOG`        | `server=info` | Tracing filter (env-filter syntax) |

## Testing Guide

Run all server tests:

```bash
cargo test -p server
```

Run only the health check integration tests:

```bash
cargo test -p server --test health_check
```

### Test Descriptions

| Test                                    | What it validates                              |
| --------------------------------------- | ---------------------------------------------- |
| `test_health_endpoint_returns_200_ok`   | `/health` responds with HTTP 200                |
| `test_health_endpoint_returns_json_status_ok` | Body parses as `{"status":"ok"}`         |
| `test_health_endpoint_content_type_is_json` | Response has `Content-Type: application/json` |
| `test_cors_preflight_allows_default_local_origin` | OPTIONS preflight allows default localhost origin |
| `test_cors_on_get_request_for_default_local_origin` | GET with localhost origin returns matching CORS header |
| `test_cors_does_not_allow_unlisted_origin` | Unlisted origins do not receive `Access-Control-Allow-Origin` |

### Extending

Add new route tests by importing `app` and `AppState` from the `server` crate
and constructing requests with `axum::http::Request`.

## Migration / Upgrade Notes

- **v0.2.0 → v0.3.0**: The placeholder `main.rs` has been replaced. The server
  now requires `tracing`, `tracing-subscriber`, and the `trace` feature for
  `tower-http`. New crates: `tower` (dev), `http-body-util` (dev), `http` (dev).
