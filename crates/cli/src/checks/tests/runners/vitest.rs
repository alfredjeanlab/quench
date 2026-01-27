// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Vitest test runner.
//!
//! Executes JavaScript/TypeScript tests using `vitest run --reporter=json`.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::js_coverage::collect_vitest_coverage;
use super::json_utils::find_json_object;
use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Vitest runner for JavaScript/TypeScript test suites.
pub struct VitestRunner;

impl TestRunner for VitestRunner {
    fn name(&self) -> &'static str {
        "vitest"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if vitest is available via npx
        let vitest_installed = Command::new("npx")
            .args(["vitest", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has package.json
        vitest_installed && ctx.root.join("package.json").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: npx vitest run --reporter=json
        let mut cmd = Command::new("npx");
        cmd.args(["vitest", "run", "--reporter=json"]);

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
                    format!("failed to spawn vitest: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "vitest");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run vitest: {e}"),
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut result = parse_vitest_json(&stdout, total_time);

        // Collect coverage if requested
        if ctx.collect_coverage {
            let coverage = collect_vitest_coverage(ctx.root, config.path.as_deref());
            result = result.with_collected_coverage(coverage, "javascript");
        }

        result
    }
}

/// Vitest JSON output structure.
#[derive(Debug, Deserialize)]
struct VitestOutput {
    #[serde(rename = "testResults", default)]
    test_results: Vec<VitestTestFile>,
}

/// A test file result from vitest.
#[derive(Debug, Deserialize)]
struct VitestTestFile {
    #[allow(dead_code)] // Deserialized from JSON but not directly used
    name: String,
    #[serde(rename = "assertionResults", default)]
    assertion_results: Vec<VitestAssertion>,
}

/// A single test assertion result.
#[derive(Debug, Deserialize)]
struct VitestAssertion {
    #[serde(rename = "fullName")]
    full_name: String,
    status: String,
    duration: Option<u64>, // milliseconds
}

/// Parse JSON output from vitest.
fn parse_vitest_json(stdout: &str, total_time: Duration) -> TestRunResult {
    // Try to find JSON in output (vitest may include other output before JSON)
    let json_str = find_json_object(stdout);

    let output: VitestOutput = match json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(o) => o,
        None => {
            // If we can't parse JSON, check if there's any indication of failure
            if stdout.contains("FAIL") || stdout.contains("Error") {
                return TestRunResult::failed(total_time, "vitest failed (no JSON output)");
            }
            return TestRunResult::passed(total_time);
        }
    };

    let mut tests = Vec::new();
    let mut all_passed = true;

    for file in output.test_results {
        for assertion in file.assertion_results {
            let duration = assertion
                .duration
                .map(Duration::from_millis)
                .unwrap_or(Duration::ZERO);

            let passed = assertion.status == "passed";
            if !passed {
                all_passed = false;
            }

            tests.push(if passed {
                TestResult::passed(&assertion.full_name, duration)
            } else {
                TestResult::failed(&assertion.full_name, duration)
            });
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

#[cfg(test)]
#[path = "vitest_tests.rs"]
mod tests;
