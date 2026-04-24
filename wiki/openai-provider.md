# OpenAI-Compatible Provider

## What is this feature?

LibreChat can talk to AI backends that use the OpenAI Chat Completions API —
this includes **Ollama** (running locally on your machine or Raspberry Pi) and
**OpenAI** (in the cloud). Think of it as a universal adapter: the server sends
the same kind of request regardless of which backend is running, and the
provider translates it into the right format.

Both **non-streaming** (wait for the full response) and **streaming** (receive
tokens as they arrive) modes are now implemented. Streaming uses a technique
called Server-Sent Events (SSE) — the same method the OpenAI API uses — so
you see words appear one by one as the model generates them.

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

### Streaming vs non-streaming

- **Non-streaming** (`chat_completion`): The server waits until the model has
  finished generating the entire response, then returns it all at once.
- **Streaming** (`chat_completion_stream`): The server opens a connection that
  sends individual tokens as the model produces them. Each token arrives as a
  small "chunk" through a channel, so you can display text progressively. The
  stream ends gracefully when the model signals it's done.

Both modes are supported by the same `OpenAiProvider`. The server automatically
sets `"stream": true` or `"stream": false` in the request body depending on
which method you call.

### What happens if the connection drops mid-stream?

If the connection is lost while streaming, you'll receive an error
(`StreamEnded`) through the channel before it closes. If a single token contains
malformed JSON (rare, but possible with some backends), you'll get an
`InvalidResponse` error for that one token — but the stream keeps going and
subsequent tokens are delivered normally.

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
| **Non-streaming** | Waiting for the full response before returning it. |
| **Streaming** | Sending tokens one by one as the model generates them, so text appears progressively. |
| **SSE (Server-Sent Events)** | A protocol where the server sends lines starting with `data:` to push updates to the client. Each line contains a JSON chunk. |
| **Channel (mpsc)** | A Rust concurrency primitive that lets one task send data and another receive it — like a pipe between the network reader and your code. |
| **Bearer token** | A security credential passed in the HTTP header to prove you're allowed to use the API. |
| **Connection pooling** | Reusing the same HTTP connection for multiple requests, which is faster and uses less memory. |
| **`[DONE]`** | A special SSE message that signals the end of a streaming response. |
| **StreamEnded** | An error indicating the connection closed unexpectedly without receiving `[DONE]`. |

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

**Q: How do I use streaming?**
A: Call `chat_completion_stream()` instead of `chat_completion()`. You'll
receive a channel receiver that yields `Result<ChatCompletionChunk>` items
as they arrive. When the model is done, the channel closes. If something goes
wrong mid-stream, you'll get an `Err` before the channel closes.

**Q: What happens if a streaming token has bad JSON?**
A: You'll get `Err(InvalidResponse)` for that one token, but the stream
keeps going. The next valid token arrives normally. This is important because
real-world APIs occasionally send slightly malformed data.

**Q: What happens if the connection drops while streaming?**
A: You'll receive `Err(StreamEnded)` through the channel, then the channel
closes. This lets you tell the difference between a normal end-of-stream
(channel closes without error) and an unexpected disconnection.

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
