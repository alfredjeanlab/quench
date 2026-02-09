// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::fs;
use tempfile::TempDir;

use super::*;

// =============================================================================
// NPM WORKSPACE TESTS
// =============================================================================

#[test]
fn detects_npm_workspace_from_package_json() {
    let dir = TempDir::new().unwrap();

    // Create root package.json with workspaces
    fs::write(dir.path().join("package.json"), r#"{"name": "root", "workspaces": ["packages/*"]}"#)
        .unwrap();

    // Create packages directory with subpackages
    fs::create_dir_all(dir.path().join("packages/core")).unwrap();
    fs::write(
        dir.path().join("packages/core/package.json"),
        r#"{"name": "@app/core", "version": "1.0.0"}"#,
    )
    .unwrap();

    fs::create_dir_all(dir.path().join("packages/cli")).unwrap();
    fs::write(
        dir.path().join("packages/cli/package.json"),
        r#"{"name": "@app/cli", "version": "1.0.0"}"#,
    )
    .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(ws.is_workspace);
    assert_eq!(ws.patterns, vec!["packages/*"]);
    assert!(ws.package_paths.contains(&"packages/cli".to_string()));
    assert!(ws.package_paths.contains(&"packages/core".to_string()));
    assert_eq!(ws.package_names.get("packages/cli"), Some(&"cli".to_string()));
    assert_eq!(ws.package_names.get("packages/core"), Some(&"core".to_string()));
}

#[test]
fn detects_yarn_workspace_object_form() {
    let dir = TempDir::new().unwrap();

    // Create root package.json with yarn workspace object form
    fs::write(
        dir.path().join("package.json"),
        r#"{"name": "root", "workspaces": {"packages": ["libs/*"]}}"#,
    )
    .unwrap();

    // Create libs directory with subpackage
    fs::create_dir_all(dir.path().join("libs/utils")).unwrap();
    fs::write(
        dir.path().join("libs/utils/package.json"),
        r#"{"name": "utils", "version": "1.0.0"}"#,
    )
    .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(ws.is_workspace);
    assert_eq!(ws.patterns, vec!["libs/*"]);
    assert!(ws.package_paths.contains(&"libs/utils".to_string()));
    assert_eq!(ws.package_names.get("libs/utils"), Some(&"utils".to_string()));
}

// =============================================================================
// PNPM WORKSPACE TESTS
// =============================================================================

#[test]
fn detects_pnpm_workspace() {
    let dir = TempDir::new().unwrap();

    // Create pnpm-workspace.yaml
    fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'apps/*'\n").unwrap();

    // Create apps directory with subpackage
    fs::create_dir_all(dir.path().join("apps/web")).unwrap();
    fs::write(
        dir.path().join("apps/web/package.json"),
        r#"{"name": "web-app", "version": "1.0.0"}"#,
    )
    .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(ws.is_workspace);
    assert_eq!(ws.patterns, vec!["apps/*"]);
    assert!(ws.package_paths.contains(&"apps/web".to_string()));
    assert_eq!(ws.package_names.get("apps/web"), Some(&"web".to_string()));
}

#[test]
fn pnpm_workspace_takes_precedence() {
    let dir = TempDir::new().unwrap();

    // Create both pnpm-workspace.yaml and package.json workspaces
    fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'pnpm-pkgs/*'\n").unwrap();

    fs::write(dir.path().join("package.json"), r#"{"name": "root", "workspaces": ["npm-pkgs/*"]}"#)
        .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(ws.is_workspace);
    // Should use pnpm patterns, not npm
    assert_eq!(ws.patterns, vec!["pnpm-pkgs/*"]);
}

// =============================================================================
// NON-WORKSPACE TESTS
// =============================================================================

#[test]
fn returns_default_for_non_workspace() {
    let dir = TempDir::new().unwrap();

    // Regular package.json without workspaces
    fs::write(dir.path().join("package.json"), r#"{"name": "single-app", "version": "1.0.0"}"#)
        .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(!ws.is_workspace);
    assert!(ws.patterns.is_empty());
    assert!(ws.package_paths.is_empty());
}

#[test]
fn returns_default_for_missing_package_json() {
    let dir = TempDir::new().unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(!ws.is_workspace);
    assert!(ws.patterns.is_empty());
    assert!(ws.package_paths.is_empty());
}

#[test]
fn handles_direct_path_pattern() {
    let dir = TempDir::new().unwrap();

    // Create package.json with direct path (no glob)
    fs::write(
        dir.path().join("package.json"),
        r#"{"name": "root", "workspaces": ["packages/core"]}"#,
    )
    .unwrap();

    fs::create_dir_all(dir.path().join("packages/core")).unwrap();
    fs::write(
        dir.path().join("packages/core/package.json"),
        r#"{"name": "core", "version": "1.0.0"}"#,
    )
    .unwrap();

    let ws = JsWorkspace::from_root(dir.path());
    assert!(ws.is_workspace);
    assert!(ws.package_paths.contains(&"packages/core".to_string()));
    assert_eq!(ws.package_names.get("packages/core"), Some(&"core".to_string()));
}
