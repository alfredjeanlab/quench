// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test suite execution and result types.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::check::CheckContext;
use crate::config::TestSuiteConfig;

use super::runners::{RunnerContext, filter_suites_for_mode, get_runner, run_setup_command};

/// Format milliseconds as a human-friendly duration string.
///
/// Returns e.g. "450ms" for small values, "3.2s" for values over 3000ms.
fn format_duration_ms(ms: u64) -> String {
    if ms > 3000 { format!("{:.1}s", ms as f64 / 1000.0) } else { format!("{}ms", ms) }
}

/// Aggregated results from all test suites.
#[derive(Debug, Default)]
pub struct SuiteResults {
    /// Whether all suites passed.
    pub passed: bool,
    /// Individual suite results.
    pub suites: Vec<SuiteResult>,
}

/// Top-level aggregated metrics across all suites.
#[derive(Debug)]
pub struct AggregatedMetrics {
    /// Total tests across all suites.
    pub test_count: usize,
    /// Total execution time in milliseconds.
    pub total_ms: u64,
    /// Weighted average time per test in milliseconds.
    pub avg_ms: Option<u64>,
    /// Maximum test time in milliseconds (across all suites).
    pub max_ms: Option<u64>,
    /// Name of the slowest test (across all suites).
    pub max_test: Option<String>,
}

impl SuiteResults {
    /// Calculate aggregated timing metrics across all suites.
    pub fn aggregated_metrics(&self) -> AggregatedMetrics {
        let test_count: usize = self.suites.iter().map(|s| s.test_count).sum();

        let total_ms: u64 = self.suites.iter().map(|s| s.total_ms).sum();

        // Weighted average: sum of (suite_avg * suite_count) / total_count
        let avg_ms = if test_count > 0 {
            let weighted_sum: u64 = self
                .suites
                .iter()
                .filter_map(|s| s.avg_ms.map(|avg| avg * s.test_count as u64))
                .sum();
            Some(weighted_sum / test_count as u64)
        } else {
            None
        };

        // Find slowest test across all suites
        let (max_ms, max_test) = self
            .suites
            .iter()
            .filter_map(|s| s.max_ms.map(|ms| (ms, s.max_test.clone())))
            .max_by_key(|(ms, _)| *ms)
            .map(|(ms, name)| (Some(ms), name))
            .unwrap_or((None, None));

        AggregatedMetrics { test_count, total_ms, avg_ms, max_ms, max_test }
    }
}

/// Result from a single test suite.
#[derive(Debug, Default)]
pub struct SuiteResult {
    /// Suite name (from config or defaults to runner).
    pub name: String,
    /// Runner used.
    pub runner: String,
    /// Whether all tests passed.
    pub passed: bool,
    /// Whether the suite was skipped.
    pub skipped: bool,
    /// Error message if skipped or failed.
    pub error: Option<String>,
    /// Number of tests run.
    pub test_count: usize,
    /// Number of skipped/ignored tests.
    pub skipped_count: usize,
    /// Total time in milliseconds.
    pub total_ms: u64,
    /// Average time per test in milliseconds.
    pub avg_ms: Option<u64>,
    /// Maximum test time in milliseconds.
    pub max_ms: Option<u64>,
    /// Name of the slowest test.
    pub max_test: Option<String>,
    /// 50th percentile duration in milliseconds.
    pub p50_ms: Option<u64>,
    /// 90th percentile duration in milliseconds.
    pub p90_ms: Option<u64>,
    /// 99th percentile duration in milliseconds.
    pub p99_ms: Option<u64>,
    /// Coverage data (language -> percentage).
    pub coverage: Option<HashMap<String, f64>>,
    /// Per-package coverage data (package name -> percentage).
    pub coverage_by_package: Option<HashMap<String, f64>>,
}

/// Run configured test suites.
///
/// Returns None if no suites are configured.
///
/// Execution strategy:
/// - CI mode with 2+ suites: Parallel execution via rayon
/// - Fast mode: Sequential with early termination on failure
pub fn run_suites(ctx: &CheckContext) -> Option<SuiteResults> {
    let suites = &ctx.config.check.tests.suite;
    if suites.is_empty() {
        return None;
    }

    let runner_ctx = RunnerContext {
        root: ctx.root,
        ci_mode: ctx.ci_mode,
        collect_coverage: ctx.ci_mode, // Coverage only in CI
        config: ctx.config,
        verbose: ctx.verbose,
    };

    // Filter suites for current mode
    let active_suites = filter_suites_for_mode(suites, ctx.ci_mode);
    if active_suites.is_empty() {
        return None;
    }

    let mut results = Vec::with_capacity(active_suites.len());
    let mut all_passed = true;

    // Parallel execution in CI mode when multiple suites
    if ctx.ci_mode && active_suites.len() > 1 {
        results =
            active_suites.par_iter().map(|suite| run_single_suite(suite, &runner_ctx)).collect();
        all_passed = results.iter().all(|r| r.passed || r.skipped);
    } else {
        // Sequential with early termination for fast mode
        for suite in active_suites {
            let result = run_single_suite(suite, &runner_ctx);
            let failed = !result.passed && !result.skipped;
            results.push(result);

            // Early termination in fast mode on first failure
            if failed && !ctx.ci_mode {
                all_passed = false;
                break;
            }
            if failed {
                all_passed = false;
            }
        }
    }

    Some(SuiteResults { passed: all_passed, suites: results })
}

