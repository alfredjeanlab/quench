// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{GoPolicyConfig, LintChangesPolicy};

fn default_policy() -> GoPolicyConfig {
    GoPolicyConfig {
        check: None,
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".golangci.yml".to_string()],
    }
}

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

// Generate standard policy tests
crate::policy_test_cases! {
    policy_type: GoPolicyConfig,
    default_policy: default_policy,
    classifier: go_classifier,
    source_files: ["main.go", "util.go"],
    lint_config_file: ".golangci.yml",
    test_file: "main_test.go",
}

// =============================================================================
// Go-specific tests
// =============================================================================

#[test]
fn recognizes_multiple_lint_configs() {
    use crate::adapter::common::test_utils::assert_violation;

    let policy = GoPolicyConfig {
        lint_config: vec![
            ".golangci.yml".to_string(),
            ".golangci.yaml".to_string(),
            ".golangci.toml".to_string(),
        ],
        ..default_policy()
    };

    assert_violation(&[".golangci.yaml", "main.go"], &policy, go_classifier);
}
