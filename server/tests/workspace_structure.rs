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
fn test_rust_toolchain_toml_fields() {
    let path = workspace_root().join("rust-toolchain.toml");
    assert!(
        path.exists(),
        "rust-toolchain.toml is missing from the repo root"
    );

    let content = std::fs::read_to_string(&path).expect("failed to read rust-toolchain.toml");
    let value: toml::Value = toml::from_str(&content).expect("failed to parse rust-toolchain.toml");

    let channel = value["toolchain"]["channel"]
        .as_str()
        .expect("toolchain.channel should be a string");
    assert_eq!(
        channel, "stable",
        "rust-toolchain.toml channel should be \"stable\", got \"{channel}\""
    );

    let targets = value["toolchain"]["targets"]
        .as_array()
        .expect("toolchain.targets should be an array");
    let target_names: Vec<&str> = targets.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        target_names.contains(&"wasm32-unknown-unknown"),
        "rust-toolchain.toml targets should include \"wasm32-unknown-unknown\", got {target_names:?}"
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
fn test_frontend_index_html_has_trunk_tag() {
    let path = workspace_root().join("frontend").join("index.html");
    assert!(path.exists(), "frontend/index.html is missing");

    let content = std::fs::read_to_string(&path).expect("failed to read frontend/index.html");
    assert!(
        content.contains("<link data-trunk rel=\"rust\" data-wasm-cargo=\"frontend\"/>"),
        "frontend/index.html must contain the Trunk Rust entrypoint tag"
    );
}
