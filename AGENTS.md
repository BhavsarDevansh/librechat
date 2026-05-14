# Repository Guidelines

## Documentation Lookup
- Use `npx ctx7@latest` to fetch current documentation whenever a task asks about a library, framework, SDK, API, CLI tool, or cloud service. Run `library` first unless the user already gives a `/org/project` Context7 ID, then run `docs` with the selected ID.
- Use the full user question in Context7 queries, avoid secrets in queries, and do not run more than 3 Context7 commands for one question.
- If Context7 fails with quota limits, tell the user to run `npx ctx7@latest login` or set `CONTEXT7_API_KEY`. If it fails due to DNS/network sandboxing, rerun it outside the default sandbox.
- Do not use Context7 for pure refactors, business-logic debugging, code review, or general programming concepts unless a library-specific question is involved.

## Project Structure
- `Cargo.toml`: Workspace manifest. Members are `server` and `frontend`; shared dependency versions live in `[workspace.dependencies]`.
- `server/`: Axum/Tokio backend crate.
- `server/src/main.rs`: Binary entry point, tracing setup, port binding, and app startup.
- `server/src/lib.rs`: Router construction, static file fallback, and shared server helpers.
- `server/src/routes/`: HTTP handlers for chat, streaming chat, models, health, and route errors.
- `server/src/providers/`: OpenAI-compatible provider traits, request/response types, and provider implementation.
- `server/src/state.rs`: Application state, provider selection, static asset directory resolution, and environment-backed configuration.
- `server/tests/`: Integration and structure tests for routes, providers, streaming, static files, frontend integration, and workspace invariants.
- `frontend/`: Leptos client-side WASM crate built by Trunk.
- `frontend/src/`: UI components, frontend state, HTTP API client, and SSE client.
- `frontend/style/main.css`: Design system and app styles.
- `frontend/index.html`: Trunk entry point, including the `data-wasm-cargo="frontend"` WASM target.
- `frontend/dist/`: Generated static assets served by the backend. Treat as build output; update only when frontend output intentionally changes.

## Build, Run, And Test
- `cargo run -p server`: Run the backend from the workspace root. By default it serves static assets from `frontend/dist`.
- `LIBRECHAT_PORT=3001 cargo run -p server`: Run the backend on a non-default port.
- `LIBRECHAT_STATIC_DIR=/absolute/path/to/dist cargo run -p server`: Serve a custom static asset directory.
- `cargo test --workspace`: Run all Rust tests.
- `cargo test -p server`: Run backend and integration tests.
- `cargo test -p server --test workspace_structure`: Run workspace invariant tests quickly when changing manifests or layout.
- `cargo fmt --all`: Format the workspace.
- `cargo clippy --workspace --all-targets`: Lint all workspace targets.
- `cargo build -p server`: Build the backend.
- `cargo build -p frontend --target wasm32-unknown-unknown`: Compile the frontend crate to WASM.
- `cd frontend && trunk build`: Produce frontend static assets in `frontend/dist`.
- `cd frontend && trunk build --release`: Produce optimized frontend assets for release checks.

## Toolchain And Frontend Notes
- The workspace uses stable Rust with the `wasm32-unknown-unknown` target, configured in `rust-toolchain.toml`.
- Keep the root workspace manifest as the source of truth for shared dependency versions.
- Do not replace Trunk metadata in `frontend/index.html`; tests expect the `data-trunk` stylesheet and `data-wasm-cargo="frontend"` Rust entry.
- When frontend behavior or CSS changes, build with Trunk and verify whether `frontend/dist` needs to be committed.

## Coding Style
- Follow standard Rust formatting and idioms. Prefer simple ownership and borrowing over unnecessary cloning.
- Keep async code on Tokio in the backend. Avoid blocking work in request handlers.
- Keep route handlers thin; put reusable provider, state, parsing, and error behavior behind focused modules.
- Prefer typed errors and explicit status mapping over stringly typed error handling.
- Preserve API compatibility for OpenAI-compatible chat and streaming endpoints unless the user explicitly requests a breaking change.
- For frontend code, keep component state and API/SSE concerns separated. Reuse existing Leptos patterns in `frontend/src` before introducing new abstractions.
- Add comments only where they explain non-obvious behavior, invariants, or protocol details.

## Dependency Policy
- This project targets Raspberry Pi and legacy hardware. Avoid heavy dependencies unless the runtime, binary-size, and maintenance costs are justified.
- Prefer existing workspace dependencies before adding a new crate.
- If a new crate is necessary, add it at the narrowest scope possible and explain why a lighter alternative is not sufficient.
- Keep feature flags minimal, especially on networking, TLS, tracing, and frontend/WASM dependencies.

## Testing Guidelines
- Add or update tests with behavioral changes. Provider logic, HTTP routes, streaming behavior, static serving, and workspace layout changes should be covered by integration tests in `server/tests`.
- Use deterministic tests with local mock servers or temporary directories; do not require real provider credentials or external network calls.
- Protect environment-variable tests with synchronization when they mutate process-wide state.
- For frontend-facing changes, cover stable contracts through Rust tests where practical and run a Trunk build for asset integration.

## Quality Gate
- Before claiming completion, run the narrowest useful verification first, then broader checks when the change warrants it.
- For a committable unit of work, the expected final checks are `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, and `cd frontend && trunk build --release` when frontend or static assets are affected.
- Perform a code review pass before finalizing. Action real findings; do not manufacture trivial changes just to satisfy the process.
- Update crate versions according to SemVer only for release-intended functional changes. Do not bump versions for documentation-only or internal test-only edits unless explicitly requested.

## Git And PR Expectations
- Use clear, imperative commit messages such as `feat: add streaming retries` or `fix: handle provider timeout`.
- Do not create branch names beginning with `codex`; use descriptive names such as `feat/ui-redesign` or `fix/static-serving`.
- Never co-sign or co-author commits.
- PRs should include a concise change summary, linked issue when applicable, verification performed, and screenshots for UI changes.
- Do not revert user changes or unrelated work. If local changes conflict with the requested task, stop and ask how to proceed.
