// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::PathBuf;

// =============================================================================
// GOLANG EXCLUDE CONFIG
// =============================================================================

#[test]
fn golang_exclude_field_parsing() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[golang]
exclude = ["vendor/", "generated/**"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.golang.exclude, vec!["vendor/", "generated/**"]);
}

#[test]
fn golang_exclude_alias_ignore() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[golang]
ignore = ["vendor/"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.golang.exclude, vec!["vendor/"]);
}

#[test]
fn golang_exclude_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();
    assert_eq!(config.golang.exclude, vec!["vendor/**"]);
}

// =============================================================================
// JAVASCRIPT EXCLUDE CONFIG
// =============================================================================

#[test]
fn javascript_exclude_field_parsing() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[javascript]
exclude = ["node_modules/", ".next/"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.javascript.exclude, vec!["node_modules/", ".next/"]);
}

#[test]
fn javascript_exclude_alias_ignore() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[javascript]
ignore = ["dist/"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.javascript.exclude, vec!["dist/"]);
}

#[test]
fn javascript_exclude_defaults() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();
    assert!(config.javascript.exclude.contains(&"node_modules/**".to_string()));
    assert!(config.javascript.exclude.contains(&"dist/**".to_string()));
}

// =============================================================================
// SHELL EXCLUDE CONFIG
// =============================================================================

#[test]
fn shell_exclude_field_parsing() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell]
exclude = ["tmp/"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.shell.exclude, vec!["tmp/"]);
}

#[test]
fn shell_exclude_alias_ignore() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[shell]
ignore = ["build/"]
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.shell.exclude, vec!["build/"]);
}

#[test]
fn shell_exclude_defaults_empty() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse(content, &path).unwrap();
    assert!(config.shell.exclude.is_empty());
}
