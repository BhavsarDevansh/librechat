# Settings Persistence

## Overview

The settings persistence layer stores user preferences and provider configuration in SQLite so they survive browser refreshes and server restarts. This is the implementation of Issue #29 (Phase 2).

## Architecture & Design

### Data Flow

```
┌─────────────┐     GET /api/settings     ┌──────────────┐
│   Leptos    | <───────────────────────> |   Axum       │
│  Frontend   |     PUT /api/settings     │   Backend    │
└─────────────┘                          └──────┬───────┘
                                                │
                                        ┌───────▼───────┐
                                        │  SQLite       │
                                        │ app_settings  │
                                        │ (single row)  │
                                        └───────────────┘
```

### Schema

The `app_settings` table is a single-row table (id pinned to 1) that consolidates the previously unused `user_settings` and `model_preferences` tables from migration 1.

| Column            | Type    | Default    | Description                          |
|-------------------|---------|------------|--------------------------------------|
| id                | INTEGER | 1          | Fixed primary key                    |
| api_endpoint      | TEXT    | ''         | Custom backend / provider URL        |
| auth_key          | TEXT    | ''         | Bearer token for cloud providers     |
| model             | TEXT    | 'llama3'   | Default model name                   |
| temperature       | REAL    | NULL       | Generation temperature (0.0–2.0)     |
| max_tokens        | INTEGER | NULL       | Maximum tokens per generation        |
| sidebar_collapsed | INTEGER | 0          | UI sidebar collapse preference       |

### Why a single-row table?

A single-row design with `ON CONFLICT(id) DO UPDATE` avoids the complexity of tracking whether a row already exists. Every read returns either the persisted row or the Rust `Default`, and every write is an upsert.

## API Reference

### `GET /api/settings`

Returns the current settings. On a fresh database the defaults are:

```json
{
  "api_endpoint": "",
  "auth_key": "",
  "model": "llama3",
  "temperature": null,
  "max_tokens": null,
  "sidebar_collapsed": false
}
```

**Errors:**
- `503 Service Unavailable` — server started without a database pool.
- `500 Internal Server Error` — unexpected SQLite failure.

### `PUT /api/settings`

Replaces the settings document after validation. Partial updates are supported: omitted fields leave the existing value unchanged.

**Request body:**

```json
{
  "api_endpoint": "http://localhost:11434",
  "auth_key": "sk-xxx",
  "model": "gpt-4",
  "temperature": 0.7,
  "max_tokens": 2048,
  "sidebar_collapsed": true
}
```

**Validation rules:**
- `api_endpoint` ≤ 2048 characters
- `auth_key` ≤ 2048 characters
- `model` ≤ 256 characters
- `temperature` must be in `[0.0, 2.0]` when provided
- `max_tokens` must be ≥ 1 when provided

**Errors:**
- `400 Bad Request` — validation failure or malformed JSON.
- `503 Service Unavailable` — database pool missing.
- `500 Internal Server Error` — SQLite error.

## Backend Types

### `AppSettingsRow` (`server/src/database.rs`)

```rust
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AppSettingsRow {
    pub api_endpoint: String,
    pub auth_key: String,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub sidebar_collapsed: i64,
}
```

### `SettingsResponse` (`server/src/routes/settings.rs`)

API-facing response type. Converts the integer `sidebar_collapsed` to a JSON boolean.

### `UpdateSettingsRequest` (`server/src/routes/settings.rs`)

Input type for `PUT /api/settings`. All fields are `Option` so that partial updates work naturally.

## Configuration

No new environment variables are required. The existing `LIBRECHAT_DATABASE_URL` controls where the SQLite file lives.

## Testing Guide

Run the settings-specific integration tests:

```bash
cargo test -p server --test settings_persistence
```

Tests cover:
- Default values on a fresh database
- Full and partial updates
- Validation failures (temperature out of range, negative max_tokens)
- 503 response when the database pool is absent
- Persistence across new `AppState` instances (simulates server restart)
- Security: malformed JSON does not leak secrets in error responses

## Migration / Upgrade Notes

Migration `3_add_settings_schema.sql` drops the unused `user_settings` and `model_preferences` tables from migration 1 and creates `app_settings`. These tables were never referenced by Rust code, so the drop is safe for existing deployments.
