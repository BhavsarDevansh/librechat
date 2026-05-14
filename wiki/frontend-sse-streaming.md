# Frontend SSE Streaming — Real-Time Token Display

## Feature Overview

Instead of waiting for the AI to finish thinking before showing its response,
LibreChat now displays tokens as they arrive — character by character — using
a technology called **Server-Sent Events (SSE)**.

In everyday terms: when you send a message, the assistant's reply starts
appearing immediately, word by word, rather than popping in all at once after a
long pause. This makes conversations feel faster and more natural, especially
for longer answers.

## How to Use

1. **Open LibreChat** in your browser (e.g. `http://localhost:3000`).
2. **Type a message** and press **Enter** or click **Send**.
3. An empty assistant bubble appears right away.
4. **Watch the text appear in real time** as the AI generates each piece.
5. When the AI finishes, the stream stops and the bubble is complete.
6. If something goes wrong during the stream, an error message appears in a
   red bubble.

## What Changed

| Before (non-streaming) | After (SSE streaming) |
|------------------------|-----------------------|
| "Thinking…" shown for the entire wait | Tokens appear immediately, replacing the wait |
| Full response appears all at once | Response builds character-by-character |
| Longer answers feel slower | Longer answers feel faster because you can read as they arrive |

## Visual Layout

```text
┌─────────────────────────────────┐
│                                 │
│  [Your message]                 │
│                                 │
│  [Assistant reply building…]    │  ← text appears here live
│                                 │
├─────────────────────────────────┤
│ [Type a message…        ] [Send]│  ← disabled while streaming
└─────────────────────────────────┘
```

## Glossary

- **SSE (Server-Sent Events)**: A browser technology that lets a web page
  receive a continuous stream of small messages from a server. Think of it like
  a one-way chat where the server sends messages and the browser reads them.
- **Token**: A small piece of text (often a word or part of a word) that the AI
  generates one at a time.
- **Stream**: The continuous flow of tokens from the AI to your screen.
- **`data: [DONE]`**: A special signal sent by the server to tell the browser
  that the AI has finished generating text.
- **Chunk**: A single packet of data in the stream, usually containing one or
  more tokens.

## FAQ

### Why does the text appear character by character?

The AI generates text one small piece at a time. With SSE streaming, each piece
is sent to your browser as soon as it is created, so you see it immediately.

### What happens if my internet drops while streaming?

The stream stops. The assistant bubble will show whatever text had arrived so
far. You can send a follow-up message to continue the conversation.

### Does streaming work with all AI models?

Yes, as long as the backend provider supports streaming. The built-in
OpenAI-compatible provider (used for Ollama, OpenAI, and similar services)
supports it.

### Is streaming slower than the old way?

No — the total time to generate a response is the same. Streaming just shows
you the text as it is created, so it *feels* faster because you are not
staring at a blank "Thinking…" indicator.

### Why is the Send button disabled during streaming?

To prevent you from sending another message while the current one is still
being generated. The button becomes active again once the stream finishes.

## Troubleshooting

**The assistant bubble stays empty.**
- The stream may have ended with no content. The server sends a fallback error
  message in this case. Check that your LLM provider is running and the model
  is loaded.

**I see a red error bubble during streaming.**
- A network or server error occurred mid-stream. Check that the backend is
  running (`cargo run`) and the LLM provider is reachable.

**The stream seems to stop early.**
- The connection may have been interrupted. Check your network and try again.
  If it happens consistently, check the server logs for provider errors.

**Text appears garbled or with strange characters.**
- This is rare, but can happen if the stream receives invalid data. The SSE
  parser is designed to handle this gracefully. If it persists, file a bug
  report.

## Related Resources

- Technical documentation: [`docs/frontend-sse-streaming.md`](../docs/frontend-sse-streaming.md)
- Backend SSE route: [`wiki/streaming-chat-completions-route.md`](streaming-chat-completions-route.md)
- Frontend API integration: [`wiki/frontend-api-integration.md`](frontend-api-integration.md)
- Chat UI design: [`wiki/chat-ui.md`](chat-ui.md)
