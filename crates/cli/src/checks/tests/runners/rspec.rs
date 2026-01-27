// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! RSpec test runner.
//!
//! Executes Ruby tests using `bundle exec rspec --format json`.

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

/// RSpec runner for Ruby test suites.
pub struct RspecRunner;

impl TestRunner for RspecRunner {
    fn name(&self) -> &'static str {
        "rspec"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if rspec is available via bundler
        let rspec_installed = Command::new("bundle")
            .args(["exec", "rspec", "--version"])
            .current_dir(ctx.root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has spec/ directory
        rspec_installed && ctx.root.join("spec").is_dir()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: bundle exec rspec --format json
        let mut cmd = Command::new("bundle");
        cmd.args(["exec", "rspec", "--format", "json"]);

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
                    format!("failed to spawn rspec: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "rspec");
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run rspec: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_rspec_json(&stdout, total_time)
    }
}

/// RSpec JSON output structure.
#[derive(Debug, Deserialize)]
pub(crate) struct RspecOutput {
    #[serde(default)]
    pub examples: Vec<RspecExample>,
    #[serde(default)]
    pub summary: Option<RspecSummary>,
}

/// A single test example from RSpec.
#[derive(Debug, Deserialize)]
pub(crate) struct RspecExample {
    pub full_description: String,
    pub status: String, // "passed", "failed", "pending"
    #[serde(default)]
    pub run_time: f64, // seconds
}

/// Summary statistics from RSpec.
#[derive(Debug, Deserialize)]
pub(crate) struct RspecSummary {
    #[allow(dead_code)]
    pub duration: f64,
    #[allow(dead_code)]
    pub example_count: u32,
    pub failure_count: u32,
    #[allow(dead_code)]
    pub pending_count: u32,
}

/// Parse JSON output from RSpec.
pub(crate) fn parse_rspec_json(stdout: &str, total_time: Duration) -> TestRunResult {
    // Try to find JSON in output (rspec may include other output before JSON)
    let json_str = find_json_object(stdout);

    let output: RspecOutput = match json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(o) => o,
        None => {
            // If we can't parse JSON, check for failure indicators
            if stdout.contains("Failures:") || stdout.contains("Error") {
                return TestRunResult::failed(total_time, "rspec failed (no JSON output)");
            }
            return TestRunResult::passed(total_time);
        }
    };

    let mut tests = Vec::new();

    for example in &output.examples {
        let duration = Duration::from_secs_f64(example.run_time);

        match example.status.as_str() {
            "passed" => {
                tests.push(TestResult::passed(&example.full_description, duration));
            }
            "pending" => {
                tests.push(TestResult::skipped(&example.full_description));
            }
            _ => {
                // "failed" or any other status
                tests.push(TestResult::failed(&example.full_description, duration));
            }
        }
    }

    // Determine overall pass/fail status
    let passed = output
        .summary
        .as_ref()
        .map(|s| s.failure_count == 0)
        .unwrap_or_else(|| tests.iter().all(|t| t.passed));

    let mut result = if passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

#[cfg(test)]
#[path = "rspec_tests.rs"]
mod tests;
