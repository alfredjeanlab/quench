// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Jest test runner.
//!
//! Executes JavaScript/TypeScript tests using `jest --json`.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, format_timeout_error, run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Jest runner for JavaScript/TypeScript test suites.
pub struct JestRunner;

impl TestRunner for JestRunner {
    fn name(&self) -> &'static str {
        "jest"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if jest is available via npx
        let jest_installed = Command::new("npx")
            .args(["jest", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has package.json
        jest_installed && ctx.root.join("package.json").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Run setup command if specified
        if let Some(setup) = &config.setup
            && let Err(e) = super::run_setup_command(setup, ctx.root)
        {
            return TestRunResult::failed(Duration::ZERO, e);
        }

        let start = Instant::now();

        // Build command: npx jest --json
        let mut cmd = Command::new("npx");
        cmd.args(["jest", "--json"]);

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
                    format!("failed to spawn jest: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                let timeout_msg = config
                    .timeout
                    .map(|t| format_timeout_error("jest", t))
                    .unwrap_or_else(|| "timed out".to_string());
                return TestRunResult::failed(start.elapsed(), timeout_msg);
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run jest: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_jest_json(&stdout, total_time)
    }
}

/// Jest JSON output structure.
#[derive(Debug, Deserialize)]
pub(crate) struct JestOutput {
    pub success: bool,
    #[serde(rename = "testResults", default)]
    pub test_results: Vec<JestTestFile>,
}

/// A test file result from jest.
#[derive(Debug, Deserialize)]
pub(crate) struct JestTestFile {
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "assertionResults", default)]
    pub assertion_results: Vec<JestAssertion>,
}

/// A single test assertion result.
#[derive(Debug, Deserialize)]
pub(crate) struct JestAssertion {
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub status: String,
    pub duration: Option<u64>, // milliseconds
}

/// Parse JSON output from jest.
pub(crate) fn parse_jest_json(stdout: &str, total_time: Duration) -> TestRunResult {
    // Try to find JSON in output (jest may include other output before JSON)
    let json_str = find_json_object(stdout);

    let output: JestOutput = match json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(o) => o,
        None => {
            // If we can't parse JSON, check if there's any indication of failure
            if stdout.contains("FAIL") || stdout.contains("Error") {
                return TestRunResult::failed(total_time, "jest failed (no JSON output)");
            }
            return TestRunResult::passed(total_time);
        }
    };

    let mut tests = Vec::new();

    for file in output.test_results {
        for assertion in file.assertion_results {
            let duration = assertion
                .duration
                .map(Duration::from_millis)
                .unwrap_or(Duration::ZERO);

            let passed = assertion.status == "passed";

            tests.push(if passed {
                TestResult::passed(&assertion.full_name, duration)
            } else {
                TestResult::failed(&assertion.full_name, duration)
            });
        }
    }

    let mut result = if output.success {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

/// Find the first JSON object in the output.
fn find_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0;
    let mut end = start;

    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > start {
        Some(&s[start..end])
    } else {
        None
    }
}

#[cfg(test)]
#[path = "jest_tests.rs"]
mod tests;
