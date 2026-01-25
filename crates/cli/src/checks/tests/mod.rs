// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check implementation.
//!
//! Reference: docs/specs/checks/tests.md

pub mod correlation;
pub mod diff;
pub mod patterns;
pub mod placeholder;
pub mod runners;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod unit_tests;

use std::sync::Arc;

use serde_json::json;

use crate::adapter::{Adapter, FileKind, GenericAdapter};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::checks::placeholders::{
    PlaceholderMetrics, collect_placeholder_metrics, default_js_patterns, default_rust_patterns,
};
use crate::config::{TestSuiteConfig, TestsCommitConfig};

use self::correlation::{
    CorrelationConfig, DiffRange, analyze_commit, analyze_correlation, has_inline_test_changes,
};
use self::diff::{ChangeType, get_base_changes, get_commits_since, get_staged_changes};
use self::patterns::{Language, candidate_test_paths_for, detect_language};
use self::placeholder::{has_js_placeholder_test, has_placeholder_test};
use self::runners::{RunnerContext, filter_suites_for_mode, get_runner, run_setup_command};
use std::path::{Path, PathBuf};

/// File extension for Rust source files.
const RUST_EXT: &str = "rs";

/// Length for truncating git hashes in display.
const SHORT_HASH_LEN: usize = 7;

