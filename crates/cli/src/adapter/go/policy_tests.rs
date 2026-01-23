#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{GoPolicyConfig, LintChangesPolicy};

use super::check_lint_policy;

fn go_classifier(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.ends_with("_test.go") {
        FileKind::Test
    } else if path_str.ends_with(".go") {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

#[test]
fn no_policy_allows_mixed_changes() {
    let policy = GoPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        lint_config: vec![".golangci.yml".to_string()],
    };

    let files: Vec<&Path> = vec![Path::new(".golangci.yml"), Path::new("main.go")];

    let result = check_lint_policy(&files, &policy, go_classifier);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_allows_lint_only() {
    let policy = GoPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".golangci.yml".to_string()],
    };

    let files: Vec<&Path> = vec![Path::new(".golangci.yml")];

    let result = check_lint_policy(&files, &policy, go_classifier);
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
}

#[test]
fn standalone_policy_allows_source_only() {
    let policy = GoPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".golangci.yml".to_string()],
    };

    let files: Vec<&Path> = vec![Path::new("main.go"), Path::new("util_test.go")];

    let result = check_lint_policy(&files, &policy, go_classifier);
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_source.len(), 2);
}

#[test]
fn standalone_policy_fails_mixed_changes() {
    let policy = GoPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".golangci.yml".to_string()],
    };

    let files: Vec<&Path> = vec![Path::new(".golangci.yml"), Path::new("main.go")];

    let result = check_lint_policy(&files, &policy, go_classifier);
    assert!(result.standalone_violated);
}

#[test]
fn recognizes_multiple_lint_configs() {
    let policy = GoPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![
            ".golangci.yml".to_string(),
            ".golangci.yaml".to_string(),
            ".golangci.toml".to_string(),
        ],
    };

    let files: Vec<&Path> = vec![Path::new(".golangci.yaml"), Path::new("main.go")];

    let result = check_lint_policy(&files, &policy, go_classifier);
    assert!(result.standalone_violated);
}
