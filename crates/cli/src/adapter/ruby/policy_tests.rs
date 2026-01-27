// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for Ruby lint policy checking.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, RubyPolicyConfig};

use super::*;

fn classify_mock(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.contains("spec/") || path_str.ends_with("_spec.rb") {
        FileKind::Test
    } else if path_str.ends_with(".rb") || path_str.ends_with(".rake") {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

#[test]
fn no_policy_allows_mixed_changes() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        ..Default::default()
    };

    let files = [Path::new(".rubocop.yml"), Path::new("lib/app.rb")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_detects_mixed_changes() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".rubocop.yml"), Path::new("lib/app.rb")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
    assert!(!result.changed_lint_config.is_empty());
    assert!(!result.changed_source.is_empty());
}

#[test]
fn standalone_policy_allows_lint_only_changes() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".rubocop.yml"), Path::new(".rubocop_todo.yml")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_allows_source_only_changes() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new("lib/app.rb"), Path::new("lib/models/user.rb")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(!result.standalone_violated);
}

#[test]
fn detects_standard_yml_as_lint_config() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        ..Default::default()
    };

    let files = [Path::new(".standard.yml"), Path::new("lib/app.rb")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}

#[test]
fn custom_lint_config_files() {
    let policy = RubyPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["custom-rubocop.yml".to_string()],
        ..Default::default()
    };

    let files = [Path::new("custom-rubocop.yml"), Path::new("lib/app.rb")];
    let refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&refs, &policy, classify_mock);
    assert!(result.standalone_violated);
}
