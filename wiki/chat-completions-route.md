# Chat Completions API

## Feature Overview

LibreChat now exposes a server endpoint at `POST /api/chat/completions` for
sending chat messages to the configured language model and getting back one
complete answer.

In plain terms, the frontend sends a JSON request to the server, the server
passes that request to the configured provider, and the provider's full reply
comes back as JSON.

This helps because the browser only needs to talk to LibreChat's backend. The
backend handles provider access, error translation, and logging.

## User Guide

Send a request like this:

```bash
curl http://localhost:3000/api/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "llama3",
    "messages": [
      { "role": "user", "content": "Say hello" }
    ],
    "temperature": 0.2,
    "stream": false
  }'
```

On success, the server returns JSON that includes:

- The response id
- The model used
- The generated assistant message
- Token usage details

Example response shape:

```json
{
  "id": "chatcmpl-123",
  "model": "llama3",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 3,
    "completion_tokens": 1,
    "total_tokens": 4
  }
}
```

If something goes wrong, the server returns a JSON error:

```json
{"error":"..."}
```

## Glossary

- `LLM`: Large language model.
- `Provider`: The backend service that generates completions, such as Ollama or
  an OpenAI-compatible API.
- `Completion`: A generated assistant reply.
- `JSON`: A text format used for structured request and response data.

## FAQ

### Why is this route non-streaming?

This issue only adds the full-response endpoint. Streaming is handled by a
separate issue and route.

### What happens if the provider is down?

The server returns `502 Bad Gateway` with a JSON error message.

### What happens if I send broken JSON?

The server returns `400 Bad Request` with a JSON error message explaining that
the request body could not be parsed.

### Does the route change my request before sending it upstream?

No. For this issue, the server forwards the received `ChatCompletionRequest`
shape directly to the configured provider.

## Troubleshooting

- If you get `400 Bad Request`, validate the JSON body and make sure the
  `Content-Type` header is `application/json`.
- If you get `502 Bad Gateway`, check that the configured upstream provider is
  running and reachable from the server.
- If you get `500 Internal Server Error`, inspect the server logs for the
  provider error that was mapped internally.

## Related Resources

- Technical documentation: `docs/chat-completions-route.md`
- Source handler: `server/src/routes/chat.rs`
- Shared router: `server/src/lib.rs`
