# LLM Providers

## What is this feature?

LibreChat can talk to different AI backends (like Ollama running locally or
OpenAI in the cloud). This feature creates a **common language** — a shared
set of types and a trait — so the rest of the server doesn't need to know
*which* backend it's talking to. Adding a new AI provider later means writing
one implementation file, not rewriting the whole app.

## User Guide

As a user, you don't interact with the provider layer directly. It operates
behind the scenes when you send a chat message. The provider abstraction ensures
that whether you're using Ollama on a Raspberry Pi or OpenAI in the cloud, the
server handles the request in the same way.

## Glossary

| Term | Meaning |
|------|---------|
| **LLM** | Large Language Model — the AI that generates responses. |
| **Provider** | A backend that serves an LLM (e.g. Ollama, OpenAI). |
| **Trait** | A Rust interface — a contract that each provider must follow. |
| **Streaming** | Sending the AI response piece by piece as it's generated, rather than waiting for the whole response. |
| **Chat Completion** | The standard API pattern: you send messages and get a completion back. |
| **SSE** | Server-Sent Events — how streaming responses are delivered over HTTP. |
| **Channel (`mpsc`)** | A Rust concurrency primitive used to send streaming chunks from the provider to the HTTP handler. |

## FAQ / Troubleshooting

**Q: Do I need to configure anything to use providers?**
A: Not yet. This module defines the types and trait only. Configuration for
specific providers (base URLs, API keys) will come when the Ollama and OpenAI
implementations are added.

**Q: Can I add my own provider?**
A: Yes — implement the `LlmProvider` trait for your struct. The trait requires
`chat_completion`, `chat_completion_stream`, and `name` methods. See the
`MockProvider` in `server/tests/provider_trait_tests.rs` for a working example.

**Q: Why does streaming use a channel instead of a Stream?**
A: It's simpler to integrate with Axum's SSE handler. A channel receiver plugs
directly into the HTTP response loop without needing custom `Stream`
implementations.

## Related Resources

- [Technical documentation](../docs/providers.md)
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)
- [Ollama OpenAI compatibility](https://github.com/ollama/ollama/blob/main/docs/openai.md)
