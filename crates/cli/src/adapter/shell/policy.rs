// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell lint policy checking.
//!
//! Checks that lint configuration changes follow the project's policy.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::ShellPolicyConfig;

// Re-export from common
pub use crate::adapter::common::policy::PolicyCheckResult;

/// Alias for backward compatibility.
pub type ShellPolicyCheckResult = PolicyCheckResult;

/// Check shell lint policy against changed files.
///
/// Takes a classifier closure to allow testing without a full adapter.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &ShellPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    crate::adapter::common::policy::check_lint_policy(changed_files, policy, classify)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
