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

    // Extract the :root block content
    let root_re = regex::Regex::new(r":root\s*\{([^}]+)\}").expect("invalid regex for :root block");
    let root_block = root_re
        .captures(&css)
        .expect("main.css must contain a :root block")
        .get(1)
        .expect("failed to extract :root block content")
        .as_str();

    for var in &required_vars {
        // Verify each variable is declared inside :root as `--<name>:`
        // Lines in the :root block are indented with leading whitespace, so we
        // match the variable name followed by optional whitespace and a colon.
        let pattern = format!(r"{var}\s*:");
        assert!(
            regex::Regex::new(&pattern)
                .expect("invalid regex pattern")
                .is_match(root_block),
            "main.css must define CSS custom property `{var}` inside :root"
        );
    }
}

#[test]
fn test_css_custom_property_values() {
    let css = read_main_css();

    // Verify exact assignments for key colour tokens
    let value_checks = [
        (
            r"--color-bg-primary:\s*#111827",
            "--color-bg-primary",
            "#111827",
        ),
        (
            r"--color-bg-secondary:\s*#1f2937",
            "--color-bg-secondary",
            "#1f2937",
        ),
        (
            r"--color-bg-input:\s*#374151",
            "--color-bg-input",
            "#374151",
        ),
        (r"--color-accent:\s*#3b82f6", "--color-accent", "#3b82f6"),
        (
            r"--color-accent-hover:\s*#2563eb",
            "--color-accent-hover",
            "#2563eb",
        ),
        (r"--color-border:\s*#4b5563", "--color-border", "#4b5563"),
    ];

    for (pattern, var, value) in &value_checks {
        assert!(
            regex::Regex::new(pattern)
                .expect("invalid regex pattern")
                .is_match(&css),
            "main.css must set {var} to {value}"
        );
    }
}

#[test]
fn test_css_dark_theme_default() {
    let css = read_main_css();

    // Verify dark background is set on html/body using the design token
    assert!(
        regex::Regex::new(r"background-color\s*:\s*var\(--color-bg-primary\)")
            .expect("invalid regex")
            .is_match(&css),
        "Dark theme must apply background-color referencing --color-bg-primary"
    );

    // Verify light text is the default using the design token
    assert!(
        regex::Regex::new(r"(?m)color\s*:\s*var\(--color-text-primary\)")
            .expect("invalid regex")
            .is_match(&css),
        "Dark theme must use --color-text-primary for default text color"
    );
}

// ---- Base reset styles ----

#[test]
fn test_css_reset_box_sizing() {
    let css = read_main_css();
    assert!(
        regex::Regex::new(
            r"\*\s*,\s*\*::before\s*,\s*\*::after\s*\{[^}]*box-sizing\s*:\s*border-box"
        )
        .expect("invalid regex")
        .is_match(&css),
        "Reset must set box-sizing: border-box on the universal selector"
    );
}

#[test]
fn test_css_reset_margin_padding() {
    let css = read_main_css();
    // Verify margin and padding reset are within the universal selector block
    assert!(
        regex::Regex::new(r"\*\s*,\s*\*::before\s*,\s*\*::after\s*\{[^}]*margin\s*:\s*0")
            .expect("invalid regex")
            .is_match(&css),
        "Reset must set margin to 0 within the universal selector block"
    );
    assert!(
        regex::Regex::new(r"\*\s*,\s*\*::before\s*,\s*\*::after\s*\{[^}]*padding\s*:\s*0")
            .expect("invalid regex")
            .is_match(&css),
        "Reset must set padding to 0 within the universal selector block"
    );
}

// ---- Layout utility classes ----

#[test]
fn test_css_has_app_root_class() {
    let css = read_main_css();
    assert!(
        regex::Regex::new(r#"\.app-root\s*\{[^}]*display\s*:\s*flex"#)
            .expect("invalid regex")
            .is_match(&css),
        "main.css must define .app-root with display: flex"
    );
}

#[test]
fn test_css_has_scroll_area_class() {
    let css = read_main_css();
    assert!(
        regex::Regex::new(r"\.scroll-area\s*\{[^}]*overflow-y\s*:\s*auto")
            .expect("invalid regex")
            .is_match(&css),
        "main.css must define .scroll-area with overflow-y: auto"
    );
}

#[test]
fn test_css_has_sticky_input_class() {
    let css = read_main_css();
    assert!(
        regex::Regex::new(r"\.sticky-input\s*\{[^}]*flex-shrink\s*:\s*0")
            .expect("invalid regex")
            .is_match(&css),
        "main.css must define .sticky-input with flex-shrink: 0"
    );
}

#[test]
fn test_css_has_flex_column_full_class() {
    let css = read_main_css();
    assert!(
        regex::Regex::new(r"\.flex-column-full\s*\{[^}]*flex-direction\s*:\s*column")
            .expect("invalid regex")
            .is_match(&css),
        "main.css must define .flex-column-full with flex-direction: column"
    );
}

// ---- index.html references stylesheet ----

#[test]
fn test_index_html_references_stylesheet() {
    let html = read_index_html();

    assert!(
        regex::Regex::new(r#"<link[^>]*rel="stylesheet"[^>]*href="[^"]*main\.css"[^>]*/>"#)
            .expect("invalid regex")
            .is_match(&html),
        "index.html must contain a <link> tag referencing main.css as a stylesheet"
    );
    assert!(
        regex::Regex::new(r#"<link[^>]*data-trunk[^>]*/>"#)
            .expect("invalid regex")
            .is_match(&html),
        "index.html stylesheet link must have data-trunk attribute"
    );
}

#[test]
fn test_index_html_still_has_rust_tag() {
    let html = read_index_html();

    assert!(
        regex::Regex::new(r#"<link[^>]*data-wasm-cargo="frontend"[^>]*/>"#)
            .expect("invalid regex")
            .is_match(&html),
        "index.html must still contain the Trunk Rust/WASM entrypoint tag with data-wasm-cargo"
    );
}

// ---- Leptos component uses app-root class ----

#[test]
fn test_app_component_uses_app_root_class() {
    let lib_rs = read_app_lib_rs();

    assert!(
        regex::Regex::new(r#"class="[^"]*\bapp-root\b[^"]*""#)
            .expect("invalid regex")
            .is_match(&lib_rs),
        "Leptos App component must apply class=\"app-root\" to its root element"
    );
}
