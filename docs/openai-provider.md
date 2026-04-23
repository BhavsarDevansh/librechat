# OpenAI-Compatible Provider Client

## Architecture & Design

The `openai` module (`server/src/providers/openai.rs`) implements the
[`LlmProvider`] trait for any backend that exposes the OpenAI Chat Completions
API. This includes local Ollama instances, OpenAI's hosted API, and any other
compatible server.

### Data Flow — Non-Streaming Request

```text
Client → Axum Handler
           │
           ▼
  OpenAiProvider::chat_completion(request)
           │
           ├─ Serialize ChatCompletionRequest → JSON (stream: false)
           ├─ POST {base_url}/v1/chat/completions
           │   └─ Authorization: Bearer {api_key} (if set)
           │
           ▼
  Provider Server (Ollama / OpenAI / …)
           │
           ▼
  Parse JSON → ChatCompletionResponse
     ─ or ─
  Map error → ProviderError::{ConnectionFailed, ApiError, InvalidResponse}
```

### Design Decisions

- **Single `reqwest::Client`**: The struct holds one `reqwest::Client` instance
  that is reused across all requests. This enables HTTP connection pooling,
  which is especially important on resource-constrained hardware (Raspberry Pi)
  where establishing a new TCP+TLS connection per request is expensive.
- **Configurable timeouts**: The client is built with a short connect timeout
  (default 10 s) and a long overall request timeout (default 300 s) so that
  slow LLM generations are not prematurely terminated. Both are configurable
  via environment variables.
- **Environment-driven configuration**: `OpenAiProvider::from_env()` reads
  `LLM_BASE_URL`, `LLM_API_KEY`, and `LLM_MODEL` from the process environment.
  Defaults target an Ollama instance on `localhost:11434` with model `llama3`,
  making zero-config local development the common case.
- **Trailing-slash normalisation**: The constructor strips trailing `/` from
  `base_url` so the URL template `{base_url}/v1/chat/completions` always
  produces a single slash.
- **Empty API key → `None`**: An empty `LLM_API_KEY` environment variable or an
  empty string passed to `new()` is treated as `None`, preventing an empty
  `Bearer` header from being sent.
- **Capped error bodies**: HTTP error response bodies are read up to 4 096
  bytes and truncated with an ellipsis, preventing unbounded memory usage from
  a misbehaving server.
- **Streaming stub**: `chat_completion_stream()` returns
  `Err(ProviderError::StreamingNotSupported)` immediately. This is distinct
  from `StreamEnded` (which represents a real mid-stream truncation) so
  callers can distinguish "not implemented" from "stream broke".
- **Error granularity**: HTTP error responses are mapped to
  `ProviderError::ApiError { status, message }` preserving the exact status
  code and (truncated) body. Connection failures map to `ConnectionFailed`, and
  malformed JSON maps to `InvalidResponse`.

## API Reference

### Struct: `OpenAiProvider`

```rust
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
}
```

| Field | Type | Description |
|-------|------|-------------|
| `client` | `reqwest::Client` | Reusable HTTP client with connection pooling. |
| `base_url` | `String` | API base URL, trailing `/` stripped (e.g. `http://localhost:11434`). |
| `api_key` | `Option<String>` | Bearer token; `None` for Ollama or if empty. |
| `model` | `String` | Default model name (e.g. `llama3`, `gpt-4o-mini`). |

### Constructors

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `(base_url: String, api_key: Option<String>, model: String) → Self` | Create with explicit configuration. Strips trailing `/`, treats empty API key as `None`. |
| `from_env` | `() → Self` | Create from environment variables (see Configuration). |

### Accessors

| Method | Return type | Description |
|--------|------------|-------------|
| `base_url()` | `&str` | The configured base URL. |
| `api_key()` | `Option<&str>` | The API key, if set. |
| `model()` | `&str` | The default model name. |

### `LlmProvider` Implementation

