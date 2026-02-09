// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, ShellPolicyConfig};

fn default_policy() -> ShellPolicyConfig {
    ShellPolicyConfig {
        check: None,
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".shellcheckrc".to_string()],
    }
}

/// Simple classifier for testing: .sh/.bash files are Source, .bats are Test, everything else is Other.
fn simple_classify(path: &Path) -> FileKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("sh") | Some("bash") => {
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

// Generate standard policy tests
crate::policy_test_cases! {
    policy_type: ShellPolicyConfig,
    default_policy: default_policy,
    classifier: simple_classify,
    source_files: ["scripts/build.sh", "scripts/deploy.sh"],
    lint_config_file: ".shellcheckrc",
    test_file: "tests/test.bats",
}

// =============================================================================
// Shell-specific tests
// =============================================================================

#[test]
fn detects_hidden_lint_config_files() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = default_policy();
    let result = check_policy(&[".shellcheckrc", "scripts/build.sh"], &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec![".shellcheckrc"]);
}

#[test]
fn detects_nested_lint_config_files() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = default_policy();
    let result =
        check_policy(&["scripts/.shellcheckrc", "scripts/build.sh"], &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 1);
    assert!(result.changed_lint_config[0].contains(".shellcheckrc"));
}

#[test]
fn custom_lint_config_list() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = ShellPolicyConfig {
        check: None,
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec!["shellcheck.yaml".to_string()],
    };
    let result = check_policy(&["shellcheck.yaml", "scripts/build.sh"], &policy, simple_classify);

    assert!(result.standalone_violated);
    assert_eq!(result.changed_lint_config, vec!["shellcheck.yaml"]);
}
