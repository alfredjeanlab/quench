// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cargo test runner.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::coverage::collect_rust_coverage;
use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Cargo test runner for Rust projects.
pub struct CargoRunner;

impl TestRunner for CargoRunner {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check for Cargo.toml in project root
        ctx.root.join("Cargo.toml").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command - use standard cargo test output (stable Rust compatible)
        let mut cmd = Command::new("cargo");
        cmd.args(["test", "--release"]);

        // Set working directory
        let work_dir = config
            .path
            .as_ref()
            .map(|p| ctx.root.join(p))
            .unwrap_or_else(|| ctx.root.to_path_buf());
        cmd.current_dir(&work_dir);

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn and run with optional timeout
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn cargo: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "cargo");
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run cargo: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse human-readable output
        let mut result = parse_cargo_output(&stdout, total_time);

        // Check if cargo command itself failed (compilation error) with no tests parsed
        if !output.status.success() && result.tests.is_empty() && result.passed {
            let exit_code = output.status.code();
            let advice = categorize_cargo_error(&stderr, exit_code);
            let msg = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
            return TestRunResult::failed(total_time, format!("{advice}\n{msg}"));
        }

        // Collect coverage if requested
        if ctx.collect_coverage {
            let coverage = collect_rust_coverage(ctx.root, config.path.as_deref());
            result = result.with_collected_coverage(coverage, "rust");
        }

        result
    }
}

/// Parse cargo test human-readable output with optimizations.
///
/// Output format (examples):
/// ```text
/// test tests::test_add ... ok
/// test tests::test_fail ... FAILED
/// test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
/// test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
/// ```
///
/// Optimizations:
/// - Pre-allocates test vector based on line count hint
/// - Uses string slices to minimize allocations
/// - Inline helper for test line parsing
pub fn parse_cargo_output(stdout: &str, total_time: Duration) -> TestRunResult {
    // Pre-count lines starting with "test " for capacity hint
    let test_line_count = stdout
        .lines()
        .filter(|l| l.trim_start().starts_with("test "))
        .count();

    let mut tests = Vec::with_capacity(test_line_count);
    let mut suite_passed = true;

    for line in stdout.lines() {
        let line = line.trim();

        // Parse individual test results
        if let Some(rest) = line.strip_prefix("test ")
            && let Some((name, result)) = parse_test_line(rest)
        {
            match result {
                "ok" => tests.push(TestResult::passed(name, Duration::ZERO)),
                "ignored" => tests.push(TestResult::skipped(name)),
                _ => {
                    suite_passed = false;
                    tests.push(TestResult::failed(name, Duration::ZERO));
                }
            }
        }

        // Parse suite summary: "test result: FAILED. X passed; Y failed; ..."
        // This must be a separate check (not else if) because "test result:"
        // also matches the "test " prefix above.
        if line.starts_with("test result: ") && line.contains("FAILED") {
            suite_passed = false;
        }
    }

    let mut result = if suite_passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

/// Parse a test line after "test " prefix.
/// Returns (name, result) where result is "ok", "FAILED", or "ignored".
#[inline]
fn parse_test_line(rest: &str) -> Option<(&str, &str)> {
    // Format: "<name> ... ok" or "<name> ... FAILED" or "<name> ... ignored"
    let sep_pos = rest.rfind(" ... ")?;
    let name = &rest[..sep_pos];
    let result = &rest[sep_pos + 5..]; // Skip " ... "
    if result == "ok" || result == "FAILED" || result == "ignored" {
        Some((name, result))
    } else {
        None
    }
}

/// Categorize cargo test error for better error messaging.
///
/// Analyzes stderr output and exit code to provide actionable error messages.
pub fn categorize_cargo_error(stderr: &str, exit_code: Option<i32>) -> String {
    // Compilation error
    if stderr.contains("error[E") || stderr.contains("could not compile") {
        return "compilation failed - fix build errors first".to_string();
    }

    // Missing test binary
    if stderr.contains("no test target") || stderr.contains("can't find") {
        return "no tests found - check test file paths".to_string();
    }

    // Timeout (from signal) - SIGKILL is 137, timeout command uses 124
    if exit_code == Some(137) || exit_code == Some(124) {
        return "test timed out - check for infinite loops or deadlocks".to_string();
    }

    // Out of memory - SIGSEGV is 139
    if stderr.contains("out of memory") || exit_code == Some(139) {
        return "out of memory - reduce test parallelism or resource usage".to_string();
    }

    // Linker errors
    if stderr.contains("linker") || stderr.contains("undefined reference") {
        return "linking failed - check dependencies and feature flags".to_string();
    }

    // Generic failure
    "tests failed".to_string()
}

#[cfg(test)]
#[path = "cargo_tests.rs"]
mod tests;
