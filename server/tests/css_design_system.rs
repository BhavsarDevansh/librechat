//! Integration tests verifying the CSS design system and stylesheet setup (Issue #2).

use std::path::Path;

/// Returns the normalized workspace root directory.
fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read_main_css() -> String {
    let path = workspace_root()
        .join("frontend")
        .join("style")
        .join("main.css");
    assert!(path.exists(), "frontend/style/main.css is missing");
    std::fs::read_to_string(&path).expect("failed to read frontend/style/main.css")
}

fn read_index_html() -> String {
    let path = workspace_root().join("frontend").join("index.html");
    assert!(path.exists(), "frontend/index.html is missing");
    std::fs::read_to_string(&path).expect("failed to read frontend/index.html")
}

fn read_app_lib_rs() -> String {
    let path = workspace_root().join("frontend").join("src").join("lib.rs");
    assert!(path.exists(), "frontend/src/lib.rs is missing");
    std::fs::read_to_string(&path).expect("failed to read frontend/src/lib.rs")
}

// ---- CSS file existence and custom properties ----

#[test]
fn test_main_css_exists() {
    let path = workspace_root()
        .join("frontend")
        .join("style")
        .join("main.css");
    assert!(path.exists(), "frontend/style/main.css must exist");
}

#[test]
fn test_css_custom_properties_defined_in_root() {
    let css = read_main_css();

    let required_vars = [
        "--color-bg-primary",
        "--color-bg-secondary",
        "--color-bg-input",
        "--color-text-primary",
        "--color-text-secondary",
        "--color-accent",
        "--color-accent-hover",
        "--color-border",
        "--font-sans",
        "--font-mono",
        "--radius-sm",
        "--radius-md",
        "--radius-lg",
    ];

    for var in &required_vars {
        assert!(
            css.contains(var),
            "main.css must define CSS custom property `{var}` in :root"
        );
    }
}

#[test]
fn test_css_custom_property_values() {
    let css = read_main_css();

    // Spot-check specific values from the issue requirements
    assert!(
        css.contains("#111827"),
        "main.css must set --color-bg-primary to #111827"
    );
    assert!(
        css.contains("#1f2937"),
        "main.css must set --color-bg-secondary to #1f2937"
    );
    assert!(
        css.contains("#374151"),
        "main.css must set --color-bg-input to #374151"
    );
    assert!(
        css.contains("#3b82f6"),
        "main.css must set --color-accent to #3b82f6"
    );
    assert!(
        css.contains("#2563eb"),
        "main.css must set --color-accent-hover to #2563eb"
    );
    assert!(
        css.contains("#4b5563"),
        "main.css must set --color-border to #4b5563"
    );
}

#[test]
fn test_css_dark_theme_default() {
    let css = read_main_css();

    // Verify dark background is set on body/html
    assert!(
        css.contains("background-color") && css.contains("var(--color-bg-primary)"),
        "Dark theme must be applied as default via background-color referencing --color-bg-primary"
    );

    // Verify light text is the default
    assert!(
        css.contains("color") && css.contains("var(--color-text-primary)"),
        "Dark theme must use --color-text-primary for default text color"
    );
}

// ---- Base reset styles ----

#[test]
fn test_css_reset_box_sizing() {
    let css = read_main_css();
    assert!(
        css.contains("box-sizing") && css.contains("border-box"),
        "Reset must set box-sizing: border-box"
    );
}

#[test]
fn test_css_reset_margin_padding() {
    let css = read_main_css();
    // Check that margin and padding are reset to 0
    assert!(
        css.contains("margin: 0") || css.contains("margin:0"),
        "Reset must set margin to 0"
    );
    assert!(
        css.contains("padding: 0") || css.contains("padding:0"),
        "Reset must set padding to 0"
    );
}

// ---- Layout utility classes ----

#[test]
fn test_css_has_app_root_class() {
    let css = read_main_css();
    assert!(
        css.contains(".app-root"),
        "main.css must define the .app-root class"
    );
}

#[test]
fn test_css_has_scroll_area_class() {
    let css = read_main_css();
    assert!(
        css.contains(".scroll-area") || css.contains(".message-list"),
        "main.css must define a scrollable area utility class for the chat layout"
    );
}

#[test]
fn test_css_has_sticky_input_class() {
    let css = read_main_css();
    assert!(
        css.contains(".sticky-input") || css.contains(".chat-input"),
        "main.css must define a sticky input utility class for the chat layout"
    );
}

#[test]
fn test_css_has_flex_column_full_class() {
    let css = read_main_css();
    assert!(
        css.contains(".flex-column-full") || css.contains("flex-direction: column"),
        "main.css must define a flex-column layout utility"
    );
}

// ---- index.html references stylesheet ----

#[test]
fn test_index_html_references_stylesheet() {
    let html = read_index_html();

    assert!(
        html.contains("data-trunk") && html.contains("stylesheet") && html.contains("main.css"),
        "index.html must reference main.css via a <link data-trunk rel=\"stylesheet\" .../> tag"
    );
}

#[test]
fn test_index_html_still_has_rust_tag() {
    let html = read_index_html();

    assert!(
        html.contains("data-trunk") && html.contains("data-wasm-cargo"),
        "index.html must still contain the Trunk Rust/WASM entrypoint tag"
    );
}

// ---- Leptos component uses app-root class ----

#[test]
fn test_app_component_uses_app_root_class() {
    let lib_rs = read_app_lib_rs();

    assert!(
        lib_rs.contains("app-root"),
        "Leptos App component must apply class=\"app-root\" to its root element"
    );
}
