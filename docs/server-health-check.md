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
  └─ binds TcpListener to 0.0.0.0:{port}
  └─ calls app(AppState::new()) to build the Router
  └─ serves via axum::serve

lib.rs
  └─ app(state) → Router
       ├─ route: GET /health → routes::health::health
       ├─ layer: TraceLayer (tower-http)
       ├─ layer: CorsLayer::permissive() (tower-http)
       └─ with_state(AppState)
```

### Design Decisions

- **lib/bin split**: The `app()` constructor and `AppState` live in the library
  crate so integration tests can import them directly. `main.rs` is a thin
  orchestrator that wires up tracing, resolves the port, binds the listener,
  and calls `app()`.

- **Permissive CORS**: `CorsLayer::permissive()` is deliberately chosen for
  local development convenience. This will be tightened in a future issue
  when authentication is introduced.

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
- `Access-Control-Allow-Origin: *` (CORS)

### `pub fn app(state: AppState) -> Router`

Builds the Axum `Router` with all routes and middleware. Receives `AppState`
and returns `Router<()>` (fully resolved state).

### `pub fn resolve_port() -> u16`

Reads `LIBRECHAT_PORT` from the environment. Returns the parsed value, or `3000`
if unset or invalid.

### `pub struct AppState` (in `server::state`)

Empty shared-state struct, `Clone`, with `new()` and `Default` implementations.
Ready to hold `reqwest::Client`, config, or database pools in future issues.

## Configuration

| Variable          | Default | Description                          |
| ----------------- | ------- | ------------------------------------ |
| `LIBRECHAT_PORT`  | `3000`  | TCP port the server binds to          |
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
| `test_cors_preflight_allows_all_origins`| OPTIONS preflight returns `Access-Control-Allow-Origin: *` |
| `test_cors_on_get_request`              | GET with Origin header returns `Access-Control-Allow-Origin: *` |

### Extending

Add new route tests by importing `app` and `AppState` from the `server` crate
and constructing requests with `axum::http::Request`.

## Migration / Upgrade Notes

- **v0.2.0 → v0.3.0**: The placeholder `main.rs` has been replaced. The server
  now requires `tracing`, `tracing-subscriber`, and the `trace` feature for
  `tower-http`. New crates: `tower` (dev), `http-body-util` (dev), `http` (dev).
