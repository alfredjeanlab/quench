// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go test runner.
//!
//! Executes Go tests using `go test -json` and parses NDJSON output.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::go_coverage::collect_go_coverage;
use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Go test runner for Go projects.
pub struct GoRunner;

impl TestRunner for GoRunner {
    fn name(&self) -> &'static str {
        "go"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if go is installed
        let go_installed = Command::new("go")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has go.mod
        go_installed && ctx.root.join("go.mod").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: go test -json <path>
        let mut cmd = Command::new("go");
        cmd.args(["test", "-json"]);

        // Add test path (default: ./...)
        let test_path = config.path.as_deref().unwrap_or("./...");
        cmd.arg(test_path);

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn go test: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "go");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run go test: {e}"),
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut result = parse_go_json(&stdout, total_time);

        // Collect coverage if requested
        if ctx.collect_coverage {
            let coverage = collect_go_coverage(ctx.root, config.path.as_deref());
            result = result.with_collected_coverage(coverage, "go");
        }

        result
    }
}

/// A single event from go test -json output.
#[derive(Debug, Deserialize)]
struct GoTestEvent {
    #[serde(rename = "Action")]
    action: String,
    #[serde(rename = "Package")]
    package: Option<String>,
    #[serde(rename = "Test")]
    test: Option<String>,
    #[serde(rename = "Elapsed")]
    elapsed: Option<f64>,
}

/// Parse NDJSON output from go test -json.
fn parse_go_json(stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut all_passed = true;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse JSON event
        let event: GoTestEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue, // Skip malformed lines
        };

        // Only process pass/fail actions with test names
        let Some(test_name) = &event.test else {
            continue;
        };

        match event.action.as_str() {
            "pass" => {
                let duration = event
                    .elapsed
                    .map(Duration::from_secs_f64)
                    .unwrap_or(Duration::ZERO);
                let full_name = format_test_name(event.package.as_deref(), test_name);
                tests.push(TestResult::passed(full_name, duration));
            }
            "fail" => {
                let duration = event
                    .elapsed
                    .map(Duration::from_secs_f64)
                    .unwrap_or(Duration::ZERO);
                let full_name = format_test_name(event.package.as_deref(), test_name);
                tests.push(TestResult::failed(full_name, duration));
                all_passed = false;
            }
            _ => {} // Ignore other actions (run, output, etc.)
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

/// Format test name with optional package prefix.
fn format_test_name(package: Option<&str>, test: &str) -> String {
    match package {
        Some(pkg) => format!("{pkg}/{test}"),
        None => test.to_string(),
    }
}

#[cfg(test)]
#[path = "go_tests.rs"]
mod tests;
