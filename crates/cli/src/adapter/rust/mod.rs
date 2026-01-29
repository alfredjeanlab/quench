// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust language adapter.
//!
//! Provides Rust-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Rust projects
//! - Inline test detection via #[cfg(test)]
//!
//! See docs/specs/langs/rust.md for specification.

use std::path::Path;

use globset::GlobSet;

use super::glob::build_glob_set;

mod cfg_test;
mod suppress;
mod workspace;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use cfg_test::CfgTestInfo;
pub use suppress::{SuppressAttr, parse_suppress_attrs};
pub use workspace::CargoWorkspace;

use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::RustPolicyConfig;

/// Default escape patterns for Rust.
///
/// Note: Does not include `.unwrap()` or `.expect()` - use Clippy's `unwrap_used`
/// and `expect_used` lints for that. Quench ensures escapes are commented, not forbidden.
const RUST_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe",
        pattern: r"unsafe\s*\{",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining the invariants.",
        in_tests: None,
    },
    EscapePattern {
        name: "transmute",
        // SAFETY: This string literal defines the pattern; it's not actual transmute usage.
        pattern: r"mem::transmute",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining type compatibility.",
        in_tests: None,
    },
];

/// Rust language adapter.
pub struct RustAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RustAdapter {
    /// Create a new Rust adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.rs".to_string()]),
            test_patterns: build_glob_set(&[
                "**/tests/**".to_string(),
                "**/test/**/*.rs".to_string(),
                "**/benches/**".to_string(),
                "**/*_test.rs".to_string(),
                "**/*_tests.rs".to_string(),
            ]),
            ignore_patterns: build_glob_set(&["target/**".to_string()]),
        }
    }

    /// Create a Rust adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&patterns.ignore),
        }
    }

    /// Check if a path should be ignored (e.g., target/).
    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Default for RustAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for RustAdapter {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
        if self.should_ignore(path) {
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
        RUST_ESCAPE_PATTERNS
    }
}

/// Result of classifying lines within a single file.
#[derive(Debug, Default)]
pub struct LineClassification {
    pub source_lines: usize,
    pub test_lines: usize,
}

impl RustAdapter {
    /// Parse a file and return line-level classification.
    ///
    /// Returns a struct with source and test line counts.
    pub fn classify_lines(&self, path: &Path, content: &str) -> LineClassification {
        // First check if the whole file is a test file
        let file_kind = self.classify(path);

        if file_kind == FileKind::Test {
            // Entire file is test code
            let total_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
            return LineClassification {
                source_lines: 0,
                test_lines: total_lines,
            };
        }

        if file_kind != FileKind::Source {
            return LineClassification::default();
        }

        // Parse for #[cfg(test)] blocks
        let cfg_info = CfgTestInfo::parse(content);

        let mut source_lines = 0;
        let mut test_lines = 0;

        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            if cfg_info.is_test_line(idx) {
                test_lines += 1;
            } else {
                source_lines += 1;
            }
        }

        LineClassification {
            source_lines,
            test_lines,
        }
    }

    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &RustPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
    }
}

#[cfg(test)]
#[path = "../rust_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
