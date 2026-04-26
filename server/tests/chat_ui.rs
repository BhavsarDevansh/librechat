//! Integration tests verifying the Leptos chat UI components (Issue #10).

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

// ---- ChatMessage struct ----

#[test]
fn test_chat_message_struct_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("struct ChatMessage"),
        "chat.rs must define a ChatMessage struct"
    );
    assert!(
        source.contains("role"),
        "ChatMessage must have a role field"
    );
    assert!(
        source.contains("content"),
        "ChatMessage must have a content field"
    );
}

#[test]
fn test_message_role_enum_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("enum MessageRole"),
        "chat.rs must define a MessageRole enum"
    );
    assert!(
        source.contains("User") && source.contains("Assistant"),
        "MessageRole must have User and Assistant variants"
    );
}

#[test]
fn test_chat_message_derives_clone() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"#\[derive[^\]]*Clone[^\]]*\]")
            .unwrap()
            .is_match(&source)
            && source.contains("struct ChatMessage"),
        "ChatMessage must derive Clone"
    );
}

// ---- ChatView component ----

#[test]
fn test_chat_view_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("#[component]") && source.contains("fn ChatView"),
        "chat.rs must define a ChatView component"
    );
}

// ---- MessageList component ----

#[test]
fn test_message_list_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("#[component]") && source.contains("fn MessageList"),
        "chat.rs must define a MessageList component"
    );
}

#[test]
fn test_message_list_uses_for_each() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("<For") || source.contains("<For "),
        "MessageList must use the <For/> component for rendering message lists"
    );
}

// ---- ChatInput component ----

#[test]
fn test_chat_input_component_defined() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("#[component]") && source.contains("fn ChatInput"),
        "chat.rs must define a ChatInput component"
    );
}

#[test]
fn test_chat_input_has_textarea() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("textarea"),
        "ChatInput must contain a textarea element"
    );
}

#[test]
fn test_chat_input_has_send_button() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r#"<button[^>]*class="[^"]*\bsend-btn\b[^"]*"[^>]*>"#)
            .unwrap()
            .is_match(&source),
        "ChatInput must contain a button with class 'send-btn'"
    );
}

#[test]
fn test_chat_input_handles_enter_key() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("keydown") || source.contains("keypress"),
        "ChatInput must handle keyboard events (Enter/Shift+Enter)"
    );
}

#[test]
fn test_chat_input_send_button_disabled_when_empty() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("disabled"),
        "ChatInput send button must have a disabled attribute"
    );
}

// ---- Signal for conversation history ----

#[test]
fn test_chat_view_uses_signal_vec() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("signal("),
        "ChatView must use a signal for managing conversation history"
    );
}

#[test]
fn test_chat_view_passes_messages_to_message_list() {
    let source = read_file("frontend/src/components/chat.rs");
    // ChatView should render MessageList and pass messages to it
    assert!(
        source.contains("MessageList"),
        "ChatView must reference MessageList component"
    );
    assert!(
        source.contains("ChatInput"),
        "ChatView must reference ChatInput component"
    );
}

// ---- App integration ----

#[test]
fn test_app_uses_chat_view() {
    let source = read_file("frontend/src/lib.rs");
    assert!(
        source.contains("ChatView"),
        "App component must render ChatView"
    );
}

#[test]
fn test_lib_rs_imports_components() {
    let source = read_file("frontend/src/lib.rs");
    assert!(
        source.contains("components") || source.contains("chat"),
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

// ---- Auto-scroll behaviour ----

#[test]
fn test_message_list_has_auto_scroll() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("NodeRef") && source.contains("scroll_to"),
        "MessageList must implement auto-scroll using NodeRef and scroll_to"
    );
}

#[test]
fn test_message_list_uses_effect_for_scroll() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("Effect::new"),
        "MessageList must use Effect to trigger auto-scroll on message changes"
    );
}

#[test]
fn test_chat_input_clears_after_send() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("String::new()"),
        "ChatInput must clear the input text after sending a message"
    );
}

#[test]
fn test_chat_input_shift_enter_allows_newline() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        source.contains("shift_key"),
        "ChatInput must check shift_key to allow Shift+Enter for newlines"
    );
}
