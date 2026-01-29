// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check: test timing metrics.
//!
//! Collects timing metrics from test runs:
//! - Total test execution time
//! - Average test time
//! - Maximum single test time
//!
//! In non-CI mode, returns stub results.
//! In CI mode, runs tests and collects timing metrics.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::adapter::{ProjectLanguage, detect_language};
use crate::check::{Check, CheckContext, CheckResult};

pub struct TestsCheck;

impl Check for TestsCheck {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn description(&self) -> &'static str {
        "Test correlation"
    }

    fn default_enabled(&self) -> bool {
        true
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Only collect metrics in CI mode with timing ratchets enabled
        let collect_timing = ctx.ci_mode
            && (ctx.config.ratchet.test_time_total
                || ctx.config.ratchet.test_time_avg
                || ctx.config.ratchet.test_time_max);

        if !collect_timing {
            return CheckResult::stub(self.name());
        }

        let language = detect_language(ctx.root);

        match run_tests_with_timing(ctx.root, language) {
            Some(metrics) => CheckResult::passed(self.name()).with_metrics(metrics.to_json()),
            None => CheckResult::skipped(self.name(), "Failed to run tests"),
        }
    }
}

/// Collected test timing metrics.
#[derive(Debug, Clone, Default)]
pub struct TestTimingMetrics {
    pub total: Duration,
    pub avg: Duration,
    pub max: Duration,
    pub test_count: usize,
    pub slowest_test: Option<String>,
}

impl TestTimingMetrics {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "total": self.total.as_secs_f64(),
            "avg": self.avg.as_secs_f64(),
            "max": self.max.as_secs_f64(),
            "test_count": self.test_count,
            "slowest_test": self.slowest_test,
        })
    }
}

/// Run tests and collect timing metrics.
fn run_tests_with_timing(root: &Path, language: ProjectLanguage) -> Option<TestTimingMetrics> {
    let test_cmd = match language {
        ProjectLanguage::Rust => vec!["cargo", "test", "--all"],
        ProjectLanguage::Go => vec!["go", "test", "-v", "./..."],
        ProjectLanguage::JavaScript => {
            // Try npm test or yarn test
            if root.join("yarn.lock").exists() {
                vec!["yarn", "test"]
            } else {
                vec!["npm", "test"]
            }
        }
        _ => return None,
    };

    let start = Instant::now();
    let output = Command::new(test_cmd[0])
        .args(&test_cmd[1..])
        .current_dir(root)
        .output()
        .ok()?;
    let total = start.elapsed();

    if !output.status.success() {
        return None;
    }

    // Parse test output for per-test timing
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (test_count, max, slowest) = parse_test_timing(&stdout, &stderr, language);

    let avg = if test_count > 0 {
        total / test_count as u32
    } else {
        Duration::ZERO
    };

    Some(TestTimingMetrics {
        total,
        avg,
        max,
        test_count,
        slowest_test: slowest,
    })
}

/// Parse test output to extract per-test timing.
fn parse_test_timing(
    stdout: &str,
    stderr: &str,
    language: ProjectLanguage,
) -> (usize, Duration, Option<String>) {
    match language {
        ProjectLanguage::Rust => parse_rust_test_output(stdout, stderr),
        ProjectLanguage::Go => parse_go_test_output(stdout, stderr),
        _ => estimate_from_output(stdout),
    }
}

/// Parse Rust cargo test output.
fn parse_rust_test_output(_stdout: &str, stderr: &str) -> (usize, Duration, Option<String>) {
    // Cargo test output is on stderr
    // Format: "test foo::bar ... ok" or "test foo::bar ... FAILED"
    let mut test_count = 0;

    for line in stderr.lines() {
        if line.starts_with("test ") && (line.ends_with(" ok") || line.ends_with(" FAILED")) {
            test_count += 1;
        }
    }

    // Cargo doesn't report individual test times by default
    // Use test_count to estimate avg from total
    // For max, we'd need --format=json which isn't always available

    (test_count, Duration::ZERO, None)
}

/// Parse Go test output.
fn parse_go_test_output(stdout: &str, _stderr: &str) -> (usize, Duration, Option<String>) {
    // Go test -v output format:
    // "=== RUN   TestFoo"
    // "--- PASS: TestFoo (0.00s)"
    let mut test_count = 0;
    let mut max_duration = Duration::ZERO;
    let mut slowest_test = None;

    for line in stdout.lines() {
        if line.starts_with("--- PASS:") || line.starts_with("--- FAIL:") {
            test_count += 1;

            // Extract test name and duration
            // Format: "--- PASS: TestFoo (0.00s)"
            if let Some(paren_start) = line.rfind('(')
                && let Some(paren_end) = line.rfind("s)")
            {
                let duration_str = &line[paren_start + 1..paren_end];
                if let Ok(secs) = duration_str.parse::<f64>() {
                    let duration = Duration::from_secs_f64(secs);
                    if duration > max_duration {
                        max_duration = duration;
                        // Extract test name
                        let name_part = &line[10..paren_start].trim();
                        slowest_test = Some(name_part.to_string());
                    }
                }
            }
        }
    }

    (test_count, max_duration, slowest_test)
}

/// Estimate test count from output for unknown languages.
fn estimate_from_output(stdout: &str) -> (usize, Duration, Option<String>) {
    // Count lines that look like test results
    let test_count = stdout
        .lines()
        .filter(|line| {
            line.contains(" ok")
                || line.contains(" PASS")
                || line.contains(" pass")
                || line.contains(" FAIL")
                || line.contains(" fail")
        })
        .count();

    (test_count.max(1), Duration::ZERO, None)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
