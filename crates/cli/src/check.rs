// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Check result types for output formatting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;

use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::config::Config;
use crate::walker::WalkedFile;

/// Context passed to all checks during execution.
pub struct CheckContext<'a> {
    /// Project root directory.
    pub root: &'a Path,
    /// Discovered files from the walker.
    pub files: &'a [WalkedFile],
    /// Parsed configuration.
    pub config: &'a Config,
    /// Violation limit (None = unlimited).
    pub limit: Option<usize>,
    /// Running violation count across all checks.
    pub violation_count: &'a AtomicUsize,
    /// Files changed since base ref (for --base flag).
    pub changed_files: Option<&'a [PathBuf]>,
    /// Whether to automatically fix violations when possible.
    pub fix: bool,
    /// Show what --fix would change without modifying files.
    pub dry_run: bool,
    /// Whether running in CI mode (enables slow checks like commit validation).
    pub ci_mode: bool,
    /// Base branch for commit comparison in CI mode.
    pub base_branch: Option<&'a str>,
    /// Whether checking only staged changes (--staged flag).
    pub staged: bool,
}

/// The Check trait defines a single quality check.
///
/// Object-safe to allow dynamic dispatch via `Box<dyn Check>`.
pub trait Check: Send + Sync {
    /// Unique identifier for this check (e.g., "cloc", "escapes").
    fn name(&self) -> &'static str;

    /// Human-readable description for help output.
    fn description(&self) -> &'static str;

    /// Run the check and return results.
    ///
    /// Implementations should:
    /// - Return `CheckResult::skipped()` if prerequisites are missing
    /// - Respect `ctx.limit` for early termination
    /// - Handle errors gracefully without panicking
    fn run(&self, ctx: &CheckContext) -> CheckResult;

    /// Whether this check is enabled by default in fast mode.
    fn default_enabled(&self) -> bool {
        true
    }
}

/// A single violation within a check.
#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    /// File path (None for non-file violations like commit messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,

    /// Line number (None if not applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,

    /// Violation category (check-specific).
    #[serde(rename = "type")]
    pub violation_type: String,

    /// Actionable guidance on how to fix.
    pub advice: String,

    /// Current value (for threshold violations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,

    /// Threshold that was exceeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<i64>,

    /// Pattern name that matched (for escape violations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Total line count (for cloc violations, always included for convenience).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<i64>,

    /// Non-blank line count (for cloc violations, always included for convenience).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonblank: Option<i64>,

    /// Other file involved in sync comparison.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_file: Option<PathBuf>,

    /// Section name for section-level violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,

    /// Commit hash for commit-level violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,

    /// Full commit message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Expected docs pattern (for area-specific violations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_docs: Option<String>,

    /// Area name that was matched (for area-specific violations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,

    /// How the area was matched ("scope" or "source").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_match: Option<String>,

    /// Referenced path for broken_toc violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Link target for broken_link violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

impl Violation {
    /// Create a file-based violation with line number.
    pub fn file(
        file: impl Into<PathBuf>,
        line: u32,
        violation_type: impl Into<String>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            file: Some(file.into()),
            line: Some(line),
            violation_type: violation_type.into(),
            advice: advice.into(),
            value: None,
            threshold: None,
            pattern: None,
            lines: None,
            nonblank: None,
            other_file: None,
            section: None,
            commit: None,
            message: None,
            expected_docs: None,
            area: None,
            area_match: None,
            path: None,
            target: None,
        }
    }

    /// Create a file-based violation without line number.
    pub fn file_only(
        file: impl Into<PathBuf>,
        violation_type: impl Into<String>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            file: Some(file.into()),
            line: None,
            violation_type: violation_type.into(),
            advice: advice.into(),
            value: None,
            threshold: None,
            pattern: None,
            lines: None,
            nonblank: None,
            other_file: None,
            section: None,
            commit: None,
            message: None,
            expected_docs: None,
            area: None,
            area_match: None,
            path: None,
            target: None,
        }
    }

    /// Create a commit-level violation (no file).
    pub fn commit_violation(
        hash: impl Into<String>,
        message: impl Into<String>,
        violation_type: impl Into<String>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            file: None,
            line: None,
            violation_type: violation_type.into(),
            advice: advice.into(),
            value: None,
            threshold: None,
            pattern: None,
            lines: None,
            nonblank: None,
            other_file: None,
            section: None,
            commit: Some(hash.into()),
            message: Some(message.into()),
            expected_docs: None,
            area: None,
            area_match: None,
            path: None,
            target: None,
        }
    }

    /// Add expected docs pattern for area-specific violations.
    pub fn with_expected_docs(mut self, docs: impl Into<String>) -> Self {
        self.expected_docs = Some(docs.into());
        self
    }

    /// Add area information to the violation.
    pub fn with_area(mut self, name: impl Into<String>, match_type: impl Into<String>) -> Self {
        self.area = Some(name.into());
        self.area_match = Some(match_type.into());
        self
    }

    /// Add value/threshold context to the violation.
    pub fn with_threshold(mut self, value: i64, threshold: i64) -> Self {
        self.value = Some(value);
        self.threshold = Some(threshold);
        self
    }

    /// Add line counts for cloc violations (both always included in JSON for convenience).
    pub fn with_line_counts(mut self, lines: i64, nonblank: i64) -> Self {
        self.lines = Some(lines);
        self.nonblank = Some(nonblank);
        self
    }

    /// Add pattern context to the violation.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Add sync context to the violation.
    pub fn with_sync(mut self, other_file: impl Into<PathBuf>, section: impl Into<String>) -> Self {
        self.other_file = Some(other_file.into());
        self.section = Some(section.into());
        self
    }

    /// Add section context to the violation.
    pub fn with_section(mut self, section: impl Into<String>) -> Self {
        self.section = Some(section.into());
        self
    }

    /// Add path context for broken_toc violations.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add target context for broken_link violations.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }
}

