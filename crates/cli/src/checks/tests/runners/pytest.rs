// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Pytest test runner.
//!
//! Executes Python tests using `pytest --durations=0 -v` and parses output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Pytest runner for Python test suites.
pub struct PytestRunner;

impl TestRunner for PytestRunner {
    fn name(&self) -> &'static str {
        "pytest"
    }

    fn available(&self, _ctx: &RunnerContext) -> bool {
        // Check if pytest is installed
        Command::new("pytest")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: pytest --durations=0 -v <path>
        let mut cmd = Command::new("pytest");
        cmd.args(["--durations=0", "-v"]);

        // Add test path if specified
        if let Some(path) = &config.path {
            cmd.arg(path);
        }

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn pytest: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "pytest");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run pytest: {e}"),
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_pytest_output(&stdout, total_time)
    }
}

/// Parse pytest output with durations.
///
/// Output format:
/// ```text
/// ============================= slowest durations =============================
/// 0.45s call     test_module.py::test_one
/// 0.23s call     test_module.py::test_two
/// 0.01s setup    test_module.py::test_one
/// ============================= 2 passed in 0.68s =============================
/// ```
fn parse_pytest_output(stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut in_durations = false;
    let mut passed_count = 0;
    let mut failed_count = 0;

    for line in stdout.lines() {
        let line = line.trim();

        // Detect durations section start
        if line.contains("slowest durations") {
            in_durations = true;
            continue;
        }

        // Detect section end (line of ='s)
        if in_durations && line.starts_with("=====") {
            in_durations = false;
            // Don't continue - the summary line also starts with '='
        }

        // Parse duration lines: "0.45s call test_module.py::test_one"
        if in_durations && let Some(result) = parse_duration_line(line) {
            tests.push(result);
        }

        // Parse summary: "2 passed, 1 failed in 0.68s" or "2 passed in 0.68s"
        if let Some((passed, failed)) = parse_summary_line(line) {
            passed_count = passed;
            failed_count = failed;
        }
    }

    // Determine overall pass/fail from summary
    let all_passed = failed_count == 0;

    let mut result = if all_passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;

    // Ensure passed status reflects summary
    if !all_passed {
        result.passed = false;
    }

    // If we got pass/fail counts but no individual tests, still report status
    if result.tests.is_empty() && (passed_count > 0 || failed_count > 0) {
        result.passed = all_passed;
    }

    result
}

/// Parse a duration line from pytest output.
///
/// Format: "0.45s call     test_module.py::test_one"
fn parse_duration_line(line: &str) -> Option<TestResult> {
    // Split on whitespace to get: ["0.45s", "call", "test_module.py::test_one"]
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    // Parse duration (e.g., "0.45s")
    let duration_str = parts[0];
    let duration = parse_duration(duration_str)?;

    // Get phase (call, setup, teardown)
    let phase = parts[1];

    // Only include "call" phase (the actual test execution)
    if phase != "call" {
        return None;
    }

    // Get test name
    let name = parts[2];

    // All tests in duration output are considered passed (failures show separately)
    Some(TestResult::passed(name, duration))
}

/// Parse duration string from pytest output (e.g., "0.45s").
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<f64>().ok().map(Duration::from_secs_f64)
    } else {
        None
    }
}

/// Parse summary line for pass/fail counts.
///
/// Examples:
/// - "===== 2 passed in 0.68s ====="
/// - "===== 2 passed, 1 failed in 0.68s ====="
/// - "===== 1 failed, 2 passed, 1 skipped in 1.00s ====="
fn parse_summary_line(line: &str) -> Option<(usize, usize)> {
    // Summary lines contain " passed" or " failed" and " in "
    if !line.contains(" passed") && !line.contains(" failed") {
        return None;
    }
    if !line.contains(" in ") {
        return None;
    }

    let mut passed = 0;
    let mut failed = 0;

    // Use regex-like matching: look for "N passed" and "N failed" patterns
    // Split by whitespace and look for number followed by "passed" or "failed"
    let words: Vec<&str> = line.split_whitespace().collect();
    for window in words.windows(2) {
        if (window[1] == "passed" || window[1] == "passed,")
            && let Ok(n) = window[0].parse::<usize>()
        {
            passed = n;
        }
        if (window[1] == "failed" || window[1] == "failed,")
            && let Ok(n) = window[0].parse::<usize>()
        {
            failed = n;
        }
    }

    // Only return Some if we found at least one count
    if passed > 0 || failed > 0 {
        Some((passed, failed))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "pytest_tests.rs"]
mod tests;
