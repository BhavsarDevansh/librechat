//! Shared application state for the Axum server.

/// Application state shared across all request handlers via Axum's
/// [`State`](axum::extract::State) extractor.
///
/// Currently empty but ready to hold shared state such as a `reqwest::Client`
/// or configuration values in future issues.
#[derive(Clone)]
pub struct AppState {}

impl AppState {
    /// Creates a new `AppState` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
