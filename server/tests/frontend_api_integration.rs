//! Structural tests for the Leptos frontend API integration (Issue #11).
//!
//! These tests verify that the frontend source files contain the expected
//! module structure, type definitions, and component logic for connecting
//! the chat UI to the non-streaming backend API.

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Extracts the body of a top-level function named `fn_name` from `source`.
///
/// Locates `pub fn <fn_name>` (or `fn <fn_name>`), then counts braces to
/// find the matching closing `}` and returns the text between the braces.
/// Returns `None` if the function cannot be found or braces are unbalanced.
///
/// **Limitation:** This function counts raw braces and does not skip braces
/// inside string literals (`"..."`), char literals (`...`), line comments
/// (`// ...`), or block comments (`/* ... */`). Bodies containing such
/// constructs may be parsed incorrectly. For more robust extraction, use a
/// proper parser such as `syn`.
fn extract_function_body(source: &str, fn_name: &str) -> Option<String> {
    let pattern = format!(r"(?m)^(?:pub\s+)?fn\s+{}\s*[\(<]", regex::escape(fn_name));
    let re = Regex::new(&pattern).ok()?;
    let mat = re.find(source)?;
    let start = mat.start();

    let after_sig = &source[start..];
    let brace_pos = after_sig.find('{')?;
    let body_start = start + brace_pos + 1;

    let mut depth = 1i32;
    let mut pos = body_start;
    let bytes = source.as_bytes();
    while pos < bytes.len() && depth > 0 {
        match bytes[pos] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        pos += 1;
    }

    if depth == 0 {
        Some(source[body_start..pos - 1].to_string())
    } else {
        None
    }
}

/// Returns the normalized workspace root directory.
fn workspace_root() -> &'static Path {
    static WORKSPACE_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut dir = Path::new(manifest_dir);
        loop {
            let candidate = dir.join("Cargo.toml");
            if candidate.exists() {
                if let Ok(contents) = std::fs::read_to_string(&candidate) {
                    if contents.contains("[workspace]") {
                        return dir.to_path_buf();
                    }
                }
            }
            dir = dir.parent().unwrap_or_else(|| {
                panic!("could not find workspace Cargo.toml above {}", manifest_dir)
            });
        }
    });
    &WORKSPACE_ROOT
}

fn read_file(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    assert!(path.exists(), "{relative_path} is missing");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {relative_path}: {e}"))
}

// ---- API module existence ----

#[test]
fn test_api_module_file_exists() {
    let path = workspace_root().join("frontend").join("src").join("api.rs");
    assert!(path.exists(), "frontend/src/api.rs must exist");
}

#[test]
fn test_lib_rs_declares_api_module() {
    let source = read_file("frontend/src/lib.rs");
    assert!(
        Regex::new(r"mod\s+api").unwrap().is_match(&source),
        "lib.rs must declare `mod api`"
    );
}

// ---- API type definitions ----

#[test]
fn test_api_defines_send_chat_request_function() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"pub\s+async\s+fn\s+send_chat_request")
            .unwrap()
            .is_match(&source),
        "api.rs must define a public async function `send_chat_request`"
    );
}

#[test]
fn test_api_defines_api_message_role_enum() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"pub\s+enum\s+ApiMessageRole")
            .unwrap()
            .is_match(&source),
        "api.rs must define `pub enum ApiMessageRole`"
    );
}

#[test]
fn test_api_defines_chat_completion_request_struct() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"pub\s+struct\s+ApiChatCompletionRequest")
            .unwrap()
            .is_match(&source),
        "api.rs must define `pub struct ApiChatCompletionRequest`"
    );
}

#[test]
fn test_api_defines_chat_completion_response_struct() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"pub\s+struct\s+ApiChatCompletionResponse")
            .unwrap()
            .is_match(&source),
        "api.rs must define `pub struct ApiChatCompletionResponse`"
    );
}

