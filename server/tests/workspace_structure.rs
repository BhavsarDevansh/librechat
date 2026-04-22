//! Integration tests verifying the Cargo workspace structure for issue #1.

use std::process::Command;

/// Workspace root directory (one level up from the server crate).
fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../")
}

/// Helper: run a command in the workspace root and assert success.
fn assert_cmd_success(cmd: &str, args: &[&str]) {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(workspace_root())
        .env("RUSTFLAGS", "--deny warnings")
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{cmd}`: {e}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "`{cmd} {}` failed.\nstdout:\n{stdout}\nstderr:\n{stderr}",
            args.join(" ")
        );
    }
}

#[test]
fn test_server_crate_compiles() {
    assert_cmd_success("cargo", &["build", "-p", "server"]);
}

#[test]
fn test_rust_toolchain_toml_exists() {
    let path = std::path::Path::new(workspace_root()).join("rust-toolchain.toml");
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
    let cargo_toml_path = std::path::Path::new(workspace_root()).join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).expect("failed to read root Cargo.toml");

    assert!(
        content.contains("[workspace]"),
        "root Cargo.toml must define a [workspace] section"
    );
    assert!(
        content.contains("\"server\""),
        "workspace members must include \"server\""
    );
    assert!(
        content.contains("\"frontend\""),
        "workspace members must include \"frontend\""
    );
}

#[test]
fn test_workspace_cargo_toml_has_shared_deps() {
    let cargo_toml_path = std::path::Path::new(workspace_root()).join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).expect("failed to read root Cargo.toml");

    assert!(
        content.contains("[workspace.dependencies]"),
        "root Cargo.toml must have a [workspace.dependencies] table"
    );
    for dep in &["axum", "tokio", "serde", "leptos"] {
        assert!(
            content.contains(dep),
            "workspace.dependencies should include `{dep}`"
        );
    }
}

#[test]
fn test_server_cargo_toml_exists() {
    let path = std::path::Path::new(workspace_root())
        .join("server")
        .join("Cargo.toml");
    assert!(path.exists(), "server/Cargo.toml is missing");
}

#[test]
fn test_frontend_cargo_toml_exists() {
    let path = std::path::Path::new(workspace_root())
        .join("frontend")
        .join("Cargo.toml");
    assert!(path.exists(), "frontend/Cargo.toml is missing");
}

#[test]
fn test_frontend_index_html_exists() {
    let path = std::path::Path::new(workspace_root())
        .join("frontend")
        .join("index.html");
    assert!(path.exists(), "frontend/index.html is missing");

    let content = std::fs::read_to_string(&path).expect("failed to read frontend/index.html");
    assert!(
        content.contains("data-trunk"),
        "frontend/index.html must contain a Trunk link tag (data-trunk)"
    );
}
