// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test run result types.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;

/// Result of running a single test.
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    /// Test name.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Test duration.
    pub duration: Duration,
}

impl TestResult {
    /// Create a passing test result.
    pub fn passed(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            passed: true,
            duration,
        }
    }

    /// Create a failing test result.
    pub fn failed(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            passed: false,
            duration,
        }
    }
}

/// Result of running an entire test suite.
#[derive(Debug, Clone)]
pub struct TestRunResult {
    /// Whether all tests passed.
    pub passed: bool,
    /// Whether the suite was skipped (runner unavailable).
    pub skipped: bool,
    /// Error message if skipped or failed.
    pub error: Option<String>,
    /// Total wall-clock time.
    pub total_time: Duration,
    /// Individual test results (if available).
    pub tests: Vec<TestResult>,
    /// Coverage percentage (0-100) by language.
    pub coverage: Option<HashMap<String, f64>>,
}

impl TestRunResult {
    /// Create a successful result with no tests.
    pub fn passed(total_time: Duration) -> Self {
        Self {
            passed: true,
            skipped: false,
            error: None,
            total_time,
            tests: Vec::new(),
            coverage: None,
        }
    }

    /// Create a failed result.
    pub fn failed(total_time: Duration, error: impl Into<String>) -> Self {
        Self {
            passed: false,
            skipped: false,
            error: Some(error.into()),
            total_time,
            tests: Vec::new(),
            coverage: None,
        }
    }

    /// Create a skipped result (runner unavailable).
    pub fn skipped(error: impl Into<String>) -> Self {
        Self {
            passed: false,
            skipped: true,
            error: Some(error.into()),
            total_time: Duration::ZERO,
            tests: Vec::new(),
            coverage: None,
        }
    }

    /// Add test results.
    pub fn with_tests(mut self, tests: Vec<TestResult>) -> Self {
        // Recompute passed based on individual tests if we have them
        if !tests.is_empty() {
            self.passed = tests.iter().all(|t| t.passed);
        }
        self.tests = tests;
        self
    }

    /// Add coverage data.
    pub fn with_coverage(mut self, coverage: HashMap<String, f64>) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Get test count.
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Get average test duration (if tests available).
    pub fn avg_duration(&self) -> Option<Duration> {
        if self.tests.is_empty() {
            return None;
        }
        let total: Duration = self.tests.iter().map(|t| t.duration).sum();
        Some(total / self.tests.len() as u32)
    }

    /// Get slowest test (if tests available).
    pub fn slowest_test(&self) -> Option<&TestResult> {
        self.tests.iter().max_by_key(|t| t.duration)
    }

    /// Get count of passed tests.
    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.passed).count()
    }

    /// Get count of failed tests.
    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|t| !t.passed).count()
    }
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