| Method | Behaviour |
|--------|-----------|
| `chat_completion` | Serialises request as JSON with `stream: false`, sends `POST {base_url}/v1/chat/completions`, adds `Authorization: Bearer` header if `api_key` is `Some`. Maps errors per table below. |
| `chat_completion_stream` | **Stub** — returns `Err(ProviderError::StreamingNotSupported)`. |
| `name` | Returns `"OpenAI-compatible"`. |

### Error Mapping

| Condition | `ProviderError` variant |
|-----------|------------------------|
| `reqwest` connection / timeout error | `ConnectionFailed(message)` |
| HTTP 4xx / 5xx | `ApiError { status, message }` (body capped at 4 KiB) |
| Response body not valid JSON | `InvalidResponse(message)` |

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_BASE_URL` | `http://localhost:11434` | Base URL of the OpenAI-compatible API. |
| `LLM_API_KEY` | *(unset)* | Bearer token for authentication. Empty string treated as unset. Leave unset for Ollama. |
| `LLM_MODEL` | `llama3` | Default model identifier sent in request bodies. |
| `LLM_CONNECT_TIMEOUT_SECS` | `10` | TCP connect timeout in seconds. |
| `LLM_TIMEOUT_SECS` | `300` | Overall request timeout in seconds (long for LLM generation). |

## Testing Guide

Run the OpenAI provider integration tests:

```sh
cargo test -p server --test openai_provider
```

| Test | Validates |
|------|-----------|
| `test_openai_provider_new` | Constructor sets fields correctly; `name()` returns `"OpenAI-compatible"`. |
| `test_openai_provider_new_trims_trailing_slash` | Trailing `/` is stripped from `base_url`. |
| `test_openai_provider_new_empty_api_key_becomes_none` | Empty API key string is treated as `None`. |
| `test_openai_provider_new_with_api_key` | Non-empty API key is stored and accessible. |
| `test_openai_provider_from_env_defaults_and_custom` | `from_env()` returns defaults when vars unset; reads custom values; empty `LLM_API_KEY` → `None`. |
| `test_chat_completion_successful_response` | Non-streaming request returns a well-formed `ChatCompletionResponse`. |
| `test_chat_completion_sends_authorization_header` | Bearer token is sent and recorded by the mock server. |
| `test_chat_completion_no_authorization_without_api_key` | No `Authorization` header when `api_key` is `None`. |
| `test_chat_completion_connection_failed` | Connection refused maps to `ConnectionFailed`. |
| `test_chat_completion_stream_returns_streaming_not_supported` | Streaming stub returns `StreamingNotSupported`. |
| `test_chat_completion_4xx_maps_to_api_error` | 400 status → `ApiError { status: 400, … }`. |
| `test_chat_completion_401_maps_to_api_error` | 401 status → `ApiError { status: 401, … }`. |
| `test_chat_completion_404_maps_to_api_error` | 404 status → `ApiError { status: 404, … }`. |
| `test_chat_completion_500_maps_to_api_error` | 500 status → `ApiError { status: 500, … }`. |
| `test_chat_completion_429_maps_to_api_error` | 429 status → `ApiError { status: 429, … }`. |

Tests spin up ephemeral `axum` mock servers on random ports and use TCP
readiness probes instead of fixed sleeps, ensuring deterministic startup.

## Migration / Upgrade Notes

- This is a **new module** — no breaking changes to existing code.
- `OpenAiProvider` is re-exported from `server::providers::OpenAiProvider`. The
  `openai` module itself is private; use the re-export.
- `MessageRole` now derives `PartialEq`, `Eq`, and `Hash`; non-breaking.
- A new `ProviderError::StreamingNotSupported` variant has been added. Code
  that matches exhaustively on `ProviderError` will need a new arm. The
  existing `StreamEnded` variant is unchanged.
- Streaming support (`chat_completion_stream`) is intentionally stubbed and
  will be implemented in a future issue.
