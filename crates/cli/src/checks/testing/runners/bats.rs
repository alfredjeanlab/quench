// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bats test runner.
//!
//! Executes shell tests using `bats --timing` and parses TAP output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    CoverageResult, InstrumentedBuild, RunnerContext, TestResult, TestRunResult, TestRunner,
    build_instrumented, collect_instrumented_coverage, collect_shell_coverage, coverage_env,
    handle_timeout_error, kcov_available, resolve_targets, run_setup_or_fail, run_with_timeout,
    rust_binary_names, shell_script_files,
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
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // If collecting Rust binary coverage, build instrumented binaries FIRST
        // so they're available when bats runs
        let rust_build = if ctx.collect_coverage && !config.targets.is_empty() {
            prepare_rust_binary_coverage(config, ctx)
        } else {
            None
        };

        // Build command: bats --timing <path>
        let mut cmd = Command::new("bats");
        cmd.arg("--timing");

        // Add test path (default: tests/)
        let test_path = config.path.as_deref().unwrap_or("tests/");
        cmd.arg(test_path);

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Add coverage environment if we built instrumented binaries
        if let Some(ref build_result) = rust_build {
            cmd.envs(coverage_env(build_result));
        }

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
                return handle_timeout_error(start.elapsed(), config.timeout, "bats");
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run bats: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut result = parse_tap_output(&stdout, total_time);

        // Collect coverage if requested and targets specified
        if ctx.collect_coverage && !config.targets.is_empty() {
            // Shell coverage via kcov
            if let Some(shell_coverage) = collect_bats_shell_coverage(config, ctx) {
                result = result.with_collected_coverage(shell_coverage, "shell");
            }

            // Rust binary coverage from the instrumented build
            if let Some(ref build_result) = rust_build {
                let coverage = collect_instrumented_coverage(build_result, ctx.root);
                if coverage.success && coverage.line_coverage.is_some() {
                    result = result.with_collected_coverage(coverage, "rust");
                }
            }
        }

        result
    }
}

/// Prepare instrumented Rust binaries for coverage collection.
/// Returns the build result if there are Rust binary targets to instrument.
fn prepare_rust_binary_coverage(
    config: &TestSuiteConfig,
    ctx: &RunnerContext,
) -> Option<InstrumentedBuild> {
    let resolved = resolve_targets(&config.targets, ctx.config, ctx.root).ok()?;
    let binaries = rust_binary_names(&resolved);

    if binaries.is_empty() {
        return None;
    }

    // Build instrumented binaries
    build_instrumented(&binaries, ctx.root).ok()
}

/// Collect shell script coverage for BATS tests via kcov.
fn collect_bats_shell_coverage(
    config: &TestSuiteConfig,
    ctx: &RunnerContext,
) -> Option<CoverageResult> {
    if !kcov_available() {
        return None;
    }

    // Resolve targets to find shell scripts
    let resolved = resolve_targets(&config.targets, ctx.config, ctx.root).ok()?;
    let scripts = shell_script_files(&resolved);

    if scripts.is_empty() {
        return None;
    }

    // Build bats command for kcov to wrap
    let test_path = config.path.as_deref().unwrap_or("tests/");
    let test_command = vec!["bats".to_string(), test_path.to_string()];

    let coverage = collect_shell_coverage(&scripts, &test_command, ctx.root);
    if coverage.success { Some(coverage) } else { None }
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