#[test]
fn test_api_defines_api_error_enum() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"pub\s+enum\s+ApiError")
            .unwrap()
            .is_match(&source),
        "api.rs must define `pub enum ApiError`"
    );
}

#[test]
fn test_api_defines_default_model_constant() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        Regex::new(r"DEFAULT_MODEL").unwrap().is_match(&source),
        "api.rs must define a DEFAULT_MODEL constant"
    );
}

#[test]
fn test_api_uses_post_method() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        source.contains("POST") || source.contains("post"),
        "api.rs must use POST HTTP method for chat requests"
    );
}

#[test]
fn test_api_references_chat_completions_endpoint() {
    let source = read_file("frontend/src/api.rs");
    assert!(
        source.contains("/api/chat/completions"),
        "api.rs must reference the /api/chat/completions endpoint"
    );
}

// ---- ChatMessage changes ----

#[test]
fn test_chat_message_has_is_error_field() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"is_error\s*:\s*bool")
            .unwrap()
            .is_match(&source),
        "ChatMessage must have an `is_error: bool` field"
    );
}

// ---- ChatView loading state ----

#[test]
fn test_chat_view_uses_loading_signal() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"let\s*\(.*\bloading\b.*\).*signal\s*\(\s*false\s*\)")
            .unwrap()
            .is_match(
                &extract_function_body(&source, "ChatView")
                    .expect("ChatView function must exist in chat.rs"),
            ),
        "ChatView must declare `let (loading, …) = signal(false)`"
    );
}

#[test]
fn test_chat_view_calls_send_chat_request() {
    let source = read_file("frontend/src/components/chat.rs");
    let body = extract_function_body(&source, "ChatView")
        .expect("ChatView function must exist in chat.rs");
    assert!(
        body.contains("stream_chat_request(") || body.contains("api::stream_chat_request("),
        "ChatView body must call stream_chat_request() from the api module"
    );
}

#[test]
fn test_chat_view_displays_thinking_indicator() {
    let source = read_file("frontend/src/components/chat.rs");
    let body = extract_function_body(&source, "MessageList")
        .expect("MessageList function must exist in chat.rs");
    assert!(
        body.contains("\"Thinking"),
        "MessageList body must contain the 'Thinking' text indicator"
    );
}

#[test]
fn test_chat_input_accepts_disabled_prop() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"fn\s+ChatInput\s*\((?:[^)]|\([^)]*\))*\bdisabled\b")
            .unwrap()
            .is_match(&source),
        "ChatInput must accept a `disabled` prop"
    );
}

#[test]
fn test_chat_input_disables_textarea_when_loading() {
    let source = read_file("frontend/src/components/chat.rs");
    assert!(
        Regex::new(r"<textarea[^>]*\bdisabled\s*=")
            .unwrap()
            .is_match(&source),
        "ChatInput textarea must bind the disabled attribute"
    );
}

// ---- CSS for error and loading states ----

#[test]
fn test_css_has_message_error_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-error\s*\{").unwrap().is_match(&css),
        "main.css must define .message-error class"
    );
}

#[test]
fn test_css_has_loading_indicator_class() {
    let css = read_file("frontend/style/main.css");
    assert!(
        Regex::new(r"\.message-loading\s*\{")
            .unwrap()
            .is_match(&css),
        "main.css must define .message-loading class"
    );
}

#[test]
fn test_frontend_cargo_toml_has_gloo_net() {
    let source = read_file("frontend/Cargo.toml");
    assert!(
        source.contains("gloo-net"),
        "frontend/Cargo.toml must include gloo-net dependency"
    );
}

#[test]
fn test_frontend_cargo_toml_has_serde() {
    let source = read_file("frontend/Cargo.toml");
    assert!(
        source.contains("serde"),
        "frontend/Cargo.toml must include serde dependency"
    );
}

#[test]
fn test_frontend_cargo_toml_has_js_sys() {
    let source = read_file("frontend/Cargo.toml");
    assert!(
        source.contains("js-sys"),
        "frontend/Cargo.toml must include js-sys dependency for JS interop"
    );
}
