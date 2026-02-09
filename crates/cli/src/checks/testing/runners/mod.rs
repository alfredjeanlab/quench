// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test runner framework.
//!
//! Provides abstractions for executing test suites and collecting metrics.

mod bats;
mod bun;
mod cargo;
mod coverage;
mod cucumber;
mod custom;
mod go;
mod go_coverage;
mod go_detect;
mod instrumented;
mod jest;
mod js_coverage;
mod js_detect;
mod json_utils;
mod kcov;
mod minitest;
mod py_detect;
mod pytest;
mod python_coverage;
mod result;
mod rspec;
mod ruby_coverage;
mod rust_detect;
mod targets;
mod unittest;
mod vitest;

pub use bats::BatsRunner;
pub use bun::BunRunner;
pub use cargo::{CargoRunner, categorize_cargo_error, parse_cargo_output};
pub use coverage::CoverageResult;
pub use cucumber::CucumberRunner;
pub use custom::CustomRunner;
pub use go::GoRunner;
pub use go_coverage::{collect_go_coverage, go_available};
pub use go_detect::{
    GoDetectionResult, GoDetectionSource, GoRunner as GoDetectRunner, detect_go_runner,
};
pub use instrumented::{
    InstrumentedBuild, build_instrumented, collect_instrumented_coverage, coverage_env,
};
pub use jest::JestRunner;
pub use js_coverage::{collect_bun_coverage, collect_jest_coverage, collect_vitest_coverage};
pub use js_detect::{DetectionResult, DetectionSource, JsRunner, detect_js_runner};
pub use kcov::{collect_shell_coverage, kcov_available};
pub use minitest::MinitestRunner;
pub use py_detect::{PyDetectionResult, PyDetectionSource, PyRunner, detect_py_runner};
pub use pytest::PytestRunner;
pub use python_coverage::collect_python_coverage;
pub use result::{TestResult, TestRunResult};
pub use rspec::RspecRunner;
pub use ruby_coverage::collect_ruby_coverage;
pub use rust_detect::{RustDetectionResult, RustDetectionSource, RustRunner, detect_rust_runner};
pub use targets::{
    ResolvedTarget, TargetResolutionError, is_glob_pattern, resolve_target, resolve_targets,
    rust_binary_names, shell_script_files,
};
pub use unittest::UnittestRunner;
pub use vitest::VitestRunner;

use std::collections::HashMap;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::TestSuiteConfig;

// =============================================================================
// Runner Helper Macros
// =============================================================================

/// Execute setup command and return early on failure.
///
/// Usage:
/// ```ignore
/// run_setup_or_fail!(config, ctx);
/// ```
#[macro_export]
macro_rules! run_setup_or_fail {
    ($config:expr, $ctx:expr) => {
        if let Some(setup) = &$config.setup {
            if let Err(e) = $crate::checks::testing::runners::run_setup_command(setup, $ctx.root) {
                return $crate::checks::testing::runners::TestRunResult::failed(
                    std::time::Duration::ZERO,
                    e,
                );
            }
        }
    };
}

pub use run_setup_or_fail;

/// List of known runner names.
pub const RUNNER_NAMES: &[&str] = &[
    "cargo", "go", "pytest", "unittest", "vitest", "bun", "jest", "bats", "rspec", "minitest",
    "cucumber", "custom",
];

/// Context passed to test runners during execution.
pub struct RunnerContext<'a> {
    /// Project root directory.
    pub root: &'a Path,
    /// Whether running in CI mode.
    pub ci_mode: bool,
    /// Whether to collect coverage.
    pub collect_coverage: bool,
    /// Project configuration (for target resolution).
    pub config: &'a crate::config::Config,
    /// Whether verbose diagnostic output is enabled.
    pub verbose: bool,
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
        Arc::new(CargoRunner),
        Arc::new(BatsRunner),
        Arc::new(GoRunner),
        Arc::new(PytestRunner),
        Arc::new(UnittestRunner),
        Arc::new(VitestRunner),
        Arc::new(BunRunner),
        Arc::new(JestRunner),
        Arc::new(RspecRunner),
        Arc::new(MinitestRunner),
        Arc::new(CucumberRunner),
        Arc::new(CustomRunner),
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

// =============================================================================
// Coverage Aggregation
// =============================================================================

/// Aggregated coverage across all test suites.
#[derive(Debug, Default)]
pub struct AggregatedCoverage {
    /// Rust coverage result (merged from all Rust sources).
    pub rust: Option<CoverageResult>,
    /// Shell coverage result (merged from all shell sources).
    pub shell: Option<CoverageResult>,
    /// Go coverage result (merged from all Go sources).
    pub go: Option<CoverageResult>,
    /// JavaScript coverage result (merged from all JS sources).
    pub javascript: Option<CoverageResult>,
    /// Ruby coverage result (merged from all Ruby sources).
    pub ruby: Option<CoverageResult>,
    /// Python coverage result (merged from all Python sources).
    pub python: Option<CoverageResult>,
}

