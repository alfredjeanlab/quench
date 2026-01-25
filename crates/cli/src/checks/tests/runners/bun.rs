// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Bun test runner.
//!
//! Executes JavaScript/TypeScript tests using `bun test --reporter=json`.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::jest::parse_jest_json;
use super::{RunnerContext, TestRunResult, TestRunner, format_timeout_error, run_with_timeout};
use crate::config::TestSuiteConfig;

/// Bun test runner for JavaScript/TypeScript test suites.
pub struct BunRunner;

impl TestRunner for BunRunner {
    fn name(&self) -> &'static str {
        "bun"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if bun is installed
        let bun_installed = Command::new("bun")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        // And project has package.json
        bun_installed && ctx.root.join("package.json").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Run setup command if specified
        if let Some(setup) = &config.setup
            && let Err(e) = super::run_setup_command(setup, ctx.root)
        {
            return TestRunResult::failed(Duration::ZERO, e);
        }

        let start = Instant::now();

        // Build command: bun test --reporter=json
        let mut cmd = Command::new("bun");
        cmd.args(["test", "--reporter=json"]);

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
                return TestRunResult::failed(start.elapsed(), format!("failed to spawn bun: {e}"));
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                let timeout_msg = config
                    .timeout
                    .map(|t| format_timeout_error("bun", t))
                    .unwrap_or_else(|| "timed out".to_string());
                return TestRunResult::failed(start.elapsed(), timeout_msg);
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), format!("failed to run bun: {e}"));
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Bun uses the same JSON format as Jest
        parse_jest_json(&stdout, total_time)
    }
}

#[cfg(test)]
#[path = "bun_tests.rs"]
mod tests;