/// Result of running a single check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Check identifier (e.g., "cloc", "escapes").
    pub name: String,

    /// Whether this check passed.
    pub passed: bool,

    /// True if check was skipped due to an error.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,

    /// True if check is a stub (not yet implemented).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stub: bool,

    /// True if fixes were applied.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,

    /// Error message if check was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// List of violations (omitted if empty).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,

    /// Summary of fixes applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_summary: Option<JsonValue>,

    /// Aggregated metrics for this check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<JsonValue>,

    /// Per-package breakdown of metrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_package: Option<HashMap<String, JsonValue>>,
}

impl CheckResult {
    /// Create a passing check result.
    pub fn passed(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            stub: false,
            fixed: false,
            error: None,
            violations: Vec::new(),
            fix_summary: None,
            metrics: None,
            by_package: None,
        }
    }

    /// Create a failing check result with violations.
    pub fn failed(name: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            skipped: false,
            stub: false,
            fixed: false,
            error: None,
            violations,
            fix_summary: None,
            metrics: None,
            by_package: None,
        }
    }

    /// Create a passing check result that includes warnings (violations that don't cause failure).
    ///
    /// Used when violations exist but all are at warn level, so the check passes
    /// while still reporting issues.
    pub fn passed_with_warnings(name: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            stub: false,
            fixed: false,
            error: None,
            violations,
            fix_summary: None,
            metrics: None,
            by_package: None,
        }
    }

    /// Create a skipped check result with an error.
    pub fn skipped(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            skipped: true,
            stub: false,
            fixed: false,
            error: Some(error.into()),
            violations: Vec::new(),
            fix_summary: None,
            metrics: None,
            by_package: None,
        }
    }

    /// Create a stub check result (not yet implemented).
    pub fn stub(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            stub: true,
            fixed: false,
            error: None,
            violations: Vec::new(),
            fix_summary: None,
            metrics: None,
            by_package: None,
        }
    }

    /// Create a fixed check result.
    pub fn fixed(name: impl Into<String>, summary: JsonValue) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            stub: false,
            fixed: true,
            error: None,
            violations: Vec::new(),
            fix_summary: Some(summary),
            metrics: None,
            by_package: None,
        }
    }

    /// Create a result with metrics.
    pub fn with_metrics(mut self, metrics: JsonValue) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Add per-package metrics breakdown.
    pub fn with_by_package(mut self, by_package: HashMap<String, JsonValue>) -> Self {
        self.by_package = Some(by_package);
        self
    }

    /// Mark this result as having fixes applied.
    pub fn with_fix_summary(mut self, summary: JsonValue) -> Self {
        self.fixed = true;
        self.fix_summary = Some(summary);
        self
    }
}

/// Aggregated results from all checks.
#[derive(Debug, Clone, Serialize)]
pub struct CheckOutput {
    /// ISO 8601 timestamp.
    pub timestamp: String,

    /// Whether all checks passed.
    pub passed: bool,

    /// Results for each check.
    pub checks: Vec<CheckResult>,
}

impl CheckOutput {
    /// Create output from check results.
    pub fn new(timestamp: String, checks: Vec<CheckResult>) -> Self {
        // Overall passed = all non-skipped checks passed
        let passed = checks.iter().all(|c| c.passed || c.skipped);
        Self {
            timestamp,
            passed,
            checks,
        }
    }

    /// Count total violations across all checks.
    pub fn total_violations(&self) -> usize {
        self.checks.iter().map(|c| c.violations.len()).sum()
    }
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;
