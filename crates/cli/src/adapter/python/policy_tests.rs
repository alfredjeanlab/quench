// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for Python lint policy checking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::adapter::common::policy::check_lint_policy;
use crate::config::{LintChangesPolicy, PythonPolicyConfig};

fn classify_mock(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.contains("tests/")
        || path_str.contains("test/")
        || path_str.starts_with("test_")
        || path_str.ends_with("_test.py")
        || path_str.ends_with("conftest.py")
    {
        FileKind::Test
    } else if path_str.ends_with(".py") {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

#[test]
fn no_policy_allows_mixed_changes() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        ..Default::default()
    };

    let files = [Path::new("ruff.toml"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_detects_mixed_changes() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("ruff.toml"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
    assert!(!result.changed_lint_config.is_empty());
    assert!(!result.changed_source.is_empty());
}

#[test]
fn standalone_policy_allows_lint_only_changes() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("ruff.toml"), Path::new(".flake8")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_allows_source_only_changes() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("src/app.py"), Path::new("src/models/user.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn detects_hidden_ruff_config() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".ruff.toml"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_flake8_config() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".flake8"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_pylintrc_config() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".pylintrc"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_pylintrc_without_dot() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("pylintrc"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_mypy_ini_config() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("mypy.ini"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_hidden_mypy_ini_config() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".mypy.ini"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_pyproject_toml() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("pyproject.toml"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn detects_setup_cfg() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("setup.cfg"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn test_files_trigger_violation() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("ruff.toml"), Path::new("tests/test_app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn non_source_files_ignored() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [
        Path::new("ruff.toml"),
        Path::new("README.md"),
        Path::new("Makefile"),
    ];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn custom_lint_config_files() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["custom-lint.yml".to_string()],
        ..Default::default()
    };

    let files = [Path::new("custom-lint.yml"), Path::new("src/app.py")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn nested_lint_config_detected() {
    let policy = PythonPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [
        Path::new("packages/foo/pyproject.toml"),
        Path::new("src/app.py"),
    ];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}
