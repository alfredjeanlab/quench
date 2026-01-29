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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;

use crate::adapter::{Adapter, FileKind, GenericAdapter};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::checks::placeholders::{
    PlaceholderMetrics, collect_placeholder_metrics, default_js_patterns, default_rust_patterns,
};
use crate::config::TestsCommitConfig;

use self::auto_detect::{
    auto_detect_go_suite, auto_detect_js_suite, auto_detect_py_suite, auto_detect_rust_suite,
    run_auto_detected_suite,
};
use self::correlation::{
    CorrelationConfig, DiffRange, analyze_commit, analyze_correlation, has_inline_test_changes,
};
use self::diff::{ChangeType, get_base_changes, get_commits_since, get_staged_changes};
use self::patterns::{Language, candidate_test_paths_for, detect_language};
use self::placeholder::{has_js_placeholder_test, has_placeholder_test};
use self::runners::{RunnerContext, filter_suites_for_mode};
use self::suite::run_suites;
use self::thresholds::{check_coverage_thresholds, check_time_thresholds};

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

        // Auto-detect test runners in CI mode only
        if ctx.ci_mode {
            // Try JavaScript
            if let Some((suite, source)) = auto_detect_js_suite(ctx.root) {
                let runner_ctx = RunnerContext {
                    root: ctx.root,
                    ci_mode: ctx.ci_mode,
                    collect_coverage: true,
                    config: ctx.config,
                };
                return run_auto_detected_suite(self.name(), suite, Some(source), &runner_ctx);
            }

            // Try Python
            if let Some((suite, source)) = auto_detect_py_suite(ctx.root) {
                let runner_ctx = RunnerContext {
                    root: ctx.root,
                    ci_mode: ctx.ci_mode,
                    collect_coverage: true,
                    config: ctx.config,
                };
                return run_auto_detected_suite(self.name(), suite, Some(source), &runner_ctx);
            }

            // Try Rust
            if let Some((suite, source)) = auto_detect_rust_suite(ctx.root) {
                let runner_ctx = RunnerContext {
                    root: ctx.root,
                    ci_mode: ctx.ci_mode,
                    collect_coverage: true,
                    config: ctx.config,
                };
                return run_auto_detected_suite(self.name(), suite, Some(source), &runner_ctx);
            }

            // Try Go
            if let Some((suite, source)) = auto_detect_go_suite(ctx.root) {
                let runner_ctx = RunnerContext {
                    root: ctx.root,
                    ci_mode: ctx.ci_mode,
                    collect_coverage: true,
                    config: ctx.config,
                };
                return run_auto_detected_suite(self.name(), suite, Some(source), &runner_ctx);
            }
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
        let suite_results = match run_suites(ctx) {
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
