# CSS Design System

## Architecture & Design

The LibreChat frontend uses **traditional CSS stylesheets** — no CSS frameworks or preprocessors. All styles live under `frontend/style/` and are copied into the Trunk build output via `<link data-trunk rel="stylesheet">` tags in `frontend/index.html`.

### Why Traditional CSS?

- **Zero runtime cost** — no JavaScript framework overhead for class generation.
- **WASM-friendly** — styles are static assets bundled by Trunk; no build-time JS toolchain required.
- **Low resource footprint** — consistent with the project's Raspberry Pi target.
- **Explicit and auditable** — all styles are in plain `.css` files, easy to grep and review.

### File Layout

```text
frontend/
├── index.html          ← references main.css via Trunk link tag
├── style/
│   └── main.css        ← global stylesheet (design tokens + reset + utilities)
└── src/
    └── lib.rs          ← Leptos App component with class="app-root"
```

Additional stylesheets can be added to `frontend/style/` and referenced in `index.html` as the project grows.

## Design Tokens

All visual constants are defined as CSS custom properties in `:root` inside `main.css`:

| Token | Default | Purpose |
|---|---|---|
| `--color-bg-primary` | `#111827` | Page background (dark) |
| `--color-bg-secondary` | `#1f2937` | Card/panel background |
| `--color-bg-input` | `#374151` | Input field background |
| `--color-text-primary` | `#f9fafb` | Primary text colour |
| `--color-text-secondary` | `#9ca3af` | Muted/secondary text |
| `--color-accent` | `#3b82f6` | Accent/brand colour (blue) |
| `--color-accent-hover` | `#2563eb` | Accent hover state |
| `--color-border` | `#4b5563` | Border colour |
| `--font-sans` | `system-ui, -apple-system, sans-serif` | Body font stack |
| `--font-mono` | `ui-monospace, "Cascadia Code", "Fira Code", monospace` | Code block font stack |
| `--radius-sm` | `0.25rem` | Small border radius |
| `--radius-md` | `0.5rem` | Medium border radius |
| `--radius-lg` | `0.75rem` | Large border radius |

Additional tokens for spacing (`--space-*`), shadows (`--shadow-*`), and transitions (`--transition-*`) are also defined.

## Base Reset

`main.css` includes a minimal reset:

- **Box-sizing**: `border-box` on all elements.
- **Margin/padding**: Zeroed on all elements via `*, *::before, *::after`.
- **HTML/body**: Full height (`100%`), system font stack, dark background, light text, antialiased rendering.

## Layout Utility Classes

| Class | Purpose |
|---|---|
| `.app-root` | Full-height flex column; the root container for the Leptos app |
| `.flex-column-full` | Generic full-height flex column |
| `.scroll-area` | Flex-grow scrollable region (for message history) |
| `.sticky-input` | Flex-shrink pinned bottom bar (for chat input) |
| `.message-list` | Vertical flex list with gaps for chat messages *(planned)* |
| `.message-bubble` | Base message bubble styling *(planned)* |
| `.message-bubble--user` | Right-aligned user message (accent background) *(planned)* |
| `.message-bubble--assistant` | Left-aligned assistant message (secondary background) *(planned)* |
| `.chat-input` | Flex row with textarea and send button *(planned)* |
| `.sr-only` | Screen-reader-only hidden text |

## API Reference

There are no Rust APIs in this issue — the design system is purely CSS. The Leptos `App` component applies the `app-root` class to its root `<div>`.

## Configuration

No environment variables or feature flags are introduced. The dark theme is the default and only theme in this phase. Theme switching will be added in a future issue.

## Testing Guide

Run the design system integration tests:

```bash
cargo test -p server --test css_design_system
```

These tests verify:
- `frontend/style/main.css` exists and contains all required CSS custom properties.
- Reset styles (box-sizing, margin, padding) are present.
- Layout utility classes (`.app-root`, `.scroll-area`, `.sticky-input`, `.flex-column-full`) are defined.
- `frontend/index.html` references `main.css` via a Trunk stylesheet link tag.
- The Leptos `App` component uses the `app-root` class.

## Migration / Upgrade Notes

This is the initial setup — no migration needed. Future UI issues should use the design tokens and utility classes defined here rather than hardcoding values.