/// Generate language-specific advice for missing tests.
fn missing_tests_advice(file_stem: &str, lang: Language) -> String {
    match lang {
        Language::Rust => format!(
            "Add tests in tests/{}_tests.rs or update inline #[cfg(test)] block",
            file_stem
        ),
        Language::Go => format!("Add tests in {}_test.go", file_stem),
        Language::JavaScript => format!(
            "Add tests in {}.test.ts or __tests__/{}.test.ts",
            file_stem, file_stem
        ),
        Language::Python => format!(
            "Add tests in test_{}.py or tests/test_{}.py",
            file_stem, file_stem
        ),
        Language::Unknown => format!("Add tests for {}", file_stem),
    }
}

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

        let config = &ctx.config.check.tests.commit;

        // Skip if disabled
        if config.check == "off" {
            return CheckResult::passed(self.name());
        }

        // Build correlation config from user settings
        let correlation_config = build_correlation_config(config);

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
        let suite_results = match self.run_suites(ctx) {
            Some(r) => r,
            None => return CheckResult::passed(self.name()),
        };

        // Calculate aggregated timing metrics
        let agg = suite_results.aggregated_metrics();

        // Aggregate coverage from all suites
        let mut aggregated_coverage: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for suite in &suite_results.suites {
            if let Some(ref cov) = suite.coverage {
                for (lang, pct) in cov {
                    // For now, take the last value for each language
                    // (future: merge coverage data properly)
                    aggregated_coverage.insert(lang.clone(), *pct);
                }
            }
        }

        // Aggregate per-package coverage from all suites
        let mut packages_coverage: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for suite in &suite_results.suites {
            if let Some(ref cov) = suite.coverage_by_package {
                for (pkg, pct) in cov {
                    packages_coverage
                        .entry(pkg.clone())
                        .and_modify(|existing| *existing = existing.max(*pct))
                        .or_insert(*pct);
                }
            }
        }

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
        let coverage_violations =
            self.check_coverage_thresholds(ctx, &aggregated_coverage, &packages_coverage);

        // Collect time threshold violations from each suite
        let mut time_violations = Vec::new();
        let active_suites = filter_suites_for_mode(&ctx.config.check.tests.suite, ctx.ci_mode);
        for (suite, result) in active_suites.iter().zip(suite_results.suites.iter()) {
            time_violations.extend(self.check_time_thresholds(ctx, suite, result));
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
            let mut violations: Vec<Violation> = suite_results
                .suites
                .iter()
                .filter(|s| !s.passed && !s.skipped)
                .map(|s| {
                    let advice = s
                        .error
                        .clone()
                        .unwrap_or_else(|| "test suite failed".to_string());
                    // Use suite name as synthetic file path for violations
                    Violation::file_only(format!("<suite:{}>", s.name), "test_suite_failed", advice)
                })
                .collect();
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

    /// Run branch-scope checking (aggregate all changes).
    fn run_branch_scope(
        &self,
        ctx: &CheckContext,
        correlation_config: &CorrelationConfig,
    ) -> CheckResult {
        let config = &ctx.config.check.tests.commit;

        // Need either --staged or --base for change detection
        let changes = if ctx.staged {
            match get_staged_changes(ctx.root) {
                Ok(c) => c,
                Err(e) => return CheckResult::skipped(self.name(), e),
            }
        } else if let Some(base) = ctx.base_branch {
            match get_base_changes(ctx.root, base) {
                Ok(c) => c,
                Err(e) => return CheckResult::skipped(self.name(), e),
            }
        } else {
            // No change context available - pass silently but still collect placeholder metrics
            let placeholder_metrics = collect_test_file_placeholder_metrics(ctx);
            let metrics = json!({
                "placeholders": placeholder_metrics.to_json(),
            });
            return CheckResult::passed(self.name()).with_metrics(metrics);
        };

        // Analyze correlation
        let mut result = analyze_correlation(&changes, correlation_config, ctx.root);

        // Check for inline test changes in Rust files
        let diff_range = if ctx.staged {
            DiffRange::Staged
        } else if let Some(base) = ctx.base_branch {
            DiffRange::Branch(base)
        } else {
            DiffRange::Staged // fallback, shouldn't reach here due to earlier check
        };
        let allow_placeholders = config.placeholders == "allow";

        result.without_tests.retain(|path| {
            // If the file has inline test changes, move it to with_tests
            if path.extension().is_some_and(|e| e == RUST_EXT)
                && has_inline_test_changes(path, ctx.root, diff_range)
            {
                return false; // Remove from without_tests
            }

            // If placeholders are allowed, check for placeholder tests
            if allow_placeholders && has_placeholder_for_source(path, ctx.root) {
                return false; // Placeholder test satisfies requirement
            }

            true // Keep in without_tests
        });

        // Build violations for source files without tests
        let mut violations = Vec::new();
        for path in &result.without_tests {
            let change = changes
                .iter()
                .find(|c| relative_to_root(&c.path, ctx.root).eq(path));

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let lang = detect_language(path);
            let advice = missing_tests_advice(file_stem, lang);

            let mut v = Violation::file_only(path, "missing_tests", advice);

            if let Some(c) = change {
                let change_type = match c.change_type {
                    ChangeType::Added => "added",
                    ChangeType::Modified => "modified",
                    ChangeType::Deleted => "deleted", // Won't occur for violations
                };
                v = v.with_change_info(change_type, c.lines_changed() as i64);
            }

            violations.push(v);

            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
        }

        // Collect placeholder metrics from all test files
        let placeholder_metrics = collect_test_file_placeholder_metrics(ctx);

        // Build metrics
        let metrics = json!({
            "source_files_changed": result.with_tests.len() + result.without_tests.len(),
            "with_test_changes": result.with_tests.len(),
            "without_test_changes": result.without_tests.len(),
            "scope": "branch",
            "placeholders": placeholder_metrics.to_json(),
        });

        if violations.is_empty() {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else if config.check == "warn" {
            CheckResult::passed_with_warnings(self.name(), violations).with_metrics(metrics)
        } else {
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }

    /// Run commit-scope checking (each commit independently).
    ///
    /// Asymmetric rules:
    /// - TDD commits (test-only) are OK
    /// - Code commits without tests FAIL
    fn run_commit_scope(
        &self,
        ctx: &CheckContext,
        base: &str,
        correlation_config: &CorrelationConfig,
    ) -> CheckResult {
        let config = &ctx.config.check.tests.commit;
        let allow_placeholders = config.placeholders == "allow";

        let commits = match get_commits_since(ctx.root, base) {
            Ok(c) => c,
            Err(e) => return CheckResult::skipped(self.name(), e),
        };

        let mut violations = Vec::new();
        let mut failing_commits = Vec::new();

        for commit in &commits {
            let analysis = analyze_commit(commit, correlation_config, ctx.root);

            // TDD commits (test-only) are OK
            if analysis.is_test_only {
                continue;
            }

            // Check each source file without tests
            for path in &analysis.source_without_tests {
                // Check for inline test changes within this commit
                if path.extension().is_some_and(|e| e == RUST_EXT)
                    && has_inline_test_changes(path, ctx.root, DiffRange::Commit(&commit.hash))
                {
                    continue;
                }

                // Check for placeholder tests
                if allow_placeholders && has_placeholder_for_source(path, ctx.root) {
                    continue;
                }

                failing_commits.push(analysis.hash.clone());

                let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                let lang = detect_language(path);
                let test_advice = missing_tests_advice(file_stem, lang);
                let advice = format!(
                    "Commit {} modifies {} without test changes. {}",
                    short_hash(&analysis.hash),
                    path.display(),
                    test_advice
                );

                // Find the change info for this file in this commit
                let change = commit
                    .changes
                    .iter()
                    .find(|c| relative_to_root(&c.path, ctx.root).eq(path));

                let mut v = Violation::file_only(path, "missing_tests", advice);

                if let Some(c) = change {
                    let change_type = match c.change_type {
                        ChangeType::Added => "added",
                        ChangeType::Modified => "modified",
                        ChangeType::Deleted => "deleted",
                    };
                    v = v.with_change_info(change_type, c.lines_changed() as i64);
                }

                violations.push(v);

                if ctx.limit.is_some_and(|l| violations.len() >= l) {
                    break;
                }
            }

            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
        }

        // Deduplicate failing commits
        failing_commits.sort();
        failing_commits.dedup();

        // Collect placeholder metrics from all test files
        let placeholder_metrics = collect_test_file_placeholder_metrics(ctx);

        let metrics = json!({
            "commits_checked": commits.len(),
            "commits_failing": failing_commits.len(),
            "scope": "commit",
            "placeholders": placeholder_metrics.to_json(),
        });

        if violations.is_empty() {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else if config.check == "warn" {
            CheckResult::passed_with_warnings(self.name(), violations).with_metrics(metrics)
        } else {
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }
}

/// Collect test files from context and compute placeholder metrics.
fn collect_test_file_placeholder_metrics(ctx: &CheckContext) -> PlaceholderMetrics {
    // Build file adapter to classify test files
    let test_patterns = if ctx.config.project.tests.is_empty() {
        default_test_patterns()
    } else {
        ctx.config.project.tests.clone()
    };
    let file_adapter = GenericAdapter::new(&[], &test_patterns);

    // Collect test file paths
    let test_files: Vec<PathBuf> = ctx
        .files
        .iter()
        .filter(|f| {
            let rel_path = f.path.strip_prefix(ctx.root).unwrap_or(&f.path);
            file_adapter.classify(rel_path) == FileKind::Test
        })
        .map(|f| f.path.clone())
        .collect();

    // Collect placeholder metrics using default patterns
    let rust_patterns = default_rust_patterns();
    let js_patterns = default_js_patterns();
    collect_placeholder_metrics(&test_files, &rust_patterns, &js_patterns)
}

/// Default test file patterns.
fn default_test_patterns() -> Vec<String> {
    vec![
        "**/tests/**".to_string(),
        "**/test/**".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.test.*".to_string(),
        "**/*.spec.*".to_string(),
    ]
}

fn build_correlation_config(config: &TestsCommitConfig) -> CorrelationConfig {
    CorrelationConfig {
        test_patterns: config.test_patterns.clone(),
        source_patterns: config.source_patterns.clone(),
        exclude_patterns: config.exclude.clone(),
    }
}

/// Normalize a path relative to root, returning the original if not under root.
fn relative_to_root<'a>(path: &'a Path, root: &Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

/// Truncate a git hash to short form for display.
fn short_hash(hash: &str) -> &str {
    if hash.len() >= SHORT_HASH_LEN {
        &hash[..SHORT_HASH_LEN]
    } else {
        hash
    }
}

/// Check if any placeholder test satisfies the test requirement for a source file.
fn has_placeholder_for_source(source_path: &Path, root: &Path) -> bool {
    let base_name = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    let lang = detect_language(source_path);
    let candidate_paths = candidate_test_paths_for(source_path);

    // Check each candidate path for placeholder tests
    candidate_paths.iter().any(|test_path| {
        let test_file = Path::new(test_path);
        if !root.join(test_file).exists() {
            return false;
        }

        // Use language-specific placeholder detection
        match lang {
            Language::JavaScript => {
                has_js_placeholder_test(test_file, base_name, root).unwrap_or(false)
            }
            Language::Rust => has_placeholder_test(test_file, base_name, root).unwrap_or(false),
            // Other languages don't have placeholder test support yet
            _ => false,
        }
    })
}

// =============================================================================
// Threshold Checking
// =============================================================================

impl TestsCheck {
    /// Check coverage against configured thresholds.
    fn check_coverage_thresholds(
        &self,
        ctx: &CheckContext,
        coverage: &std::collections::HashMap<String, f64>,
        packages: &std::collections::HashMap<String, f64>,
    ) -> Vec<(Violation, bool)> {
        let config = &ctx.config.check.tests.coverage;
        if config.check == "off" {
            return Vec::new();
        }

        let is_error = config.check == "error";
        let mut violations = Vec::new();

        // Check global minimum
        if let Some(min) = config.min {
            for (lang, &actual) in coverage {
                if actual < min {
                    let advice = format!("Coverage {:.1}% below minimum {:.1}%", actual, min);
                    let v = Violation::file_only(
                        format!("<coverage:{}>", lang),
                        "coverage_below_min",
                        advice,
                    )
                    .with_threshold(actual as i64, min as i64);
                    violations.push((v, is_error));
                }
            }
        }

        // Check per-package thresholds
        for (pkg, pkg_config) in &config.package {
            if let Some(&actual) = packages.get(pkg)
                && actual < pkg_config.min
            {
                let advice = format!(
                    "Package '{}' coverage {:.1}% below minimum {:.1}%",
                    pkg, actual, pkg_config.min
                );
                let v = Violation::file_only(
                    format!("<coverage:{}>", pkg),
                    "coverage_below_min",
                    advice,
                )
                .with_threshold(actual as i64, pkg_config.min as i64);
                violations.push((v, is_error));
            }
        }

        violations
    }

    /// Check time thresholds for a suite.
    fn check_time_thresholds(
        &self,
        ctx: &CheckContext,
        suite: &TestSuiteConfig,
        result: &SuiteResult,
    ) -> Vec<(Violation, bool)> {
        let config = &ctx.config.check.tests.time;
        if config.check == "off" {
            return Vec::new();
        }

        let is_error = config.check == "error";
        let mut violations = Vec::new();
        let suite_name = &result.name;

        // Check max_total
        if let Some(max_total) = suite.max_total {
            let max_ms = max_total.as_millis() as u64;
            if result.total_ms > max_ms {
                let advice = format!(
                    "Suite '{}' took {}ms, exceeds max_total {}ms",
                    suite_name, result.total_ms, max_ms
                );
                let v = Violation::file_only(
                    format!("<suite:{}>", suite_name),
                    "time_total_exceeded",
                    advice,
                )
                .with_threshold(result.total_ms as i64, max_ms as i64);
                violations.push((v, is_error));
            }
        }

        // Check max_avg
        if let Some(max_avg) = suite.max_avg
            && let Some(avg_ms) = result.avg_ms
        {
            let max_ms = max_avg.as_millis() as u64;
            if avg_ms > max_ms {
                let advice = format!(
                    "Suite '{}' average {}ms/test, exceeds max_avg {}ms",
                    suite_name, avg_ms, max_ms
                );
                let v = Violation::file_only(
                    format!("<suite:{}>", suite_name),
                    "time_avg_exceeded",
                    advice,
                )
                .with_threshold(avg_ms as i64, max_ms as i64);
                violations.push((v, is_error));
            }
        }

        // Check max_test
        if let Some(max_test) = suite.max_test
            && let Some(max_ms) = result.max_ms
        {
            let threshold_ms = max_test.as_millis() as u64;
            if max_ms > threshold_ms {
                let test_name = result.max_test.as_deref().unwrap_or("unknown");
                let advice = format!(
                    "Test '{}' took {}ms, exceeds max_test {}ms",
                    test_name, max_ms, threshold_ms
                );
                let v = Violation::file_only(
                    format!("<test:{}>", test_name),
                    "time_test_exceeded",
                    advice,
                )
                .with_threshold(max_ms as i64, threshold_ms as i64);
                violations.push((v, is_error));
            }
        }

        violations
    }
}

// =============================================================================
// Test Suite Execution
// =============================================================================

impl TestsCheck {
    /// Run configured test suites.
    ///
    /// Returns None if no suites are configured.
    fn run_suites(&self, ctx: &CheckContext) -> Option<SuiteResults> {
        let suites = &ctx.config.check.tests.suite;
        if suites.is_empty() {
            return None;
        }

        let runner_ctx = RunnerContext {
            root: ctx.root,
            ci_mode: ctx.ci_mode,
            collect_coverage: ctx.ci_mode, // Coverage only in CI
        };

        // Filter suites for current mode
        let active_suites = filter_suites_for_mode(suites, ctx.ci_mode);
        if active_suites.is_empty() {
            return None;
        }

        let mut results = Vec::new();
        let mut all_passed = true;

        for suite in active_suites {
            // Run setup command if configured
            if let Some(ref setup) = suite.setup
                && let Err(e) = run_setup_command(setup, ctx.root)
            {
                // Setup failure skips the suite
                results.push(SuiteResult {
                    name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                    runner: suite.runner.clone(),
                    skipped: true,
                    error: Some(e),
                    ..Default::default()
                });
                continue;
            }

            // Get runner for this suite
            let runner = match get_runner(&suite.runner) {
                Some(r) => r,
                None => {
                    results.push(SuiteResult {
                        name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                        runner: suite.runner.clone(),
                        skipped: true,
                        error: Some(format!("unknown runner: {}", suite.runner)),
                        ..Default::default()
                    });
                    continue;
                }
            };

            // Check runner availability
            if !runner.available(&runner_ctx) {
                results.push(SuiteResult {
                    name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                    runner: suite.runner.clone(),
                    skipped: true,
                    error: Some(format!("{} not available", suite.runner)),
                    ..Default::default()
                });
                continue;
            }

            // Execute the runner
            let run_result = runner.run(suite, &runner_ctx);
            all_passed = all_passed && run_result.passed;

            // Collect metrics before moving error
            let test_count = run_result.test_count();
            let total_ms = run_result.total_time.as_millis() as u64;
            let avg_ms = run_result.avg_duration().map(|d| d.as_millis() as u64);
            let max_ms = run_result
                .slowest_test()
                .map(|t| t.duration.as_millis() as u64);
            let max_test = run_result.slowest_test().map(|t| t.name.clone());
            let coverage = run_result.coverage.clone();
            let coverage_by_package = run_result.coverage_by_package.clone();

            results.push(SuiteResult {
                name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                runner: suite.runner.clone(),
                passed: run_result.passed,
                skipped: run_result.skipped,
                error: run_result.error,
                test_count,
                total_ms,
                avg_ms,
                max_ms,
                max_test,
                coverage,
                coverage_by_package,
            });
        }

        Some(SuiteResults {
            passed: all_passed,
            suites: results,
        })
    }
}

/// Aggregated results from all test suites.
#[derive(Debug, Default)]
struct SuiteResults {
    /// Whether all suites passed.
    passed: bool,
    /// Individual suite results.
    suites: Vec<SuiteResult>,
}

/// Top-level aggregated metrics across all suites.
#[derive(Debug)]
struct AggregatedMetrics {
    /// Total tests across all suites.
    test_count: usize,
    /// Total execution time in milliseconds.
    total_ms: u64,
    /// Weighted average time per test in milliseconds.
    avg_ms: Option<u64>,
    /// Maximum test time in milliseconds (across all suites).
    max_ms: Option<u64>,
    /// Name of the slowest test (across all suites).
    max_test: Option<String>,
}

impl SuiteResults {
    /// Calculate aggregated timing metrics across all suites.
    fn aggregated_metrics(&self) -> AggregatedMetrics {
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

        AggregatedMetrics {
            test_count,
            total_ms,
            avg_ms,
            max_ms,
            max_test,
        }
    }
}

impl SuiteResults {
    /// Convert to JSON for metrics.
    #[allow(dead_code)] // Will be used in future phases
    fn to_json(&self) -> serde_json::Value {
        let suites: Vec<serde_json::Value> = self
            .suites
            .iter()
            .map(|s| {
                let mut obj = serde_json::json!({
                    "name": s.name,
                    "runner": s.runner,
                    "passed": s.passed,
                    "skipped": s.skipped,
                });
                if let Some(ref err) = s.error {
                    obj["error"] = serde_json::json!(err);
                }
                if s.test_count > 0 {
                    obj["test_count"] = serde_json::json!(s.test_count);
                }
                if s.total_ms > 0 {
                    obj["total_ms"] = serde_json::json!(s.total_ms);
                }
                if let Some(avg) = s.avg_ms {
                    obj["avg_ms"] = serde_json::json!(avg);
                }
                if let Some(max) = s.max_ms {
                    obj["max_ms"] = serde_json::json!(max);
                }
                if let Some(ref test) = s.max_test {
                    obj["max_test"] = serde_json::json!(test);
                }
                obj
            })
            .collect();

        serde_json::json!({
            "passed": self.passed,
            "suites": suites,
            "total_tests": self.suites.iter().map(|s| s.test_count).sum::<usize>(),
            "total_ms": self.suites.iter().map(|s| s.total_ms).sum::<u64>(),
        })
    }
}

/// Result from a single test suite.
#[derive(Debug, Default)]
struct SuiteResult {
    /// Suite name (from config or defaults to runner).
    name: String,
    /// Runner used.
    runner: String,
    /// Whether all tests passed.
    passed: bool,
    /// Whether the suite was skipped.
    skipped: bool,
    /// Error message if skipped or failed.
    error: Option<String>,
    /// Number of tests run.
    test_count: usize,
    /// Total time in milliseconds.
    total_ms: u64,
    /// Average time per test in milliseconds.
    avg_ms: Option<u64>,
    /// Maximum test time in milliseconds.
    max_ms: Option<u64>,
    /// Name of the slowest test.
    max_test: Option<String>,
    /// Coverage data (language -> percentage).
    coverage: Option<std::collections::HashMap<String, f64>>,
    /// Per-package coverage data (package name -> percentage).
    coverage_by_package: Option<std::collections::HashMap<String, f64>>,
}
