// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Custom command test runner.
//!
//! Executes arbitrary test commands and reports pass/fail based on exit code.
//! Does not provide per-test timing information.

use std::io::ErrorKind;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    RunnerContext, TestRunResult, TestRunner, handle_timeout_error, run_setup_or_fail,
    run_with_timeout,
};
use crate::config::TestSuiteConfig;

/// Custom command runner for arbitrary test commands.
pub struct CustomRunner;

impl TestRunner for CustomRunner {
    fn name(&self) -> &'static str {
        "custom"
    }

    fn available(&self, _ctx: &RunnerContext) -> bool {
        // Always available; command existence is checked at runtime
        true
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Custom runner requires a command
        let command = match &config.command {
            Some(cmd) => cmd,
            None => {
                return TestRunResult::failed(
                    Duration::ZERO,
                    "custom runner requires 'command' field",
                );
            }
        };

        run_setup_or_fail!(config, ctx);

        let start = Instant::now();

        // Execute command via shell
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to spawn custom command: {e}"),
                );
            }
        };

        let output = match run_with_timeout(child, config.timeout) {
            Ok(out) => out,
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                return handle_timeout_error(start.elapsed(), config.timeout, "custom");
            }
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to execute command: {e}"),
                );
            }
        };

        let total_time = start.elapsed();

        if output.status.success() {
            TestRunResult::passed(total_time)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let truncated: String = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
            let message = if truncated.is_empty() {
                format!("command failed with exit code {:?}", output.status.code())
            } else {
                truncated
            };
            TestRunResult::failed(total_time, message)
        }
    }
}

#[cfg(test)]
#[path = "custom_tests.rs"]
mod tests;
