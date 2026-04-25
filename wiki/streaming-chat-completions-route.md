# Streaming Chat Completions API (SSE)

## Feature Overview

LibreChat now supports real-time streaming responses at
`POST /api/chat/completions/stream`. Instead of waiting for the entire answer,
the server sends each piece of text as it arrives from the language model, using
a technology called Server-Sent Events (SSE).

In everyday terms: you send a chat request, and the response comes back piece by
piece — like watching someone type — rather than all at once after a long wait.
This makes the app feel faster and more responsive, especially for longer
replies.

## User Guide

Send a streaming request like this:

```bash
curl -N http://localhost:3000/api/chat/completions/stream \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "llama3",
    "messages": [
      { "role": "user", "content": "Tell me a story" }
    ],
    "stream": true
  }'
```

The `-N` flag disables buffering so you can see chunks arrive in real time.

The server responds with a stream of events, each prefixed with `data:`:

```text
data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":"Once"},"finish_reason":null}]}

data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":" upon"},"finish_reason":null}]}

data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":" a"},"finish_reason":null}]}

data: {"id":"chatcmpl-1","model":"llama3","choices":[{"index":0,"delta":{"content":" time"},"finish_reason":null}]}

data: [DONE]
```

The `data: [DONE]` message signals that the stream has finished.

If something goes wrong mid-stream, the server sends an error event:

```text
event: error
data: Connection failed: upstream disconnected
```

After an error event, no more data is sent and the stream closes.

## Glossary

- **SSE (Server-Sent Events)**: A web standard for streaming data from a
  server to a browser. Each message is prefixed with `data:` and separated by
  blank lines.
- **Chunk**: A small piece of the model's response (usually a few tokens of
  text) delivered as it becomes available.
- **`data: [DONE]`**: The special end-of-stream sentinel defined by OpenAI's
  streaming protocol.
- **Provider**: The backend service that generates completions, such as Ollama
  or an OpenAI-compatible API.
- **`event: error`**: An SSE event type used to report mid-stream errors.

## FAQ

### How is streaming different from the regular endpoint?

The non-streaming endpoint (`POST /api/chat/completions`) returns the full
response as one JSON object after the model finishes. The streaming endpoint
sends text piece by piece, so users see output sooner.

### What happens if the connection drops mid-stream?

The server closes the SSE channel and the client stops receiving events. No
`data: [DONE]` message is sent in this case — its absence tells the client the
stream ended unexpectedly.

### Can I use this in a browser?

Yes. Browsers have built-in `EventSource` and `fetch` APIs that consume SSE
streams. Many JavaScript libraries (e.g. OpenAI's SDK) handle this
automatically.

### Does this work with any provider?

Yes, as long as the provider implements `LlmProvider::chat_completion_stream`.
The bundled `OpenAiProvider` supports streaming for any OpenAI-compatible API
(Ollama, OpenAI, etc.).

### What happens if the provider fails before streaming starts?

The server returns a normal HTTP error response (e.g. `502 Bad Gateway` or
`429 Too Many Requests`) with a JSON body like `{"error":"..."}` — the same
format as the non-streaming endpoint.

## Troubleshooting

- **No `data: [DONE]` message**: The upstream provider closed the connection
  unexpectedly. Check that the model is running and the `LLM_BASE_URL` is
  correct.
- **`event: error` in the stream**: An error occurred while reading from the
  provider. The error data field contains the details. Check server logs for
  more information.
- **`502 Bad Gateway`**: The configured provider is not reachable. Verify
  `LLM_BASE_URL` and ensure the provider is running.
- **`400 Bad Request`**: The JSON body could not be parsed. Check that the
  request body is valid JSON and includes required fields (`model`, `messages`).
- **Stream hangs**: The keep-alive mechanism sends periodic empty comments to
  prevent connection timeouts. If the stream still hangs, check for proxy or
  firewall timeouts.

## Related Resources

- Technical documentation: `docs/streaming-chat-completions-route.md`
- Source handler: `server/src/routes/chat_stream.rs`
- Shared error module: `server/src/routes/error.rs`
- Shared router: `server/src/lib.rs`
- Non-streaming feature wiki: `wiki/chat-completions-route.md`
