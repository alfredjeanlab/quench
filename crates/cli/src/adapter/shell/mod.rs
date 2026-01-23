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

mod policy;
mod suppress;

pub use policy::{ShellPolicyCheckResult, check_lint_policy};
pub use suppress::{ShellcheckSuppress, parse_shellcheck_suppresses};

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
        advice: "Add a # OK: comment explaining why error checking is disabled.",
    },
    EscapePattern {
        name: "eval",
        pattern: r"\beval\s",
        action: EscapeAction::Comment,
        comment: Some("# OK:"),
        advice: "Add a # OK: comment explaining why eval is safe here.",
    },
];

/// Shell language adapter.
pub struct ShellAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
}

impl ShellAdapter {
    /// Create a new Shell adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.sh".to_string(), "**/*.bash".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**/*.bats".to_string(),
                "test/**/*.bats".to_string(),
                "*_test.sh".to_string(),
                "**/*_test.sh".to_string(),
            ]),
        }
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
    ) -> ShellPolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}

#[cfg(test)]
#[path = "../shell_tests.rs"]
mod tests;
