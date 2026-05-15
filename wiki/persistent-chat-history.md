# Persistent Chat History

## What it does

LibreChat now remembers your conversations even after you close the browser. When you send a message, the chat thread and its messages are saved to a local SQLite database. When you reopen the app, your previous conversations appear in the sidebar and you can continue where you left off.

## How to use

1. **Start chatting** — send a message as usual. The first message automatically becomes the conversation title.
2. **Switch threads** — click any conversation in the sidebar to load its full message history.
3. **Delete a thread** — hover over a conversation in the sidebar and click the × button. This removes it from the app and from the database.
4. **Offline-friendly** — if the database is not available, the app falls back to in-memory-only mode and shows a dismissible warning.

## Known limitations

- **No search** — you cannot search across all saved conversations yet.
- **No export** — conversations cannot be exported to JSON or Markdown.
- **Single-user** — there is no multi-user support; all conversations live in the same local database.

## Troubleshooting

**"Database not available" warning**
- This appears when the server was started without a SQLite database (e.g. `LIBRECHAT_DATABASE_URL` is set to an invalid path). The chat still works, but nothing is saved.

**Conversations appear in the sidebar but messages are empty**
- Click the conversation to trigger a background fetch. Messages are loaded on demand to keep startup fast.

**Title does not update after renaming**
- The title is synced when the first user message is sent. Renaming an existing conversation requires a backend sync that happens automatically.
