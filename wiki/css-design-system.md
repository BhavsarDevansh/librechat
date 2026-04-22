# CSS Design System

## Feature Overview

LibreChat uses a **traditional CSS design system** — no Tailwind, no CSS-in-JS, no preprocessors. All visual styles are defined in plain `.css` files under `frontend/style/`, and the build tool (Trunk) bundles them into the final WASM output.

The design system gives the chat interface a **dark theme by default**, with a consistent set of colours, fonts, spacing values, and ready-made utility classes. Every UI component built in future issues should use these design tokens instead of hardcoding values.

### Why does this exist?

- A consistent look and feel across the entire app, built once and reused everywhere.
- Zero JavaScript overhead for styling — styles are static assets, perfect for resource-constrained devices (Raspberry Pi, old laptops).
- Easy to audit and modify — just edit a `.css` file, no build step required for style changes.

## User Guide

### For developers: using the design system

1. **Always use CSS custom properties** (e.g., `var(--color-accent)`) instead of raw hex values. This keeps the UI consistent and makes future theming easy.

2. **Use the layout utility classes** for common patterns:

   | What you need | Class to use |
   |---|---|
   | Full-height app shell | `.app-root` |
   | Full-height flex column | `.flex-column-full` |
   | Scrollable message area | `.scroll-area` |
   | Sticky bottom input bar | `.sticky-input` |
   | Visually hidden but accessible text | `.sr-only` |

3. **Adding new styles**: Create new classes in `frontend/style/main.css` (or a new `.css` file added to `index.html`).

4. **Previewing changes**: Run `cd frontend && trunk serve` to see the styled page in a browser.

### For reviewers: what to check

- No Tailwind or CSS framework imports.
- All colours reference `var(--color-*)` tokens.
- New layout patterns reuse existing utility classes before creating new ones.

## Glossary

| Term | Meaning |
|---|---|
| **CSS Custom Property** | A variable in CSS defined with `--name: value;` and used with `var(--name)`. Also called "CSS variable". |
| **Design Token** | A named value (colour, spacing, font) that defines a visual property of the design system. |
| **Trunk** | The build tool that compiles Leptos WASM frontends and bundles static assets (CSS, WASM, JS) into a `dist/` folder. |
| **Reset** | A set of CSS rules that normalise browser defaults (margins, padding, box-sizing) to a consistent baseline. |
| **Dark Theme** | The default colour scheme with dark backgrounds (`#111827`) and light text (`#f9fafb`). |

## FAQ / Troubleshooting

**Q: Styles aren't showing up after I edit `main.css` — what do I do?**

A: Make sure Trunk is running (`trunk serve`). Trunk watches for file changes and rebuilds automatically. If the issue persists, try a hard refresh (Ctrl+Shift+R) to clear the browser cache.

**Q: Can I use Tailwind or another CSS framework?**

A: No — this project intentionally uses traditional CSS stylesheets for minimal overhead and no build-time JS dependencies.

**Q: Where do I add a new colour or spacing value?**

A: Add it as a CSS custom property in `:root` inside `frontend/style/main.css`, then reference it with `var(--your-new-token)` wherever needed.

**Q: How do I add a new stylesheet file?**

A: Create the `.css` file under `frontend/style/`, then add a `<link data-trunk rel="stylesheet" href="style/your-file.css"/>` tag in `frontend/index.html`.

## Related Resources

- [Trunk documentation](https://trunkrs.dev/) — build tool for Leptos WASM apps
- [Leptos documentation](https://leptos.dev/) — Rust full-stack WASM framework
- [Issue #2 on GitHub](https://github.com/BhavsarDevansh/librechat/issues/2) — the original issue for this feature
