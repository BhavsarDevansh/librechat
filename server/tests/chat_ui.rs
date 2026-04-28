//! Structural and CSS integration tests for the Leptos chat UI (Issue #10).
//!
//! Behavioral tests live in frontend/tests/chat_ui.rs and exercise compiled
//! components in a browser environment via wasm-bindgen-test.
//!
//! This file verifies file existence, component definitions, and CSS rules —
//! concerns that are best checked with static source analysis rather than
//! DOM queries.

use regex::Regex;
use std::path::Path;

/// Returns the normalized workspace root directory.
fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read_file(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    assert!(path.exists(), "{relative_path} is missing");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {relative_path}: {e}"))
}

// ---- Component file existence ----

#[test]
fn test_chat_component_file_exists() {
    let path = workspace_root()
        .join("frontend")
        .join("src")
        .join("components")
        .join("chat.rs");
    assert!(path.exists(), "frontend/src/components/chat.rs must exist");
}

#[test]
fn test_components_module_file_exists() {
    let path = workspace_root()
        .join("frontend")
        .join("src")
        .join("components")
        .join("mod.rs");
    assert!(path.exists(), "frontend/src/components/mod.rs must exist");
}

// ---- Component definitions (structural) ----

#[test]
fn test_chat_view_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"#\[component\]\s*(?:pub\s+)?fn\s+ChatView\b\s*\(")
            .unwrap()
            .is_match(&source),
        "chat.rs must define a ChatView component"
    );
}

#[test]
fn test_message_list_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"#\[component\]\s*(?:pub\s+)?fn\s+MessageList\b\s*\(")
            .unwrap()
            .is_match(&source),
        "chat.rs must define a MessageList component"
    );
}

#[test]
fn test_chat_input_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"#\[component\]\s*(?:pub\s+)?fn\s+ChatInput\b\s*\(")
            .unwrap()
            .is_match(&source),
        "chat.rs must define a ChatInput component"
    );
}

// ---- App integration ----

#[test]
fn test_app_uses_chat_view() {
    let source = read_file("frontend/src/lib.rs");
    assert!(
        Regex::new(r"<ChatView\b").unwrap().is_match(&source),
        "App component must render ChatView"
    );
}

#[test]
fn test_lib_rs_imports_components() {
    let source = read_file("frontend/src/lib.rs");
    assert!(
        Regex::new(r"(?:pub\s+)?mod\s+components\b|use\s+(?:crate::)?components\b")
            .unwrap()
            .is_match(&source),
        "lib.rs must import the components module"
    );
}

// ---- CSS design system integration ----

#[test]
fn test_css_has_message_bubble_classes() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-bubble\s*\{").unwrap().is_match(&css),
        "main.css must define .message-bubble class"
    );
}

#[test]
fn test_css_has_message_user_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-user\s*\{").unwrap().is_match(&css),
        "main.css must define .message-user class"
    );
}

#[test]
fn test_css_has_message_assistant_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-assistant\s*\{")
            .unwrap()
            .is_match(&css),
        "main.css must define .message-assistant class"
    );
}

#[test]
fn test_css_has_chat_input_area_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.chat-input-area\s*\{")
            .unwrap()
            .is_match(&css),
        "main.css must define .chat-input-area class"
    );
}

#[test]
fn test_css_has_send_btn_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.send-btn\s*\{").unwrap().is_match(&css),
        "main.css must define .send-btn class"
    );
}

#[test]
fn test_css_message_user_aligned_right() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-user\s*\{[^}]*align-self\s*:\s*flex-end")
            .unwrap()
            .is_match(&css),
        ".message-user must be right-aligned (align-self: flex-end)"
    );
}

#[test]
fn test_css_message_assistant_aligned_left() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-assistant\s*\{[^}]*align-self\s*:\s*flex-start")
            .unwrap()
            .is_match(&css),
        ".message-assistant must be left-aligned (align-self: flex-start)"
    );
}

#[test]
fn test_css_send_btn_disabled_state() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.send-btn\s*:\s*disabled\s*\{[^}]*opacity")
            .unwrap()
            .is_match(&css),
        ".send-btn must have a :disabled style with reduced opacity"
    );
}

#[test]
fn test_css_message_list_responsive() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-list\s*\{").unwrap().is_match(&css),
        "main.css must define .message-list class"
    );
}