impl AggregatedCoverage {
    /// Merge coverage from a suite into the aggregate.
    pub fn merge_rust(&mut self, result: CoverageResult) {
        self.rust = Some(match self.rust.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Merge shell coverage from a suite into the aggregate.
    pub fn merge_shell(&mut self, result: CoverageResult) {
        self.shell = Some(match self.shell.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Merge Go coverage from a suite into the aggregate.
    pub fn merge_go(&mut self, result: CoverageResult) {
        self.go = Some(match self.go.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Merge JavaScript coverage from a suite into the aggregate.
    pub fn merge_javascript(&mut self, result: CoverageResult) {
        self.javascript = Some(match self.javascript.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Merge Ruby coverage from a suite into the aggregate.
    pub fn merge_ruby(&mut self, result: CoverageResult) {
        self.ruby = Some(match self.ruby.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Merge Python coverage from a suite into the aggregate.
    pub fn merge_python(&mut self, result: CoverageResult) {
        self.python = Some(match self.python.take() {
            Some(existing) => merge_coverage_results(existing, result),
            None => result,
        });
    }

    /// Convert to a language -> percentage map for metrics.
    pub fn to_coverage_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        if let Some(ref rust) = self.rust
            && let Some(pct) = rust.line_coverage
        {
            map.insert("rust".to_string(), pct);
        }
        if let Some(ref shell) = self.shell
            && let Some(pct) = shell.line_coverage
        {
            map.insert("shell".to_string(), pct);
        }
        if let Some(ref go) = self.go
            && let Some(pct) = go.line_coverage
        {
            map.insert("go".to_string(), pct);
        }
        if let Some(ref javascript) = self.javascript
            && let Some(pct) = javascript.line_coverage
        {
            map.insert("javascript".to_string(), pct);
        }
        if let Some(ref ruby) = self.ruby
            && let Some(pct) = ruby.line_coverage
        {
            map.insert("ruby".to_string(), pct);
        }
        if let Some(ref python) = self.python
            && let Some(pct) = python.line_coverage
        {
            map.insert("python".to_string(), pct);
        }
        map
    }

    /// Check if any coverage data is available.
    pub fn has_data(&self) -> bool {
        self.rust.as_ref().is_some_and(|r| r.line_coverage.is_some())
            || self.shell.as_ref().is_some_and(|r| r.line_coverage.is_some())
            || self.go.as_ref().is_some_and(|r| r.line_coverage.is_some())
            || self.javascript.as_ref().is_some_and(|r| r.line_coverage.is_some())
            || self.ruby.as_ref().is_some_and(|r| r.line_coverage.is_some())
            || self.python.as_ref().is_some_and(|r| r.line_coverage.is_some())
    }
}

/// Merge two coverage results by taking max coverage per file.
pub fn merge_coverage_results(a: CoverageResult, b: CoverageResult) -> CoverageResult {
    let mut files = a.files;
    for (path, coverage) in b.files {
        files
            .entry(path)
            .and_modify(|existing| {
                if coverage > *existing {
                    *existing = coverage
                }
            })
            .or_insert(coverage);
    }

    // Merge packages by taking max coverage per package
    let mut packages = a.packages;
    for (pkg, coverage) in b.packages {
        packages
            .entry(pkg)
            .and_modify(|existing| {
                if coverage > *existing {
                    *existing = coverage
                }
            })
            .or_insert(coverage);
    }

    // Recalculate overall percentage from merged files
    let total_coverage = if files.is_empty() {
        // If no per-file data, try to merge overall coverage
        match (a.line_coverage, b.line_coverage) {
            (Some(a_cov), Some(b_cov)) => Some(a_cov.max(b_cov)),
            (Some(cov), None) | (None, Some(cov)) => Some(cov),
            (None, None) => None,
        }
    } else {
        Some(files.values().sum::<f64>() / files.len() as f64)
    };

    CoverageResult {
        success: a.success && b.success,
        error: a.error.or(b.error),
        duration: a.duration + b.duration,
        line_coverage: total_coverage,
        files,
        packages,
    }
}

// =============================================================================
// Timeout Support
// =============================================================================

/// Format a timeout error message with runner-specific advice.
pub fn format_timeout_error(runner: &str, timeout: Duration) -> String {
    let base = format!("timed out after {:?}", timeout);
    let advice = match runner {
        "cargo" => "check for infinite loops or deadlocks",
        "bats" => "check for infinite loops in shell scripts",
        "pytest" => "check for slow tests or missing mocks",
        "go" => "check for goroutine leaks or infinite loops",
        "jest" | "vitest" | "bun" => "check for unresolved promises or infinite loops",
        "rspec" | "minitest" | "cucumber" => "check for slow database queries or missing mocks",
        _ => "check for slow or hanging tests",
    };
    format!("{} - {}", base, advice)
}

/// Handle a timeout error by returning an appropriate `TestRunResult`.
///
/// This is a helper function for the common timeout error handling pattern
/// in test runners.
pub fn handle_timeout_error(
    elapsed: Duration,
    timeout: Option<Duration>,
    runner_name: &str,
) -> TestRunResult {
    let msg = timeout
        .map(|t| format_timeout_error(runner_name, t))
        .unwrap_or_else(|| "timed out".to_string());
    TestRunResult::failed(elapsed, msg)
}

/// Run a child process with an optional timeout.
///
/// If timeout is None, waits indefinitely.
/// If timeout expires, kills the process and returns a TimedOut error.
pub fn run_with_timeout(mut child: Child, timeout: Option<Duration>) -> io::Result<Output> {
    match timeout {
        Some(t) => {
            let start = Instant::now();
            let poll_interval = Duration::from_millis(50);

            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process finished - collect output
                        let stdout = child
                            .stdout
                            .take()
                            .map(|mut s| {
                                let mut buf = Vec::new();
                                s.read_to_end(&mut buf).ok();
                                buf
                            })
                            .unwrap_or_default();
                        let stderr = child
                            .stderr
                            .take()
                            .map(|mut s| {
                                let mut buf = Vec::new();
                                s.read_to_end(&mut buf).ok();
                                buf
                            })
                            .unwrap_or_default();
                        return Ok(Output { status, stdout, stderr });
                    }
                    Ok(None) => {
                        // Process still running
                        if start.elapsed() > t {
                            // Timeout - kill the process
                            child.kill().ok();
                            child.wait().ok();
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                format!("command timed out after {:?}", t),
                            ));
                        }
                        std::thread::sleep(poll_interval);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        None => child.wait_with_output(),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
