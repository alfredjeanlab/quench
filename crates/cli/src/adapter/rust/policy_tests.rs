// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, RustPolicyConfig};

use super::*;

fn default_policy() -> RustPolicyConfig {
    RustPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![
            "rustfmt.toml".to_string(),
            ".rustfmt.toml".to_string(),
            "clippy.toml".to_string(),
            ".clippy.toml".to_string(),
        ],
    }
}

/// Simple classifier for testing: .rs files are Source, everything else is Other.
fn simple_classify(path: &Path) -> FileKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => {
            // Check for test patterns
            let path_str = path.to_string_lossy();
            if path_str.contains("tests/") || path_str.ends_with("_test.rs") {
                FileKind::Test
            } else {
                FileKind::Source
            }
        }
        _ => FileKind::Other,
    }
}

#[test]
fn no_violation_when_only_source_changed() {
    let policy = default_policy();
    let files = [Path::new("src/lib.rs"), Path::new("src/main.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
    assert!(result.changed_lint_config.is_empty());
    assert_eq!(result.changed_source.len(), 2);
}

#[test]
fn no_violation_when_only_lint_config_changed() {
    let policy = default_policy();
    let files = [Path::new("rustfmt.toml"), Path::new("clippy.toml")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 2);
    assert!(result.changed_source.is_empty());
}

#[test]
fn violation_when_both_changed() {
    let policy = default_policy();
    let files = [Path::new("rustfmt.toml"), Path::new("src/lib.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert_eq!(result.changed_source.len(), 1);
}

#[test]
fn no_violation_when_policy_disabled() {
    let policy = RustPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        ..default_policy()
    };
    let files = [Path::new("rustfmt.toml"), Path::new("src/lib.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
}

#[test]
fn detects_hidden_lint_config_files() {
    let policy = default_policy();
    let files = [Path::new(".rustfmt.toml"), Path::new("src/lib.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec![".rustfmt.toml"]);
}

#[test]
fn detects_nested_lint_config_files() {
    let policy = default_policy();
    let files = [
        Path::new("crates/foo/rustfmt.toml"),
        Path::new("src/lib.rs"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_lint_config[0].contains("rustfmt.toml"));
}

#[test]
fn test_files_count_as_source_for_policy() {
    let policy = default_policy();
    let files = [Path::new("rustfmt.toml"), Path::new("tests/test.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    // Test files should also trigger the violation
    assert!(result.standalone_violated);
    assert_eq!(result.changed_source.len(), 1);
}

#[test]
fn custom_lint_config_list() {
    let policy = RustPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["custom-lint.toml".to_string()],
    };
    let files = [Path::new("custom-lint.toml"), Path::new("src/lib.rs")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec!["custom-lint.toml"]);
}

#[test]
fn non_source_non_lint_files_ignored() {
    let policy = default_policy();
    let files = [
        Path::new("rustfmt.toml"),
        Path::new("README.md"),
        Path::new("Cargo.toml"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    // Only lint config, no source files -> no violation
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_source.is_empty());
}
