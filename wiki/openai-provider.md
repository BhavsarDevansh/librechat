# OpenAI-Compatible Provider

## What is this feature?

LibreChat can now talk to AI backends that use the OpenAI Chat Completions API
— this includes **Ollama** (running locally on your machine or Raspberry Pi) and
**OpenAI** (in the cloud). Think of it as a universal adapter: the server sends
the same kind of request regardless of which backend is running, and the
provider translates it into the right format.

Right now only the **non-streaming** (full response) mode is implemented. You
send a message and get the complete reply back. Streaming — where the response
arrives word by word — is coming in a future update.

## User Guide

### Running locally with Ollama (zero config)

1. Install and start [Ollama](https://ollama.com).
2. Run `ollama pull llama3` to download the default model.
3. Start the LibreChat server — it automatically connects to
   `http://localhost:11434` with model `llama3`.

### Connecting to OpenAI

Set environment variables before starting the server:

```sh
export LLM_BASE_URL=https://api.openai.com
export LLM_API_KEY=sk-your-key-here
export LLM_MODEL=gpt-4o-mini
```

### Configuration reference

| Variable | What it does | Default |
|----------|-------------|---------|
| `LLM_BASE_URL` | Where the AI server lives | `http://localhost:11434` |
| `LLM_API_KEY` | Secret key for authentication | *(none — not needed for Ollama)* |
| `LLM_MODEL` | Which AI model to use | `llama3` |
| `LLM_CONNECT_TIMEOUT_SECS` | Seconds to wait for TCP connection | `10` |
| `LLM_TIMEOUT_SECS` | Seconds to wait for the full response | `300` |

### Timeout tips

Long-running models on slow hardware may need more than the default 5-minute
timeout. Increase it with:

```sh
export LLM_TIMEOUT_SECS=600   # 10 minutes
```

## Glossary

| Term | Meaning |
|------|---------|
| **Ollama** | A tool for running AI models locally on your own hardware. |
| **OpenAI API** | The cloud service by OpenAI that hosts models like GPT-4. |
| **Non-streaming** | Waiting for the full response before returning it (as opposed to streaming, which sends words as they arrive). |
| **Bearer token** | A security credential passed in the HTTP header to prove you're allowed to use the API. |
| **Connection pooling** | Reusing the same HTTP connection for multiple requests, which is faster and uses less memory. |
| **StreamingNotSupported** | The error returned when you try streaming on a provider that hasn't implemented it yet. |

## FAQ / Troubleshooting

**Q: I'm getting `ConnectionFailed` errors. What's wrong?**
A: The server can't reach the AI backend. Check that:
- Ollama is running (`ollama serve`).
- `LLM_BASE_URL` points to the right address.
- No firewall is blocking the port (default 11434 for Ollama).

**Q: I'm getting `ApiError { status: 401 }`. What does that mean?**
A: The API key is missing or invalid. Set `LLM_API_KEY` to a valid key for
your provider. Ollama doesn't require a key.

**Q: I'm getting `InvalidResponse` errors.**
A: The server received a response from the AI backend but couldn't understand
it. This usually means the backend isn't returning the expected JSON format.
Make sure you're pointing to an OpenAI-compatible server.

**Q: Can I use this with something other than Ollama or OpenAI?**
A: Yes — any server that implements the `/v1/chat/completions` endpoint in the
OpenAI format will work. Just set `LLM_BASE_URL` to its address.

**Q: What about streaming?**
A: Streaming support is not yet implemented. Calling the streaming method will
return a `StreamingNotSupported` error. This will be added in a future update.

**Q: My request times out before getting a response.**
A: The default request timeout is 300 seconds (5 minutes). If your model needs
more time (common on Raspberry Pi), increase `LLM_TIMEOUT_SECS`.

**Q: What happens if I set `LLM_API_KEY` to an empty string?**
A: It's treated the same as not setting it at all — no `Authorization` header
is sent. This is the right behaviour for Ollama.

## Related Resources

- [Technical documentation](../docs/openai-provider.md)
- [Provider abstraction layer docs](../docs/providers.md)
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)
- [Ollama OpenAI compatibility](https://github.com/ollama/ollama/blob/main/docs/openai.md)
