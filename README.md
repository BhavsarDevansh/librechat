# LibreChat (Rust Edition)

A modular Rust-based LLM orchestration server designed for high efficiency and low resource consumption, targeting Raspberry Pi and legacy hardware.

## Architecture

The project follows a modular architecture:
- `src/api/`: Axum route handlers and middleware.
- `src/services/`: Business logic for LLM orchestration and prompt handling.
- `src/data/`: Database access layers (SQLx) and state management.
- `src/providers/`: Traits and implementations for LLM backends (Ollama, OpenAI).

## Development

### Build and Run
```bash
cargo run
```

### Testing
```bash
cargo test
```

### Linting and Formatting
```bash
cargo clippy
cargo fmt
```

## Resource Constraints
This project is optimized for low-resource environments. Avoid adding heavy dependencies that significantly impact binary size or runtime memory overhead.
