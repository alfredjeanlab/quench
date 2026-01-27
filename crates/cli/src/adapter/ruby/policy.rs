// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ruby lint policy checking.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::RubyPolicyConfig;

// Re-export from common
pub use crate::adapter::common::policy::PolicyCheckResult;

/// Check Ruby lint policy against changed files.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &RubyPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    crate::adapter::common::policy::check_lint_policy(changed_files, policy, classify)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
