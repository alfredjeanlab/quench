// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Lint policy checking.
//!
//! Checks that lint configuration changes follow the project's policy.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::RustPolicyConfig;

// Re-export from common
pub use crate::adapter::common::policy::PolicyCheckResult;

/// Check lint policy against changed files.
///
/// Takes a classifier closure to allow testing without a full adapter.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &RustPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    crate::adapter::common::policy::check_lint_policy(changed_files, policy, classify)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