/// Execute a single test suite and return its result.
pub fn run_single_suite(suite: &TestSuiteConfig, runner_ctx: &RunnerContext) -> SuiteResult {
    let suite_name = suite.name.clone().unwrap_or_else(|| suite.runner.clone());

    // Verbose: show which suite is starting
    if runner_ctx.verbose {
        eprintln!("  Running suite: {} ...", suite_name);
        if let Some(ref cmd) = suite.command {
            eprintln!("    command: {}", cmd);
        } else {
            eprintln!("    runner: {}", suite.runner);
        }
    }

    // Run setup command if configured
    if let Some(ref setup) = suite.setup
        && let Err(e) = run_setup_command(setup, runner_ctx.root)
    {
        // Setup failure skips the suite
        if runner_ctx.verbose {
            eprintln!("  Suite {:?} skipped: setup failed", suite_name);
        }
        return SuiteResult {
            name: suite_name,
            runner: suite.runner.clone(),
            skipped: true,
            error: Some(e),
            ..Default::default()
        };
    }

    // Get runner for this suite
    let runner = match get_runner(&suite.runner) {
        Some(r) => r,
        None => {
            if runner_ctx.verbose {
                eprintln!("  Suite {:?} skipped: unknown runner", suite_name);
            }
            return SuiteResult {
                name: suite_name,
                runner: suite.runner.clone(),
                skipped: true,
                error: Some(format!("unknown runner: {}", suite.runner)),
                ..Default::default()
            };
        }
    };

    // Check runner availability
    if !runner.available(runner_ctx) {
        if runner_ctx.verbose {
            eprintln!("  Suite {:?} skipped: runner not available", suite_name);
        }
        return SuiteResult {
            name: suite_name,
            runner: suite.runner.clone(),
            skipped: true,
            error: Some(format!("{} not available", suite.runner)),
            ..Default::default()
        };
    }

    // Execute the runner
    let run_result = runner.run(suite, runner_ctx);

    // Collect metrics before moving error
    let test_count = run_result.test_count();
    let skipped_count = run_result.skipped_count();
    let total_ms = run_result.total_time.as_millis() as u64;
    let avg_ms = run_result.avg_duration().map(|d| d.as_millis() as u64);
    let max_ms = run_result.slowest_test().map(|t| t.duration.as_millis() as u64);
    let max_test = run_result.slowest_test().map(|t| t.name.clone());
    let p50_ms = run_result.percentile_duration(50.0).map(|d| d.as_millis() as u64);
    let p90_ms = run_result.percentile_duration(90.0).map(|d| d.as_millis() as u64);
    let p99_ms = run_result.percentile_duration(99.0).map(|d| d.as_millis() as u64);
    let coverage = run_result.coverage.clone();
    let coverage_by_package = run_result.coverage_by_package.clone();

    // Verbose: show suite completion
    if runner_ctx.verbose {
        let exit_status = if run_result.passed { "passed" } else { "FAILED" };
        if run_result.passed {
            eprintln!(
                "  Suite {:?} completed: {}, {} tests, {}",
                suite_name,
                exit_status,
                test_count,
                format_duration_ms(total_ms),
            );
        } else {
            let failing =
                test_count.saturating_sub(run_result.tests.iter().filter(|t| t.passed).count());
            eprintln!(
                "  Suite {:?} completed: {}, {} tests ({} failing), {}",
                suite_name,
                exit_status,
                test_count,
                failing,
                format_duration_ms(total_ms),
            );
        }
    }

    SuiteResult {
        name: suite_name,
        runner: suite.runner.clone(),
        passed: run_result.passed,
        skipped: run_result.skipped,
        error: run_result.error,
        test_count,
        skipped_count,
        total_ms,
        avg_ms,
        max_ms,
        max_test,
        p50_ms,
        p90_ms,
        p99_ms,
        coverage,
        coverage_by_package,
    }
}

#[cfg(test)]
#[path = "suite_tests.rs"]
mod tests;
