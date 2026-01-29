// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check implementation.
//!
//! Reference: docs/specs/checks/tests.md

pub mod auto_detect;
pub mod correlation;
pub mod diff;
pub mod patterns;
pub mod placeholder;
pub mod runners;
pub mod suite;
pub mod thresholds;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod unit_tests;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};

use self::auto_detect::{
    auto_detect_go_suite, auto_detect_js_suite, auto_detect_py_suite, auto_detect_rust_suite,
};
use self::correlation::CorrelationConfig;
use self::runners::{RunnerContext, filter_suites_for_mode};
use self::suite::{SuiteResult, run_single_suite, run_suites};
use self::thresholds::{check_coverage_thresholds, check_time_thresholds};

pub struct TestsCheck;

impl TestsCheck {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl Check for TestsCheck {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn description(&self) -> &'static str {
        "Test correlation"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Run test suites if configured
        if !ctx.config.check.tests.suite.is_empty() {
            return self.run_test_suites(ctx);
        }

        // Auto-detect test runners in CI mode only
        if ctx.ci_mode {
            // Collect all auto-detected suites
            let mut auto_detected_suites = Vec::new();

            // Try JavaScript
            if let Some((suite, source)) = auto_detect_js_suite(ctx.root) {
                auto_detected_suites.push((suite, source));
            }

            // Try Python
            if let Some((suite, source)) = auto_detect_py_suite(ctx.root) {
                auto_detected_suites.push((suite, source));
            }

            // Try Rust
            if let Some((suite, source)) = auto_detect_rust_suite(ctx.root) {
                auto_detected_suites.push((suite, source));
            }

            // Try Go
            if let Some((suite, source)) = auto_detect_go_suite(ctx.root) {
                auto_detected_suites.push((suite, source));
            }

            // If we found any auto-detected suites, run them all
            if !auto_detected_suites.is_empty() {
                return self.run_auto_detected_suites(ctx, auto_detected_suites);
            }
        }

        let config = &ctx.config.check.tests.commit;

        // Skip if disabled
        if config.check == "off" {
            return CheckResult::passed(self.name());
        }

        // Build correlation config from user settings
        let correlation_config = CorrelationConfig {
            test_patterns: config.test_patterns.clone(),
            source_patterns: config.source_patterns.clone(),
            exclude_patterns: config.exclude.clone(),
        };

        // Commit scope: check each commit individually
        // Branch scope: aggregate all changes (existing behavior)
        // Staged mode: always use branch-like behavior (single unit of changes)
        if config.scope == "commit"
            && !ctx.staged
            && let Some(base) = ctx.base_branch
        {
            return self.run_commit_scope(ctx, base, &correlation_config);
        }

