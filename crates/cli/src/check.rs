//! Check result types for output formatting.

use serde::Serialize;
use std::path::PathBuf;

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
        }
    }

    /// Add value/threshold context to the violation.
    pub fn with_threshold(mut self, value: i64, threshold: i64) -> Self {
        self.value = Some(value);
        self.threshold = Some(threshold);
        self
    }

    /// Add pattern context to the violation.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
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

    /// Error message if check was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// List of violations (omitted if empty).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Violation>,
}

impl CheckResult {
    /// Create a passing check result.
    pub fn passed(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            error: None,
            violations: Vec::new(),
        }
    }

    /// Create a failing check result with violations.
    pub fn failed(name: impl Into<String>, violations: Vec<Violation>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            skipped: false,
            error: None,
            violations,
        }
    }

    /// Create a skipped check result with an error.
    pub fn skipped(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            skipped: true,
            error: Some(error.into()),
            violations: Vec::new(),
        }
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
        let passed = checks.iter().all(|c| c.passed);
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
