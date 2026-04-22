# Project North Star: Rust-based Open WebUI Alternative

## 🎯 Vision
An entirely open-source, high-performance, and resource-efficient alternative to Open WebUI. Built in pure Rust to ensure minimal memory footprint, maximum speed, and seamless deployment on extremely resource-constrained environments.

## 🚀 Core Goals
- **Extreme Efficiency**: Replace Python/Node.js overhead with Rust's zero-cost abstractions.
- **Ultra-Low Resource Footprint**: Optimized for 10+ year old laptops and Raspberry Pis. The application must coexist with other services (e.g., Home Assistant) without competing for limited RAM/CPU.
- **Unified Language Stack**: Pure Rust from backend to frontend via WASM.
- **Modern Rust Patterns**: Leveraging the latest async ecosystem (Tokio, Axum) and type-safe database interactions (SQLx).

## 🛠 Technical Stack (The "Rust Power Stack")

### Backend
- **Language**: Rust (Latest Stable)
- **Web Framework**: `axum` (High-performance, ergonomic, based on `tower` and `hyper`).
- **Async Runtime**: `tokio` (The industry standard for async I/O).
- **Database**: `sqlx` (Compile-time checked SQL, async, using SQLite for the primary local-first engine).
- **API Integration**: `reqwest` (For interacting with Ollama, OpenAI-compatible APIs).

### Frontend (WASM)
- **Framework**: `leptos` or `dioxus` (Full-stack Rust WASM frameworks to ensure a unified language stack, type safety from end-to-end, and no JS-tooling overhead).
- **Styling**: Tailwind CSS (integrated via Rust tooling).

## 📋 Feature Parity Roadmap (Inspired by Open WebUI)

### Phase 1: Foundation (The Essentials)
- [ ] **Multi-Provider Integration**: Support for Ollama and OpenAI-compatible APIs.
- [ ] **Chat Interface**: Responsive UI with Markdown and LaTeX support.
- [ ] **Session Management**: Persistent chat history and user profiles.
- [ ] **Model Management**: Ability to switch and configure models via UI.
- [ ] **Resource Optimization**: Fine-tuning memory usage and binary size for ARM/x86_64.

### Phase 2: Intelligence & RAG (Deferred)
- [ ] **Local RAG**: Document upload, embedding generation, and retrieval. (Initial versions will use embedded storage; future versions will support Qdrant containers).
- [ ] **Web Search Integration**: Pluggable search providers to inject real-time data.
- [ ] **Web Browsing**: Ability to crawl and process URLs as context.

### Phase 3: Advanced Capabilities
- [ ] **Tool Use / Function Calling**: Rust-native implementation of tool execution.
- [ ] **RBAC**: Granular permissions and user groups for multi-user environments.
- [ ] **Multi-Model Conversations**: Parallel queries to multiple models for comparison.
- [ ] **Voice/Video**: Integration with Whisper (STT) and various TTS engines.

## 📐 Architecture Overview

### Layered Design
1. **API Layer (`axum`)**: Handles HTTP requests, WebSocket connections for streaming LLM responses, and Auth.
2. **Service Layer**: Business logic for chat orchestration, prompt templating, and provider routing.
3. **Data Layer (`sqlx` / SQLite)**: Local-first persistence for users/chats. Designed for low I/O overhead.
4. **Provider Layer**: Abstracted traits for LLM providers, allowing easy addition of new backends without touching core logic.

### Deployment Strategy
- **Single Binary**: Compile to a static binary for effortless deployment on Linux/ARM.
- **Docker**: Extremely minimal Alpine or Distroless images to minimize image size and runtime RAM.
- **Resource Constraints**: Explicitly designed to run alongside other containers on a Raspberry Pi.

## ⚙️ Efficiency Mandate
Given the target hardware (10yr+ laptops, Raspberry Pis), the following rules apply:
- **No "Heavy" Dependencies**: Avoid crates that introduce massive bloat or runtime overhead.
- **Zero-Copy Where Possible**: Prioritize `&str` and `Cow` over `String` and `Vec` in the hot path.
- **Surgical Memory Management**: Avoid unnecessary allocations and prefer efficient data structures.
- **Low-Overhead Async**: Careful use of `tokio` tasks to prevent thread starvation in low-core environments.
