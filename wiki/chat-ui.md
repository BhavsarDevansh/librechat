# Chat UI — Feature Guide

## Overview

The chat interface is the main way you interact with LibreChat. It provides a simple, responsive message window where you type prompts and receive responses — just like any modern chat app.

Currently, the assistant echoes back what you type (prefixed with "Echo:"). This is a placeholder while the real AI backend connection is being built.

## How to Use

1. **Type a message** in the text box at the bottom of the screen.
2. Press **Enter** to send, or click the **Send** button.
3. Need a new line? Press **Shift+Enter**.
4. Messages appear as coloured bubbles:
   - **Your messages** (blue, right-aligned)
   - **Assistant messages** (dark grey, left-aligned)
5. The conversation automatically scrolls to the latest message.

## Visual Layout

```text
┌─────────────────────────────────┐
│                                 │
│  [Assistant message]            │  ← left-aligned
│                                 │
│            [Your message]       │  ← right-aligned
│                                 │
│  [Assistant message]            │
│                                 │
├─────────────────────────────────┤
│ [Type a message…        ] [Send]│  ← input area
└─────────────────────────────────┘
```

## Glossary

| Term           | Meaning                                                        |
| -------------- | -------------------------------------------------------------- |
| **CSR**        | Client-Side Rendering — the UI is built entirely in the browser |
| **Signal**     | A Leptos reactive value — when it changes, the UI updates automatically |
| **Component**  | A self-contained piece of UI (like a button or message list)   |
| **WASM**        | WebAssembly — how Rust code runs directly in the browser        |

## FAQ / Troubleshooting

**The UI doesn't appear after building.**
Make sure you ran `trunk build` (or `trunk serve`) from the `frontend/` directory. The WASM binary needs to be compiled and served alongside `index.html`.

**Messages don't scroll to the bottom.**
The auto-scroll relies on a browser API (`scrollTo`). If you're testing in a minimal headless browser, this may not be supported.

**The Send button is always greyed out.**
The button is disabled when the input text is empty. Type at least one character to enable it.

## Related Resources

- Technical documentation: [`docs/chat-ui.md`](../docs/chat-ui.md)
- CSS design system: [`docs/css-design-system.md`](../docs/css-design-system.md)
- Backend chat completions: [`docs/chat-completions-route.md`](../docs/chat-completions-route.md)
