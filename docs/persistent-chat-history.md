# Persistent Chat History

## Architecture

Chat history is stored in SQLite via the existing SQLx pool. The feature spans three layers:

1. **Database Repository** (`server/src/database.rs`)
   - `ConversationSummary` and `Message` structs map to the `conversations` and `messages` tables.
   - Repository functions provide CRUD operations and enforce ordering via `sequence ASC`.

2. **HTTP API** (`server/src/routes/history.rs`)
   - Six endpoints expose the repository to the frontend.
   - All handlers return `503 Service Unavailable` when `AppState.db_pool` is `None`.

3. **Frontend State** (`frontend/src/state.rs`)
   - `ChatThread` gains `backend_id` and `persisted_count` fields.
   - `AppState` loads summaries on startup, fetches full messages on demand, and appends new messages asynchronously after each chat turn.
   - Optimistic UI: local state updates immediately; backend sync happens in the background with rollback-free error handling via `history_error`.

## API Reference

### `GET /api/conversations`

Returns conversation summaries ordered by `updated_at DESC`.

**Response `200 OK`**
```json
[
  {
    "id": 1,
    "title": "Hello world",
    "model": "llama3",
    "provider": null,
    "created_at": "2026-05-15 10:00:00",
    "updated_at": "2026-05-15 10:05:00"
  }
]
```

### `POST /api/conversations`

Creates a new conversation.

**Request**
```json
{
  "title": "New Chat",
  "model": "llama3",
  "provider": "ollama"
}
```

**Response `200 OK`**
```json
{
  "id": 1,
  "title": "New Chat",
  "model": "llama3",
  "provider": "ollama",
  "created_at": "2026-05-15 10:00:00",
  "updated_at": "2026-05-15 10:00:00"
}
```

### `GET /api/conversations/{id}`

Fetches a single conversation with ordered messages.

**Response `200 OK`**
```json
{
  "id": 1,
  "title": "New Chat",
  "model": "llama3",
  "provider": "ollama",
  "created_at": "2026-05-15 10:00:00",
  "updated_at": "2026-05-15 10:00:00",
  "messages": [
    { "id": 1, "role": "user", "content": "Hello", "sequence": 0, "is_error": false, "created_at": "..." }
  ]
}
```

**Response `404 Not Found`**

### `PATCH /api/conversations/{id}`

Updates metadata (any field optional).

**Request**
```json
{ "title": "Renamed Chat" }
```

**Response `200 OK`** — returns updated conversation summary.

**Response `404 Not Found`**

### `POST /api/conversations/{id}/messages`

Appends one or more messages.

**Request**
```json
{
  "messages": [
    { "role": "user", "content": "Hello", "sequence": 0, "is_error": false }
  ]
}
```

**Response `200 OK`** — returns `{ "appended": N }` indicating how many messages were stored.

**Response `404 Not Found`** — conversation does not exist.

### `DELETE /api/conversations/{id}`

Deletes a conversation and cascades to its messages.

**Response `200 OK`**

**Response `404 Not Found`**

## Configuration

No new environment variables are required. The existing `LIBRECHAT_DATABASE_URL` controls where the SQLite file lives. When the variable is unset and the default `sqlite:librechat.db` is used, history is enabled automatically.

## Testing

Run the backend integration tests:

```bash
cargo test -p server --test conversation_history
```

Run the full workspace:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

## Migration Notes

Migration `2_add_chat_metadata.sql` is additive:

- Adds `model`, `provider` to `conversations`
- Adds `sequence`, `is_error` to `messages`
- Adds `update_conversations_timestamp_on_message` trigger so inserting a message bumps the parent conversation's `updated_at`

Existing rows receive `NULL` for new columns and `0` for `sequence` / `is_error`.
