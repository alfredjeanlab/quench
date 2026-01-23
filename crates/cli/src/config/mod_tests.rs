// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn parses_minimal_config() {
    let path = PathBuf::from("quench.toml");
    let config = parse("version = 1\n", &path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn parses_config_with_project() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[project]
name = "test-project"
"#;
    let config = parse(content, &path).unwrap();
    assert_eq!(config.version, 1);
    assert_eq!(config.project.name, Some("test-project".to_string()));
}

#[test]
fn rejects_missing_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse("", &path);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("missing required field: version"));
}

#[test]
fn rejects_unsupported_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse("version = 2\n", &path);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("unsupported config version 2"));
}

#[test]
fn rejects_version_zero() {
    let path = PathBuf::from("quench.toml");
    let result = parse("version = 0\n", &path);
    assert!(result.is_err());
}

#[test]
fn load_reads_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("quench.toml");
    fs::write(&config_path, "version = 1\n").unwrap();

    let config = load(&config_path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn load_fails_on_missing_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("nonexistent.toml");

    let result = load(&config_path);
    assert!(result.is_err());
}

// Unknown key warning tests

#[test]
fn parse_with_warnings_accepts_unknown_top_level_key() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1
unknown_key = true
"#;
    // Should succeed, not error
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn parse_with_warnings_accepts_unknown_nested_key() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.unknown]
field = "value"
"#;
    // Should succeed, not error
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.version, 1);
}

#[test]
fn parse_with_warnings_preserves_known_fields() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1
unknown_key = true

[project]
name = "test"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.version, 1);
    assert_eq!(config.project.name, Some("test".to_string()));
}

#[test]
fn parse_with_warnings_rejects_invalid_version() {
    let path = PathBuf::from("quench.toml");
    let result = parse_with_warnings("version = 99\n", &path);
    assert!(result.is_err());
}

// max_tokens config tests

#[test]
fn parse_max_tokens_default() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, Some(20000));
}

#[test]
fn parse_max_tokens_custom() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.cloc]
max_tokens = 10000
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, Some(10000));
}

#[test]
fn parse_max_tokens_false_disables() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[check.cloc]
max_tokens = false
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.check.cloc.max_tokens, None);
}

// Rust policy config tests

#[test]
fn parse_rust_policy_default() {
    let path = PathBuf::from("quench.toml");
    let content = "version = 1\n";
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.rust.policy.lint_changes, LintChangesPolicy::None);
    assert_eq!(config.rust.policy.lint_config.len(), 4);
    assert!(
        config
            .rust
            .policy
            .lint_config
            .contains(&"rustfmt.toml".to_string())
    );
}

#[test]
fn parse_rust_policy_standalone() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "standalone"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.rust.policy.lint_changes,
        LintChangesPolicy::Standalone
    );
}

#[test]
fn parse_rust_policy_custom_lint_config() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", "custom-lint.toml"]
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(
        config.rust.policy.lint_changes,
        LintChangesPolicy::Standalone
    );
    assert_eq!(config.rust.policy.lint_config.len(), 2);
    assert!(
        config
            .rust
            .policy
            .lint_config
            .contains(&"custom-lint.toml".to_string())
    );
}

#[test]
fn parse_rust_policy_none_explicit() {
    let path = PathBuf::from("quench.toml");
    let content = r#"
version = 1

[rust.policy]
lint_changes = "none"
"#;
    let config = parse_with_warnings(content, &path).unwrap();
    assert_eq!(config.rust.policy.lint_changes, LintChangesPolicy::None);
}
