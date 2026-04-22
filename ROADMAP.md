# Project Roadmap: Rust-based Open WebUI Alternative

## 🚩 Phase 1: Foundation & Basic Connectivity
*The goal of this phase is to establish a working "Hello World" of the application: a user can send a prompt and receive a response from a local LLM.*

- [ ] **Project Scaffolding**: Establish the Cargo workspace, dependency management, and build pipeline for the native Axum server and Leptos CSR WASM frontend.
- [ ] **Core Web Server**: Implement a minimal Axum web server with health check endpoint, CORS, and static file serving for the WASM frontend.
- [ ] **LLM Provider Trait & OpenAI-Compatible Client**: Define a generic `LlmProvider` trait and implement a concrete client targeting the OpenAI Chat Completions API (`/v1/chat/completions`), which is compatible with both Ollama and OpenAI.
- [ ] **Chat Completions API Route**: Add an Axum route that accepts chat messages from the frontend, forwards them to the provider, and returns the response.
- [ ] **Basic Chat UI**: Build a Leptos CSR chat interface with a message list, text input, and send button that communicates with the backend.
- [ ] **Live Streaming**: Enable real-time SSE streaming of LLM responses from the Axum server to the Leptos frontend so users see tokens as they arrive.

## 🚩 Phase 2: State & Persistence
*The goal of this phase is to transition from a stateless demo to a usable tool that remembers who the user is and what they've talked about.*

- [ ] **Local Database Integration (SQLite via sqlx)**: Set up SQLite with sqlx, including migrations, connection pooling, and compile-time checked queries for all persistence needs.
- [ ] **Chat History**: Allow users to save, load, and delete past conversations.
- [ ] **User Profiles & Settings**: Create a way for users to save their preferences and API configurations.
- [ ] **Model Management**: Implement a UI to browse, select, and configure available models from the connected providers.

## 🚩 Phase 3: User Experience & Polishing
*The goal of this phase is to make the application feel like a professional product, focusing on readability, accessibility, and resource efficiency.*

- [ ] **Rich Text Rendering**: Implement full support for Markdown and LaTeX to make technical responses readable.
- [ ] **Responsive Design Refinement**: Ensure the UI works perfectly across mobile, tablet, and desktop environments.
- [ ] **Resource Optimization**: Audit memory and CPU usage to ensure the application remains lean on Raspberry Pi and legacy hardware.
- [ ] **Authentication & Security**: Add basic user authentication to protect the interface in multi-user environments.

## 🚩 Phase 4: Advanced Intelligence (The "Power User" Suite)
*The goal of this phase is to implement the advanced features that make the application a powerful AI platform.*

- [ ] **Local RAG Implementation**: Enable the ability to upload documents and reference them in chats.
- [ ] **Web Integration**: Implement capabilities for the AI to search the web or browse specific URLs for context.
- [ ] **Tool Execution**: Create a framework for the AI to call and execute local functions/tools.
- [ ] **Multi-Model Orchestration**: Allow users to query multiple models simultaneously for comparison.

## 🚩 Phase 5: Ecosystem & Deployment
*The goal of this phase is to make the application easy to install and maintain for the end-user.*

- [ ] **Single-Binary Distribution**: Optimize the build process to produce a single, static executable with embedded frontend assets.
- [ ] **Minimal Containerization**: Create ultra-small Docker images for easy deployment.
- [ ] **Deployment Guides**: Provide clear instructions for installing on Raspberry Pi and other constrained environments.
