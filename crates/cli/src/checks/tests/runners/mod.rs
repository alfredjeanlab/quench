// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test runner framework.
//!
//! Provides abstractions for executing test suites and collecting metrics.

mod result;
mod stub;

pub use result::{TestResult, TestRunResult};
pub use stub::StubRunner;

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::config::TestSuiteConfig;

/// List of known runner names.
pub const RUNNER_NAMES: &[&str] = &[
    "cargo", "go", "pytest", "vitest", "bun", "jest", "bats", "custom",
];

/// Context passed to test runners during execution.
pub struct RunnerContext<'a> {
    /// Project root directory.
    pub root: &'a Path,
    /// Whether running in CI mode.
    pub ci_mode: bool,
    /// Whether to collect coverage.
    pub collect_coverage: bool,
}

/// Trait for pluggable test runners.
///
/// Implementors execute tests and return timing/coverage metrics.
pub trait TestRunner: Send + Sync {
    /// Runner name (e.g., "cargo", "bats").
    fn name(&self) -> &'static str;

    /// Check if this runner can handle the given configuration.
    ///
    /// Returns false if required tools are not installed.
    fn available(&self, ctx: &RunnerContext) -> bool;

    /// Execute the test suite and return results.
    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult;
}

/// Get all available runners.
pub fn all_runners() -> Vec<Arc<dyn TestRunner>> {
    vec![
        // Stub implementations for now - concrete runners in later phases
        Arc::new(StubRunner::new("cargo")),
        Arc::new(StubRunner::new("go")),
        Arc::new(StubRunner::new("pytest")),
        Arc::new(StubRunner::new("vitest")),
        Arc::new(StubRunner::new("bun")),
        Arc::new(StubRunner::new("jest")),
        Arc::new(StubRunner::new("bats")),
    ]
}

/// Get a runner by name.
pub fn get_runner(name: &str) -> Option<Arc<dyn TestRunner>> {
    all_runners().into_iter().find(|r| r.name() == name)
}

/// Filter suites based on CI mode.
///
/// In fast mode: skip suites with `ci = true`
/// In CI mode: run all suites
pub fn filter_suites_for_mode(suites: &[TestSuiteConfig], ci_mode: bool) -> Vec<&TestSuiteConfig> {
    suites.iter().filter(|s| ci_mode || !s.ci).collect()
}

/// Execute a setup command before running tests.
///
/// Returns Ok(()) on success, Err(message) on failure.
pub fn run_setup_command(setup: &str, root: &Path) -> Result<(), String> {
    // Use shell to handle complex commands
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", setup])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    } else {
        Command::new("sh")
            .args(["-c", setup])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    };

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let truncated: String = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
            Err(format!("setup command failed: {setup}\n{truncated}"))
        }
        Err(e) => Err(format!("failed to execute setup: {e}")),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
