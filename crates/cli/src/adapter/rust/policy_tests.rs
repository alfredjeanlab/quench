// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, RustPolicyConfig};

#[allow(unused_imports)]
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

// Generate standard policy tests
crate::policy_test_cases! {
    policy_type: RustPolicyConfig,
    default_policy: default_policy,
    classifier: simple_classify,
    source_files: ["src/lib.rs", "src/main.rs"],
    lint_config_file: "rustfmt.toml",
    test_file: "tests/test.rs",
}

// =============================================================================
// Rust-specific tests
// =============================================================================

#[test]
fn detects_hidden_lint_config_files() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = default_policy();
    let result = check_policy(&[".rustfmt.toml", "src/lib.rs"], &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec![".rustfmt.toml"]);
}

#[test]
fn detects_nested_lint_config_files() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = default_policy();
    let result = check_policy(
        &["crates/foo/rustfmt.toml", "src/lib.rs"],
        &policy,
        simple_classify,
    );

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_lint_config[0].contains("rustfmt.toml"));
}

#[test]
fn custom_lint_config_list() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = RustPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["custom-lint.toml".to_string()],
    };
    let result = check_policy(
        &["custom-lint.toml", "src/lib.rs"],
        &policy,
        simple_classify,
    );

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec!["custom-lint.toml"]);
}
