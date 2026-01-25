// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bats test runner.
//!
//! Executes shell tests using `bats --timing` and parses TAP output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, format_timeout_error, run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Bats test runner for shell script testing.
pub struct BatsRunner;

impl TestRunner for BatsRunner {
    fn name(&self) -> &'static str {
        "bats"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if bats is installed by running bats --version
        let bats_installed = Command::new("bats")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And test directory exists (if specified or default)
        bats_installed && ctx.root.join("tests").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Run setup command if specified
        if let Some(setup) = &config.setup
            && let Err(e) = super::run_setup_command(setup, ctx.root)
        {
            return TestRunResult::failed(Duration::ZERO, e);
        }

        let start = Instant::now();

        // Build command: bats --timing <path>
        let mut cmd = Command::new("bats");
        cmd.arg("--timing");

        // Add test path (default: tests/)
        let test_path = config.path.as_deref().unwrap_or("tests/");
        cmd.arg(test_path);

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn bats: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                let timeout_msg = config
                    .timeout
                    .map(|t| format_timeout_error("bats", t))
                    .unwrap_or_else(|| "timed out".to_string());
                return TestRunResult::failed(start.elapsed(), timeout_msg);
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run bats: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_tap_output(&stdout, total_time)
    }
}

/// Parse TAP output from bats --timing.
fn parse_tap_output(stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut all_passed = true;

    for line in stdout.lines() {
        let line = line.trim();

        // Skip plan line (1..N) and comments (# ...)
        if line.starts_with("1..") || line.starts_with('#') {
            continue;
        }

        // Parse test result: "ok N description" or "not ok N description"
        if let Some(result) = parse_tap_line(line) {
            if !result.passed {
                all_passed = false;
            }
            tests.push(result);
        }
    }

    let mut result = if all_passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

/// Parse a single TAP result line.
fn parse_tap_line(line: &str) -> Option<TestResult> {
    let (passed, rest) = if let Some(rest) = line.strip_prefix("ok ") {
        (true, rest)
    } else if let Some(rest) = line.strip_prefix("not ok ") {
        (false, rest)
    } else {
        return None;
    };

    // Skip test number, get description
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ');

    // Extract timing if present: "description in Xms"
    let (name, duration) = extract_timing(rest);

    Some(if passed {
        TestResult::passed(name, duration)
    } else {
        TestResult::failed(name, duration)
    })
}

/// Extract timing from TAP description suffix.
fn extract_timing(desc: &str) -> (String, Duration) {
    // Pattern: "description in 45ms" or "description in 1.234s"
    if let Some(idx) = desc.rfind(" in ") {
        let timing_part = &desc[idx + 4..];
        if let Some(duration) = parse_duration(timing_part) {
            return (desc[..idx].to_string(), duration);
        }
    }
    (desc.to_string(), Duration::ZERO)
}

/// Parse duration string from bats timing output.
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<f64>().ok().map(Duration::from_secs_f64)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "bats_tests.rs"]
mod tests;
