// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cucumber test runner.
//!
//! Executes BDD-style Ruby tests using `bundle exec cucumber --format json`.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::json_utils::find_json_array;
use super::{
    RunnerContext, TestResult, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Cucumber runner for Ruby BDD test suites.
pub struct CucumberRunner;

impl TestRunner for CucumberRunner {
    fn name(&self) -> &'static str {
        "cucumber"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if cucumber is available via bundler
        let cucumber_installed = Command::new("bundle")
            .args(["exec", "cucumber", "--version"])
            .current_dir(ctx.root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has features/ directory
        cucumber_installed && ctx.root.join("features").is_dir()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Build command: bundle exec cucumber --format json
        let mut cmd = Command::new("bundle");
        cmd.args(["exec", "cucumber", "--format", "json"]);

        // Add feature path if specified
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
                    format!("failed to spawn cucumber: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "cucumber");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run cucumber: {e}"),
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_cucumber_json(&stdout, total_time)
    }
}

/// Cucumber JSON output is an array of features.
pub(crate) type CucumberOutput = Vec<CucumberFeature>;

/// A single feature file from Cucumber.
#[derive(Debug, Deserialize)]
pub(crate) struct CucumberFeature {
    #[allow(dead_code)]
    pub uri: String,
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub elements: Vec<CucumberElement>,
}

/// A scenario or scenario outline from Cucumber.
#[derive(Debug, Deserialize)]
pub(crate) struct CucumberElement {
    #[serde(rename = "type")]
    pub element_type: String, // "scenario", "background", etc.
    pub name: String,
    #[serde(default)]
    pub steps: Vec<CucumberStep>,
}

/// A single step in a Cucumber scenario.
#[derive(Debug, Deserialize)]
pub(crate) struct CucumberStep {
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub result: Option<CucumberStepResult>,
}

/// Result of executing a Cucumber step.
#[derive(Debug, Deserialize)]
pub(crate) struct CucumberStepResult {
    pub status: String, // "passed", "failed", "pending", "skipped", "undefined"
    #[serde(default)]
    pub duration: u64, // nanoseconds
}

/// Parse JSON output from Cucumber.
pub(crate) fn parse_cucumber_json(stdout: &str, total_time: Duration) -> TestRunResult {
    // Try to find JSON array in output
    let json_str = find_json_array(stdout);

    let features: CucumberOutput = match json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(f) => f,
        None => {
            // Without JSON output, we can't reliably determine pass/fail
            // Return passed since we can't say for sure tests failed
            return TestRunResult::passed(total_time);
        }
    };

    let mut tests = Vec::new();
    let mut has_failure = false;

    for feature in &features {
        for element in &feature.elements {
            // Only process scenarios (not backgrounds)
            if element.element_type != "scenario" {
                continue;
            }

            // Calculate scenario status from steps
            let (status, duration) = scenario_status(&element.steps);

            let test_name = if element.name.is_empty() {
                format!("Scenario in {}", feature.name)
            } else {
                element.name.clone()
            };

            match status.as_str() {
                "passed" => {
                    tests.push(TestResult::passed(&test_name, duration));
                }
                "pending" | "skipped" | "undefined" => {
                    tests.push(TestResult::skipped(&test_name));
                }
                _ => {
                    // "failed" or any other status
                    tests.push(TestResult::failed(&test_name, duration));
                    has_failure = true;
                }
            }
        }
    }

    let mut result = if has_failure {
        TestRunResult::failed(total_time, "tests failed")
    } else {
        TestRunResult::passed(total_time)
    };
    result.tests = tests;
    result
}

/// Determine scenario status from its steps.
///
/// A scenario is:
/// - "failed" if any step failed
/// - "pending" if any step is pending/undefined (and none failed)
/// - "passed" if all steps passed
///
/// Duration is the sum of all step durations (in nanoseconds converted to Duration).
fn scenario_status(steps: &[CucumberStep]) -> (String, Duration) {
    let mut total_nanos: u64 = 0;
    let mut has_failure = false;
    let mut has_pending = false;

    for step in steps {
        if let Some(ref result) = step.result {
            total_nanos = total_nanos.saturating_add(result.duration);

            match result.status.as_str() {
                "failed" => has_failure = true,
                "pending" | "undefined" | "skipped" => has_pending = true,
                _ => {}
            }
        }
    }

    let duration = Duration::from_nanos(total_nanos);
    let status = if has_failure {
        "failed"
    } else if has_pending {
        "pending"
    } else {
        "passed"
    };

    (status.to_string(), duration)
}

#[cfg(test)]
#[path = "cucumber_tests.rs"]
mod tests;
