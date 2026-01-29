// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python unittest test runner.
//!
//! Executes Python tests using `python -m unittest discover` and parses output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Unittest runner for Python test suites.
pub struct UnittestRunner;

impl TestRunner for UnittestRunner {
    fn name(&self) -> &'static str {
        "unittest"
    }

    fn available(&self, _ctx: &RunnerContext) -> bool {
        // Python's unittest is part of the standard library, so we just check if python is available
        Command::new("python")
            .args(["--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: python -m unittest discover -v [path]
        let mut cmd = Command::new("python");
        cmd.args(["-m", "unittest", "discover", "-v"]);

        // Add test path if specified
        if let Some(path) = &config.path {
            cmd.args(["-s", path]);
        }

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn python: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "unittest");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run unittest: {e}"),
                );
            }
        };

        let total_time = start.elapsed();

        // unittest outputs to stderr, not stdout
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_unittest_output(&stderr, &stdout, total_time)
    }
}

/// Parse unittest output.
///
/// Unittest verbose output format:
/// ```text
/// test_add (test_app.TestApp.test_add) ... ok
/// test_sub (test_app.TestApp.test_sub) ... FAIL
///
/// ======================================================================
/// FAIL: test_sub (test_app.TestApp.test_sub)
/// ----------------------------------------------------------------------
/// Traceback (most recent call last):
///   ...
/// AssertionError: ...
///
/// ----------------------------------------------------------------------
/// Ran 2 tests in 0.001s
///
/// FAILED (failures=1)
/// ```
///
/// Or for passing:
/// ```text
/// test_add (test_app.TestApp.test_add) ... ok
///
/// ----------------------------------------------------------------------
/// Ran 1 test in 0.001s
///
/// OK
/// ```
fn parse_unittest_output(stderr: &str, stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut failed_count = 0;
    let mut error_count = 0;

    // Combine stderr and stdout - unittest typically uses stderr for test output
    let combined = format!("{}\n{}", stderr, stdout);

    for line in combined.lines() {
        let line = line.trim();

        // Parse test result lines: "test_name (module.class.method) ... ok/FAIL/ERROR/skipped"
        if let Some(result) = parse_test_line(line) {
            match result.1 {
                TestStatus::Fail => failed_count += 1,
                TestStatus::Error => error_count += 1,
                _ => {}
            }

            let test_result = match result.1 {
                TestStatus::Ok => TestResult::passed(&result.0, Duration::ZERO),
                TestStatus::Fail | TestStatus::Error => {
                    TestResult::failed(&result.0, Duration::ZERO)
                }
                TestStatus::Skipped => TestResult::skipped(&result.0),
            };
            tests.push(test_result);
        }

        // Parse summary line: "Ran N test(s) in X.XXXs"
        if let Some((count, _duration)) = parse_summary_line(line) {
            // If we didn't get individual test results, use the count
            if tests.is_empty() && count > 0 {
                // We know the count but not individual tests
                // This can happen with non-verbose output
            }
        }
    }

    // Check for final status
    let all_passed = failed_count == 0 && error_count == 0;

    let mut result = if all_passed {
        TestRunResult::passed(total_time)
    } else {
        let error_msg = if error_count > 0 {
            format!("{} tests failed, {} errors", failed_count, error_count)
        } else {
            format!("{} tests failed", failed_count)
        };
        TestRunResult::failed(total_time, error_msg)
    };
    result.tests = tests;

    result
}

/// Test status from unittest output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestStatus {
    Ok,
    Fail,
    Error,
    Skipped,
}

/// Parse a test result line from unittest verbose output.
///
/// Format: "test_name (module.class.method) ... ok/FAIL/ERROR/skipped 'reason'"
fn parse_test_line(line: &str) -> Option<(String, TestStatus)> {
    // Check for the " ... " separator
    let parts: Vec<&str> = line.split(" ... ").collect();
    if parts.len() != 2 {
        return None;
    }

    let test_name = parts[0].trim();
    let status_part = parts[1].trim().to_lowercase();

    // Determine status
    let status = if status_part == "ok" {
        TestStatus::Ok
    } else if status_part.starts_with("fail") {
        TestStatus::Fail
    } else if status_part.starts_with("error") {
        TestStatus::Error
    } else if status_part.starts_with("skipped") {
        TestStatus::Skipped
    } else {
        return None;
    };

    // Extract full test name - prefer the part in parentheses if present
    let name = if let (Some(start), Some(end)) = (test_name.find('('), test_name.find(')')) {
        test_name[start + 1..end].to_string()
    } else {
        test_name.to_string()
    };

    Some((name, status))
}

/// Parse the summary line from unittest output.
///
/// Format: "Ran N test(s) in X.XXXs"
fn parse_summary_line(line: &str) -> Option<(usize, Duration)> {
    if !line.starts_with("Ran ") || !line.contains(" test") || !line.contains(" in ") {
        return None;
    }

    // Extract count: "Ran N test(s) in X.XXXs"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let count = parts[1].parse::<usize>().ok()?;

    // Extract duration from last part (e.g., "0.001s")
    let duration_str = parts.last()?;
    let duration = parse_duration(duration_str)?;

    Some((count, duration))
}

/// Parse duration string from unittest output (e.g., "0.001s").
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<f64>().ok().map(Duration::from_secs_f64)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "unittest_tests.rs"]
mod tests;
