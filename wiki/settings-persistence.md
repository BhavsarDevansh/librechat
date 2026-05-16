# Settings Persistence

## Feature Overview

LibreChat now remembers your settings between visits. Previously, every time you refreshed the page or restarted the server you had to re-enter your API endpoint, auth key, and model preferences. With settings persistence, these choices are stored in a local SQLite database and restored automatically.

## Who This Benefits

- **Self-hosters** who run LibreChat on a Raspberry Pi or home server and want configuration to survive reboots.
- **Multi-device users** who expect the same endpoint and model selection when they reopen the app in the browser.
- **Privacy-conscious users** who prefer local storage over cloud-based configuration sync.

## User Guide

### Opening Settings

1. Click the **⚙ Settings** button at the bottom of the sidebar (or the gear icon when the sidebar is collapsed).
2. The settings modal appears.

### Configuring Your Provider

- **API Endpoint** — Enter the base URL of your LLM provider. Leave it empty to use the same origin as the LibreChat backend (the default).
  - Example for Ollama: `http://localhost:11434`
  - Example for OpenAI: `https://api.openai.com/v1`
- **Auth Key** — Paste your API key or bearer token. Click the 👁 / 🙈 toggle to show or hide the key.
  - Optional for local providers such as Ollama.
  - Required for OpenAI, Anthropic, and most cloud providers.

### Choosing a Default Model

- **Default Model** — Enter the model name you want selected for new chats.
  - Example: `llama3`, `gpt-4`, `mistral`
  - If left empty, the app falls back to `llama3`.

### Generation Parameters

- **Temperature** (optional) — Controls randomness. `0.0` is very deterministic, `1.0` is balanced, `2.0` is highly creative. Values must be between `0.0` and `2.0`.
- **Max Tokens** (optional) — Limits how many tokens the model generates per response. Must be at least `1`.

### UI Preferences

- **Collapse sidebar by default** — Check this box if you prefer the sidebar to start collapsed.

### Saving Changes

Click **Save** to store your settings. They are sent to the backend and written to the local database immediately. Click **Cancel** to discard changes.

### Sidebar Toggle

The sidebar collapse/expand button also updates your saved preference automatically, so the next time you open the app the sidebar will be in the same state you left it.

## Glossary

- **API endpoint** — The web address of the LLM service that LibreChat talks to.
- **Auth key / Bearer token** — A secret string that proves you are allowed to use a paid or private API.
- **Temperature** — A slider-like number that controls how creative or predictable the model's replies are.
- **Max tokens** — A token is roughly a word or part of a word. This setting caps the length of each reply.
- **SQLite** — A lightweight file-based database built into the LibreChat server. No separate database server is needed.

## FAQ / Troubleshooting

**Q: Where are my settings stored?**
A: In a file called `librechat.db` in the server's working directory. You can move or back up this file just like any other file.

**Q: Will my API key be safe?**
A: The key is stored in plain text inside the local database file. It is not sent to any external service except the LLM provider you configured. It is never logged by LibreChat, and error messages never echo the key back.

**Q: I changed settings but the model list is still empty.**
A: Make sure your API endpoint is correct and the provider is running. Click the refresh indicator next to the model selector to retry fetching models.

**Q: Can I use different settings per browser or device?**
A: Not yet. Settings are tied to the server instance, not the browser. Every client connecting to the same server sees the same settings.

**Q: What happens if I delete `librechat.db`?**
A: The app will recreate it on the next startup and restore factory defaults: empty endpoint, empty auth key, model `llama3`.

## Related Resources

- [Technical documentation: Settings Persistence](../docs/settings-persistence.md)
- [SQLite Persistence Foundation](../docs/sqlite-persistence.md)
- [Frontend API Integration](../docs/frontend-api-integration.md)