        // Default to branch scope
        self.run_branch_scope(ctx, &correlation_config)
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

impl TestsCheck {
    /// Run configured test suites and return results.
    fn run_test_suites(&self, ctx: &CheckContext) -> CheckResult {
        let suite_results = match run_suites(ctx) {
            Some(r) => r,
            None => return CheckResult::passed(self.name()),
        };

        // Calculate aggregated timing metrics
        let agg = suite_results.aggregated_metrics();

        // Aggregate coverage from all suites
        let suite_refs: Vec<&SuiteResult> = suite_results.suites.iter().collect();
        let (aggregated_coverage, packages_coverage) = aggregate_suite_coverage(&suite_refs);

        // Build metrics JSON with top-level aggregates
        let mut metrics = json!({
            "test_count": agg.test_count,
            "total_ms": agg.total_ms,
            "suites": suite_results.suites.iter().map(|s| {
                let mut obj = json!({
                    "name": s.name,
                    "runner": s.runner,
                    "passed": s.passed,
                    "test_count": s.test_count,
                });
                if s.skipped_count > 0 {
                    obj["skipped_count"] = json!(s.skipped_count);
                }
                if let Some(ref err) = s.error {
                    obj["error"] = json!(err);
                }
                if s.total_ms > 0 {
                    obj["total_ms"] = json!(s.total_ms);
                }
                if let Some(avg) = s.avg_ms {
                    obj["avg_ms"] = json!(avg);
                }
                if let Some(max) = s.max_ms {
                    obj["max_ms"] = json!(max);
                }
                if let Some(ref test) = s.max_test {
                    obj["max_test"] = json!(test);
                }
                if let Some(p50) = s.p50_ms {
                    obj["p50_ms"] = json!(p50);
                }
                if let Some(p90) = s.p90_ms {
                    obj["p90_ms"] = json!(p90);
                }
                if let Some(p99) = s.p99_ms {
                    obj["p99_ms"] = json!(p99);
                }
                obj
            }).collect::<Vec<_>>(),
        });

        // Add optional aggregated timing metrics
        if let Some(avg) = agg.avg_ms {
            metrics["avg_ms"] = json!(avg);
        }
        if let Some(max) = agg.max_ms {
            metrics["max_ms"] = json!(max);
        }
        if let Some(ref test) = agg.max_test {
            metrics["max_test"] = json!(test);
        }

        // Add coverage to metrics if available
        if !aggregated_coverage.is_empty() {
            metrics["coverage"] = json!(aggregated_coverage);
        }

        // Add per-package coverage if available
        if !packages_coverage.is_empty() {
            metrics["coverage_by_package"] = json!(packages_coverage);
        }

        // Collect coverage threshold violations
        let coverage_violations = check_coverage_thresholds(
            &ctx.config.check.tests,
            &aggregated_coverage,
            &packages_coverage,
        );

        // Collect time threshold violations from each suite
        let mut time_violations = Vec::new();
        let active_suites = filter_suites_for_mode(&ctx.config.check.tests.suite, ctx.ci_mode);
        for (suite, result) in active_suites.iter().zip(suite_results.suites.iter()) {
            time_violations.extend(check_time_thresholds(
                &ctx.config.check.tests,
                suite,
                result,
            ));
        }

        // Combine all threshold violations
        let all_threshold_violations: Vec<(Violation, bool)> = coverage_violations
            .into_iter()
            .chain(time_violations)
            .collect();

        let has_threshold_errors = all_threshold_violations.iter().any(|(_, is_err)| *is_err);
        let threshold_violations: Vec<Violation> = all_threshold_violations
            .into_iter()
            .map(|(v, _)| v)
            .collect();

        if suite_results.passed && threshold_violations.is_empty() {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else if !suite_results.passed {
            // Build violations for failed suites
            let suite_refs: Vec<&SuiteResult> = suite_results.suites.iter().collect();
            let mut violations = build_suite_violations(&suite_refs);
            // Add threshold violations to suite failure violations
            violations.extend(threshold_violations);
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        } else if has_threshold_errors {
            CheckResult::failed(self.name(), threshold_violations).with_metrics(metrics)
        } else {
            // Threshold violations exist but are warnings only
            CheckResult::passed_with_warnings(self.name(), threshold_violations)
                .with_metrics(metrics)
        }
    }

    /// Run all auto-detected test suites and aggregate results.
    fn run_auto_detected_suites(
        &self,
        ctx: &CheckContext,
        auto_detected: Vec<(crate::config::TestSuiteConfig, String)>,
    ) -> CheckResult {
        // === Verbose: Auto-detected test suites ===
        if ctx.verbose {
            eprintln!("\nTest Suites:");
            eprintln!("  Auto-detected suites:");
            for (suite, source) in &auto_detected {
                let name = suite.name.clone().unwrap_or_else(|| suite.runner.clone());
                eprintln!("    {} (detected: {})", name, source);
            }
        }

        let runner_ctx = RunnerContext {
            root: ctx.root,
            ci_mode: ctx.ci_mode,
            collect_coverage: true,
            config: ctx.config,
            verbose: ctx.verbose,
        };

        // Run all auto-detected suites
        let suite_results: Vec<(SuiteResult, String)> = auto_detected
            .into_iter()
            .map(|(suite, detection_source)| {
                let result = run_single_suite(&suite, &runner_ctx);
                (result, detection_source)
            })
            .collect();

        // Aggregate results
        let all_passed = suite_results.iter().all(|(r, _)| r.passed || r.skipped);
        let test_count: usize = suite_results.iter().map(|(r, _)| r.test_count).sum();
        let total_ms: u64 = suite_results.iter().map(|(r, _)| r.total_ms).sum();

        // Weighted average across all suites
        let avg_ms = if test_count > 0 {
            let weighted_sum: u64 = suite_results
                .iter()
                .filter_map(|(r, _)| r.avg_ms.map(|avg| avg * r.test_count as u64))
                .sum();
            Some(weighted_sum / test_count as u64)
        } else {
            None
        };

        // Find slowest test across all suites
        let (max_ms, max_test) = suite_results
            .iter()
            .filter_map(|(r, _)| r.max_ms.map(|ms| (ms, r.max_test.clone())))
            .max_by_key(|(ms, _)| *ms)
            .map(|(ms, name)| (Some(ms), name))
            .unwrap_or((None, None));

        // Aggregate coverage from all suites
        let suites_only: Vec<&SuiteResult> = suite_results.iter().map(|(r, _)| r).collect();
        let (aggregated_coverage, packages_coverage) = aggregate_suite_coverage(&suites_only);

        // Build metrics JSON
        let mut metrics = json!({
            "test_count": test_count,
            "total_ms": total_ms,
            "auto_detected": true,
            "suites": suite_results.iter().map(|(s, source)| {
                let mut obj = json!({
                    "name": s.name,
                    "runner": s.runner,
                    "passed": s.passed,
                    "test_count": s.test_count,
                    "detection_source": source,
                });
                if s.total_ms > 0 {
                    obj["total_ms"] = json!(s.total_ms);
                }
                if let Some(avg) = s.avg_ms {
                    obj["avg_ms"] = json!(avg);
                }
                if let Some(max) = s.max_ms {
                    obj["max_ms"] = json!(max);
                }
                if let Some(ref test) = s.max_test {
                    obj["max_test"] = json!(test);
                }
                obj
            }).collect::<Vec<_>>(),
        });

        // Add optional aggregated timing metrics
        if let Some(avg) = avg_ms {
            metrics["avg_ms"] = json!(avg);
        }
        if let Some(max) = max_ms {
            metrics["max_ms"] = json!(max);
        }
        if let Some(ref test) = max_test {
            metrics["max_test"] = json!(test);
        }

        // Add coverage if available
        if !aggregated_coverage.is_empty() {
            metrics["coverage"] = json!(aggregated_coverage);
        }

        // Add per-package coverage if available
        if !packages_coverage.is_empty() {
            metrics["coverage_by_package"] = json!(packages_coverage);
        }

        // Build result
        if all_passed {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else {
            let violations = build_suite_violations(&suites_only);
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }
    /// Run branch-scope checking (aggregate all changes).
    fn run_branch_scope(&self, ctx: &CheckContext, correlation_config: &CorrelationConfig) -> CheckResult {
        correlation::check_branch_scope(self.name(), ctx, correlation_config)
    }

    /// Run commit-scope checking (each commit independently).
    fn run_commit_scope(&self, ctx: &CheckContext, base: &str, correlation_config: &CorrelationConfig) -> CheckResult {
        correlation::check_commit_scope(self.name(), ctx, base, correlation_config)
    }
}

// =============================================================================
// Suite Checking Helpers
// =============================================================================

/// Aggregate coverage data from suite results by language.
fn aggregate_suite_coverage(
    suites: &[&SuiteResult],
) -> (HashMap<String, f64>, HashMap<String, f64>) {
    let mut by_language = HashMap::new();
    let mut by_package = HashMap::new();

    for &suite in suites {
        if let Some(ref cov) = suite.coverage {
            for (lang, pct) in cov {
                by_language
                    .entry(lang.clone())
                    .and_modify(|existing: &mut f64| *existing = existing.max(*pct))
                    .or_insert(*pct);
            }
        }
        if let Some(ref cov) = suite.coverage_by_package {
            for (pkg, pct) in cov {
                by_package
                    .entry(pkg.clone())
                    .and_modify(|existing: &mut f64| *existing = existing.max(*pct))
                    .or_insert(*pct);
            }
        }
    }

    (by_language, by_package)
}

/// Build violations from failed suites.
fn build_suite_violations(suites: &[&SuiteResult]) -> Vec<Violation> {
    suites
        .iter()
        .filter(|&&s| !s.passed && !s.skipped)
        .map(|&s| {
            let advice = s
                .error
                .clone()
                .unwrap_or_else(|| "test suite failed".to_string());
            Violation::file_only(format!("<suite:{}>", s.name), "test_suite_failed", advice)
        })
        .collect()
}

