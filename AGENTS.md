# Repository Guidelines

## Project Structure & Module Organization
The project follows a modular Rust architecture optimized for a single-binary distribution:
- `src/`: Main source code.
    - `main.rs`: Application entry point and server initialization.
    - `api/`: Axum route handlers and middleware.
    - `services/`: Business logic for LLM orchestration and prompt handling.
    - `data/`: Database access layers (SQLx) and state management.
    - `providers/`: Traits and implementations for LLM backends (Ollama, OpenAI).
- `tests/`: Integration tests.
- `assets/`: Static assets for the WASM frontend.

## Build, Test, and Development Commands
- `cargo run`: Build and run the backend server locally.
- `cargo test`: Execute all unit and integration tests.
- `cargo build --release`: Generate an optimized binary for resource-constrained hardware.
- `cargo clippy`: Run the Rust linter to ensure code quality.
- `cargo fmt`: Format the codebase according to standard Rust style.

## Coding Style & Naming Conventions
- **Standard Style**: Adhere to the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/).
- **Naming**: Use `snake_case` for functions, variables, and modules; `PascalCase` for structs and enums.
- **Efficiency**: Prioritize zero-copy patterns (`&str`, `Cow`) over frequent allocations to maintain the low-resource mandate.
- **Async**: Use `tokio` for all asynchronous operations.

## Testing Guidelines
- **Framework**: Use the built-in `cargo test` framework.
- **Naming**: Test functions should be descriptive (e.g., `test_ollama_provider_streaming`).
- **Coverage**: Core provider logic and API routes must have accompanying integration tests in the `tests/` directory.

## Commit & Pull Request Guidelines
- **Commits**: Use clear, imperative commit messages (e.g., `feat: add Ollama streaming support`, `fix: resolve SQLite locking issue`).
- **PRs**: All Pull Requests must:
    - Include a clear description of changes.
    - Link to a corresponding issue.
    - Pass all `cargo clippy` and `cargo test` checks.
    - Include screenshots for any UI changes.
- **Authorship**: NEVER co-sign or co-author commits.

## Development Lifecycle & Quality Assurance
A "unit of work" is defined as any committable amount of work. For every unit of work, the following must be performed:
1. **Up-to-Date Standards**: Always use Context7 to verify the most recent patterns, security guidelines, performance best practices, and library versions.
2. **Code Review**: Perform a comprehensive code review upon completion. Every recommendation from the review must be actioned, regardless of how trivial.
3. **Versioning**: Update the project version number according to Semantic Versioning (SemVer) rules immediately following the completion of the unit of work.

## Resource Constraints Note
Since this project targets Raspberry Pi and legacy hardware, avoid adding "heavy" dependencies. Every new crate must be evaluated for its impact on binary size and runtime memory overhead.

## Branch Naming
- Do NOT name branches starting with 'codex'. Use descriptive names like 'feat/ui-redesign' or 'fix/streaming-bug'.
