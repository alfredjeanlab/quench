// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, ShellPolicyConfig};

use super::*;

fn default_policy() -> ShellPolicyConfig {
    ShellPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".shellcheckrc".to_string()],
    }
}

/// Simple classifier for testing: .sh/.bash files are Source, .bats are Test, everything else is Other.
fn simple_classify(path: &Path) -> FileKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("sh") | Some("bash") => {
            // Check for test patterns
            let path_str = path.to_string_lossy();
            if path_str.contains("tests/") || path_str.ends_with("_test.sh") {
                FileKind::Test
            } else {
                FileKind::Source
            }
        }
        Some("bats") => FileKind::Test,
        _ => FileKind::Other,
    }
}

#[test]
fn no_violation_when_only_source_changed() {
    let policy = default_policy();
    let files = [
        Path::new("scripts/build.sh"),
        Path::new("scripts/deploy.sh"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
    assert!(result.changed_lint_config.is_empty());
    assert_eq!(result.changed_source.len(), 2);
}

#[test]
fn no_violation_when_only_lint_config_changed() {
    let policy = default_policy();
    let files = [Path::new(".shellcheckrc")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_source.is_empty());
}

#[test]
fn violation_when_both_changed() {
    let policy = default_policy();
    let files = [Path::new(".shellcheckrc"), Path::new("scripts/build.sh")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert_eq!(result.changed_source.len(), 1);
}

#[test]
fn no_violation_when_policy_disabled() {
    let policy = ShellPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        ..default_policy()
    };
    let files = [Path::new(".shellcheckrc"), Path::new("scripts/build.sh")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(!result.standalone_violated);
}

#[test]
fn detects_hidden_lint_config_files() {
    let policy = default_policy();
    let files = [Path::new(".shellcheckrc"), Path::new("scripts/build.sh")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec![".shellcheckrc"]);
}

#[test]
fn detects_nested_lint_config_files() {
    let policy = default_policy();
    let files = [
        Path::new("scripts/.shellcheckrc"),
        Path::new("scripts/build.sh"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_lint_config[0].contains(".shellcheckrc"));
}

#[test]
fn test_files_count_as_source_for_policy() {
    let policy = default_policy();
    let files = [Path::new(".shellcheckrc"), Path::new("tests/test.bats")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    // Test files should also trigger the violation
    assert!(result.standalone_violated);
    assert_eq!(result.changed_source.len(), 1);
}

#[test]
fn custom_lint_config_list() {
    let policy = ShellPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["shellcheck.yaml".to_string()],
    };
    let files = [Path::new("shellcheck.yaml"), Path::new("scripts/build.sh")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec!["shellcheck.yaml"]);
}

#[test]
fn non_source_non_lint_files_ignored() {
    let policy = default_policy();
    let files = [
        Path::new(".shellcheckrc"),
        Path::new("README.md"),
        Path::new("Makefile"),
    ];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, simple_classify);

    // Only lint config, no source files -> no violation
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_source.is_empty());
}
