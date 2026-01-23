#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use tempfile::TempDir;

use super::*;

fn create_workspace(dir: &Path, manifest: &str) {
    std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
}

fn create_package(dir: &Path, name: &str) {
    let pkg_dir = dir.join(name);
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(
        pkg_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
"#
        ),
    )
    .unwrap();
}

#[test]
fn single_package() {
    let dir = TempDir::new().unwrap();
    create_workspace(
        dir.path(),
        r#"[package]
name = "my-project"
version = "0.1.0"
"#,
    );

    let workspace = CargoWorkspace::from_root(dir.path());
    assert!(!workspace.is_workspace);
    assert_eq!(workspace.packages, vec!["my-project"]);
    assert!(workspace.member_patterns.is_empty());
}

#[test]
fn workspace_with_explicit_members() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("crates")).unwrap();
    create_package(&dir.path().join("crates"), "core");
    create_package(&dir.path().join("crates"), "cli");

    create_workspace(
        dir.path(),
        r#"[workspace]
members = ["crates/core", "crates/cli"]
"#,
    );

    let workspace = CargoWorkspace::from_root(dir.path());
    assert!(workspace.is_workspace);
    assert_eq!(workspace.packages, vec!["cli", "core"]);
    assert_eq!(workspace.member_patterns, vec!["crates/core", "crates/cli"]);
}

#[test]
fn workspace_with_glob_members() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("crates")).unwrap();
    create_package(&dir.path().join("crates"), "core");
    create_package(&dir.path().join("crates"), "cli");

    create_workspace(
        dir.path(),
        r#"[workspace]
members = ["crates/*"]
"#,
    );

    let workspace = CargoWorkspace::from_root(dir.path());
    assert!(workspace.is_workspace);
    assert_eq!(workspace.packages, vec!["cli", "core"]);
    assert_eq!(workspace.member_patterns, vec!["crates/*"]);
}

#[test]
fn no_cargo_toml() {
    let dir = TempDir::new().unwrap();
    let workspace = CargoWorkspace::from_root(dir.path());
    assert!(!workspace.is_workspace);
    assert!(workspace.packages.is_empty());
    assert!(workspace.member_patterns.is_empty());
}
