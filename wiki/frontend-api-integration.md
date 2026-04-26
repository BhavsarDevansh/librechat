# Frontend API Integration — Feature Guide

## Overview

The chat interface now connects to the real AI backend. When you send a message, it goes to the LibreChat server, which forwards it to the configured LLM (e.g. Ollama running locally). The assistant's response appears in the chat once the model finishes generating it.

This replaces the previous "Echo" placeholder with actual AI-powered responses.

## How to Use

1. **Make sure the backend is running** — start the server with `cargo run` (it defaults to `http://localhost:3000`).
2. **Make sure an LLM is available** — e.g. run `ollama serve` and pull a model like `llama3`.
3. **Open the app** in your browser at `http://localhost:3000`.
4. **Type a message** and press **Enter** or click **Send**.
5. While the AI is thinking, you'll see a "Thinking…" indicator with a pulsing animation.
6. Once the response arrives, the assistant's reply appears as a message bubble.
7. If something goes wrong (network error, server down), an error message appears in a red bubble.

## What Changed

| Before | After |
|--------|-------|
| Messages echoed back as "Echo: …" | Messages sent to the AI backend |
| No loading indicator | "Thinking…" pulsing indicator shown while waiting |
| Send button always enabled | Send button and textarea disabled during requests |
| No error handling | Network/HTTP errors shown in red bubbles |

## Visual Layout (Updated)

```text
┌─────────────────────────────────┐
│                                 │
│  [Your message]                 │
│                                 │
│  [AI response]                  │
│                                 │
│  ┌─ Thinking… ───────────────┐ │  ← pulsing animation while loading
│  └────────────────────────────┘ │
│                                 │
│  ┌─ ⚠ Network error: … ─────┐ │  ← red bubble on error
│  └────────────────────────────┘ │
│                                 │
├─────────────────────────────────┤
│ [Type a message…        ] [Send]│  ← both disabled during request
└─────────────────────────────────┘
```

## Configuring the API URL

By default, the frontend sends requests to the same server that serves the page (relative URLs). This works when both are on the same origin.

To override the API base URL, set a JavaScript global before the WASM module loads:

```html
<script>
  window.__LIBRECHAT_API_URL__ = "http://192.168.1.100:3000";
</script>
```

This is useful for development or when the frontend and backend are served from different origins.

## Changing the Default Model

The frontend sends `"llama3"` as the default model. To use a different model, change the `DEFAULT_MODEL` constant in `frontend/src/api.rs`:

```rust
pub const DEFAULT_MODEL: &str = "mistral";  // or any model your Ollama instance supports
```

## Glossary

| Term | Meaning |
|------|---------|
| **Non-streaming** | The full response is sent at once (not token-by-token) |
| **Ollama** | A local LLM runtime that serves OpenAI-compatible APIs |
| **CORS** | Cross-Origin Resource Sharing — browser security feature |
| **WASM** | WebAssembly — how Rust code runs directly in the browser |

## FAQ / Troubleshooting

**I see "Network error: …" in the chat.**
- Make sure the backend server is running (`cargo run`).
- Check that the API URL is correct. If you're accessing the frontend from a different host, set `window.__LIBRECHAT_API_URL__`.
- Ensure Ollama is running: `ollama serve`.

**I see "HTTP 502: …" in the chat.**
- The backend couldn't reach the LLM provider. Verify Ollama is running and the model is available: `ollama list`.

**The "Thinking…" indicator never goes away.**
- The request may have hung. Check the browser's Network tab and the server logs for errors.

**The textarea and Send button stay disabled.**
- This should only happen while a request is in-flight. If the request failed and the controls are still disabled, it's a bug — please file an issue.

## Related Resources

- Technical documentation: [`docs/frontend-api-integration.md`](../docs/frontend-api-integration.md)
- Chat UI feature: [`wiki/chat-ui.md`](chat-ui.md)
- Backend chat completions: [`wiki/chat-completions-route.md`](chat-completions-route.md)
