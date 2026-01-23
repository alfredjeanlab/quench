// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell lint policy checking.
//!
//! Checks that lint configuration changes follow the project's policy.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, ShellPolicyConfig};

/// Result of checking shell lint policy.
#[derive(Debug, Default)]
pub struct ShellPolicyCheckResult {
    /// Lint config files that were changed.
    pub changed_lint_config: Vec<String>,
    /// Source/test files that were changed.
    pub changed_source: Vec<String>,
    /// Whether the standalone policy is violated.
    pub standalone_violated: bool,
}

/// Check shell lint policy against changed files.
///
/// Takes a classifier closure to allow testing without a full adapter.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &ShellPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> ShellPolicyCheckResult {
    if policy.lint_changes == LintChangesPolicy::None {
        return ShellPolicyCheckResult::default();
    }

    let mut result = ShellPolicyCheckResult::default();

    for file in changed_files {
        let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if it's a lint config file (e.g., .shellcheckrc)
        if policy
            .lint_config
            .iter()
            .any(|cfg| filename == cfg || file.to_string_lossy().ends_with(cfg))
        {
            result.changed_lint_config.push(file.display().to_string());
            continue;
        }

        // Check if it's a shell source or test file
        let kind = classify(file);
        if kind == FileKind::Source || kind == FileKind::Test {
            result.changed_source.push(file.display().to_string());
        }
    }

    // Standalone policy violated if both lint config AND source changed
    result.standalone_violated = policy.lint_changes == LintChangesPolicy::Standalone
        && !result.changed_lint_config.is_empty()
        && !result.changed_source.is_empty();

    result
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
