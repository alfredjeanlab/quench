// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Minitest test runner.
//!
//! Executes Ruby tests using Minitest with JSON output via minitest-reporters,
//! or falls back to parsing standard dot output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::json_utils::find_json_object;
use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Minitest runner for Ruby test suites.
pub struct MinitestRunner;

impl TestRunner for MinitestRunner {
    fn name(&self) -> &'static str {
        "minitest"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if bundler is available
        let bundler_available = Command::new("bundle")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has test/ directory (Minitest convention)
        bundler_available && ctx.root.join("test").is_dir()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Try to run with rake test first (standard Minitest approach)
        let mut cmd = Command::new("bundle");
        cmd.args(["exec", "rake", "test"]);

        // Add TESTOPTS if path specified
        if let Some(path) = &config.path {
            cmd.env("TESTOPTS", format!("--name={path}"));
        }

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn minitest: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "minitest");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run minitest: {e}"),
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Try JSON parsing first (minitest-reporters JsonReporter)
        if let Some(result) = try_parse_minitest_json(&stdout, total_time) {
            return result;
        }

        // Fall back to standard Minitest output parsing
        parse_minitest_output(&stdout, &stderr, total_time, output.status.success())
    }
}

/// Minitest JSON output structure (via minitest-reporters).
#[derive(Debug, Deserialize)]
pub(crate) struct MinitestJsonOutput {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tests: Vec<MinitestTest>,
    #[serde(default)]
    pub summary: Option<MinitestSummary>,
}

/// A single test from Minitest JSON.
#[derive(Debug, Deserialize)]
pub(crate) struct MinitestTest {
    pub name: String,
    #[serde(default)]
    pub classname: String,
    #[serde(default)]
    pub time: f64, // seconds
    pub status: String, // "pass", "fail", "error", "skip"
}

/// Summary from Minitest JSON.
#[derive(Debug, Deserialize)]
pub(crate) struct MinitestSummary {
    #[allow(dead_code)]
    pub total: u32,
    #[allow(dead_code)]
    pub passed: u32,
    pub failed: u32,
    #[allow(dead_code)]
    pub skipped: u32,
    #[allow(dead_code)]
    pub time: f64,
}

/// Try to parse JSON output from minitest-reporters.
fn try_parse_minitest_json(stdout: &str, total_time: Duration) -> Option<TestRunResult> {
    let json_str = find_json_object(stdout)?;
    let output: MinitestJsonOutput = serde_json::from_str(json_str).ok()?;

    let mut tests = Vec::new();

    for test in &output.tests {
        let duration = Duration::from_secs_f64(test.time);
        let name = if test.classname.is_empty() {
            test.name.clone()
        } else {
            format!("{}#{}", test.classname, test.name)
        };

        match test.status.as_str() {
            "pass" => tests.push(TestResult::passed(&name, duration)),
            "skip" => tests.push(TestResult::skipped(&name)),
            _ => tests.push(TestResult::failed(&name, duration)),
        }
    }

    let passed = output
        .summary
        .as_ref()
        .map(|s| s.failed == 0)
        .unwrap_or_else(|| output.status == "pass");

    let mut result = if passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    Some(result)
}

/// Parse standard Minitest text output.
///
/// Example output:
/// ```text
/// Run options: --seed 12345
///
/// # Running:
///
/// ..F.S....
///
/// Finished in 0.012345s, 100.0000 runs/s, 200.0000 assertions/s.
///
/// 10 runs, 20 assertions, 1 failures, 0 errors, 1 skips
/// ```
pub(crate) fn parse_minitest_output(
    stdout: &str,
    _stderr: &str,
    total_time: Duration,
    exit_success: bool,
) -> TestRunResult {
    // Look for the summary line: "X runs, Y assertions, Z failures, A errors, B skips"
    let mut runs = 0u32;
    let mut failures = 0u32;
    let mut errors = 0u32;
    let mut skips = 0u32;

    for line in stdout.lines() {
        if let Some(parsed) = parse_summary_line(line) {
            runs = parsed.runs;
            failures = parsed.failures;
            errors = parsed.errors;
            skips = parsed.skips;
            break;
        }
    }

    // If we found a summary line, use that for pass/fail determination
    let passed = if runs > 0 {
        failures == 0 && errors == 0
    } else {
        exit_success
    };

    // Create individual test results based on dot output
    let tests = parse_dot_output(stdout, runs, failures, errors, skips);

    let mut result = if passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

struct SummaryLine {
    runs: u32,
    failures: u32,
    errors: u32,
    skips: u32,
}

/// Parse the summary line like "10 runs, 20 assertions, 1 failures, 0 errors, 1 skips"
fn parse_summary_line(line: &str) -> Option<SummaryLine> {
    // Match both plural ("runs,") and singular ("run,") forms
    if !line.contains("runs,") && !line.contains("run,") {
        return None;
    }

    let mut runs = 0u32;
    let mut failures = 0u32;
    let mut errors = 0u32;
    let mut skips = 0u32;

    for part in line.split(',') {
        let part = part.trim();
        let words: Vec<&str> = part.split_whitespace().collect();
        if words.len() >= 2
            && let Ok(n) = words[0].parse::<u32>()
        {
            match words[1] {
                "runs" | "run" => runs = n,
                "failures" | "failure" => failures = n,
                "errors" | "error" => errors = n,
                "skips" | "skip" => skips = n,
                _ => {}
            }
        }
    }

    if runs > 0 {
        Some(SummaryLine {
            runs,
            failures,
            errors,
            skips,
        })
    } else {
        None
    }
}

/// Parse dot output (. = pass, F = fail, E = error, S = skip)
fn parse_dot_output(
    _stdout: &str,
    total_runs: u32,
    failures: u32,
    errors: u32,
    skips: u32,
) -> Vec<TestResult> {
    // If we have no runs, we can't create tests
    if total_runs == 0 {
        return Vec::new();
    }

    let mut tests = Vec::new();
    let passes = total_runs.saturating_sub(failures + errors + skips);

    // Create anonymous test results based on counts
    for i in 0..passes {
        tests.push(TestResult::passed(
            format!("test_{}", i + 1),
            Duration::ZERO,
        ));
    }
    for i in 0..failures {
        tests.push(TestResult::failed(
            format!("failed_test_{}", i + 1),
            Duration::ZERO,
        ));
    }
    for i in 0..errors {
        tests.push(TestResult::failed(
            format!("error_test_{}", i + 1),
            Duration::ZERO,
        ));
    }
    for i in 0..skips {
        tests.push(TestResult::skipped(format!("skipped_test_{}", i + 1)));
    }

    tests
}

#[cfg(test)]
#[path = "minitest_tests.rs"]
mod tests;
