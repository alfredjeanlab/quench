// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell language adapter.
//!
//! Provides Shell-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for shell scripts
//! - Default escape patterns (set +e, eval)
//! - Shellcheck suppress directive parsing
//!
//! See docs/specs/langs/shell.md for specification.

use std::path::Path;

use globset::GlobSet;

mod suppress;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use suppress::{ShellcheckSuppress, parse_shellcheck_suppresses};

use super::common;
use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::ShellPolicyConfig;

/// Default escape patterns for Shell.
const SHELL_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "set_plus_e",
        pattern: r"set \+e",
        action: EscapeAction::Comment,
        comment: Some("# OK:"),
        advice: "Most bash scripts should use 'set -e' to exit on errors. \
                 Consider adding it to this script. \
                 If error checking was intentionally disabled, add a # OK: comment explaining why.",
        in_tests: None,
    },
    EscapePattern {
        name: "eval",
        pattern: r"\beval\s",
        action: EscapeAction::Comment,
        comment: Some("# OK:"),
        advice: "eval can execute arbitrary code and is a common source of injection vulnerabilities. \
                 If this usage is safe, add a # OK: comment explaining why.",
        in_tests: None,
    },
];

/// Shell language adapter.
pub struct ShellAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl ShellAdapter {
    /// Create a new Shell adapter with default patterns.
    ///
    /// Note: `**/*_test.sh` matches root-level files like `foo_test.sh` too
    /// (the `**/` prefix matches zero or more path components), so a separate
    /// `*_test.sh` pattern is not needed.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.sh".to_string(), "**/*.bash".to_string()]),
            test_patterns: build_glob_set(&[
                "**/tests/**/*.bats".to_string(),
                "**/test/**/*.bats".to_string(),
                "**/*_test.sh".to_string(),
            ]),
            exclude_patterns: build_glob_set(&[]),
        }
    }

    /// Create a Shell adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            exclude_patterns: build_glob_set(&patterns.exclude),
        }
    }

    /// Check if a path should be excluded.
    pub fn should_exclude(&self, path: &Path) -> bool {
        common::patterns::check_exclude_patterns(path, &self.exclude_patterns, None)
    }
}

impl Default for ShellAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for ShellAdapter {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sh", "bash", "bats"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Check exclusions first
        if self.should_exclude(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Source patterns
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }

        FileKind::Other
    }

    fn default_escapes(&self) -> &'static [EscapePattern] {
        SHELL_ESCAPE_PATTERNS
    }
}

impl ShellAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &ShellPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
    }
}

#[cfg(test)]
#[path = "../shell_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
