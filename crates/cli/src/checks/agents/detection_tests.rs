// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use super::*;

#[test]
fn classify_scope_root_for_direct_child() {
    let root = Path::new("/project");
    let file = Path::new("/project/CLAUDE.md");
    let packages: Vec<String> = vec![];

    assert_eq!(classify_scope(file, root, &packages), Scope::Root);
}

#[test]
fn classify_scope_module_for_nested_file() {
    let root = Path::new("/project");
    let file = Path::new("/project/src/parser/CLAUDE.md");
    let packages: Vec<String> = vec![];

    assert_eq!(classify_scope(file, root, &packages), Scope::Module);
}

#[test]
fn classify_scope_package_with_exact_pattern() {
    let root = Path::new("/project");
    let file = Path::new("/project/packages/api/CLAUDE.md");
    let packages = vec!["packages/api".to_string()];

    assert_eq!(classify_scope(file, root, &packages), Scope::Package("packages/api".to_string()));
}

#[test]
fn classify_scope_package_with_wildcard_pattern() {
    let root = Path::new("/project");
    let file = Path::new("/project/crates/cli/CLAUDE.md");
    let packages = vec!["crates/*".to_string()];

    assert_eq!(classify_scope(file, root, &packages), Scope::Package("crates/cli".to_string()));
}

#[test]
fn is_in_package_exact_match() {
    let relative = Path::new("packages/api/CLAUDE.md");
    assert!(is_in_package(relative, "packages/api"));
}

#[test]
fn is_in_package_wildcard_match() {
    let relative = Path::new("crates/cli/CLAUDE.md");
    assert!(is_in_package(relative, "crates/*"));
}

#[test]
fn is_in_package_no_match() {
    let relative = Path::new("src/lib.rs");
    assert!(!is_in_package(relative, "crates/*"));
}

#[test]
fn extract_package_name_from_wildcard() {
    let relative = Path::new("crates/cli/CLAUDE.md");
    assert_eq!(extract_package_name(relative, "crates/*"), "crates/cli");
}

#[test]
fn extract_package_name_exact() {
    let relative = Path::new("packages/api/CLAUDE.md");
    assert_eq!(extract_package_name(relative, "packages/api"), "packages/api");
}
