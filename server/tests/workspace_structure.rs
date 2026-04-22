//! Integration tests verifying the Cargo workspace structure for issue #1.

use std::path::Path;

/// Returns the normalized workspace root directory.
fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read_root_cargo_toml() -> toml::Value {
    let path = workspace_root().join("Cargo.toml");
    let content = std::fs::read_to_string(&path).expect("failed to read root Cargo.toml");
    toml::from_str(&content).expect("failed to parse root Cargo.toml")
}

#[test]
fn test_rust_toolchain_toml_exists() {
    let path = workspace_root().join("rust-toolchain.toml");
    assert!(
        path.exists(),
        "rust-toolchain.toml is missing from the repo root"
    );

    let content = std::fs::read_to_string(&path).expect("failed to read rust-toolchain.toml");
    assert!(
        content.contains("stable"),
        "rust-toolchain.toml should specify the stable channel"
    );
    assert!(
        content.contains("wasm32-unknown-unknown"),
        "rust-toolchain.toml should include the wasm32-unknown-unknown target"
    );
}

#[test]
fn test_workspace_cargo_toml_has_members() {
    let manifest = read_root_cargo_toml();

    let members = manifest["workspace"]["members"]
        .as_array()
        .expect("workspace.members should be an array");

    let member_names: Vec<&str> = members.iter().filter_map(|v| v.as_str()).collect();

    assert!(
        member_names.contains(&"server"),
        "workspace members must include \"server\", found: {member_names:?}"
    );
    assert!(
        member_names.contains(&"frontend"),
        "workspace members must include \"frontend\", found: {member_names:?}"
    );
}

#[test]
fn test_workspace_cargo_toml_has_shared_deps() {
    let manifest = read_root_cargo_toml();

    let deps = manifest["workspace"]["dependencies"]
        .as_table()
        .expect("workspace.dependencies should be a table");

    for dep in ["axum", "tokio", "serde", "leptos"] {
        assert!(
            deps.contains_key(dep),
            "workspace.dependencies should include `{dep}`"
        );
    }
}

#[test]
fn test_server_cargo_toml_exists() {
    let path = workspace_root().join("server").join("Cargo.toml");
    assert!(path.exists(), "server/Cargo.toml is missing");
}

#[test]
fn test_frontend_cargo_toml_exists() {
    let path = workspace_root().join("frontend").join("Cargo.toml");
    assert!(path.exists(), "frontend/Cargo.toml is missing");
}

#[test]
fn test_frontend_index_html_exists() {
    let path = workspace_root().join("frontend").join("index.html");
    assert!(path.exists(), "frontend/index.html is missing");

    let content = std::fs::read_to_string(&path).expect("failed to read frontend/index.html");
    assert!(
        content.contains("data-trunk"),
        "frontend/index.html must contain a Trunk link tag (data-trunk)"
    );
}
