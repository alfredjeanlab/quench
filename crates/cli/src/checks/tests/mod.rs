// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check implementation.
//!
//! Reference: docs/specs/checks/tests.md

pub mod correlation;
pub mod diff;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod unit_tests;

use std::sync::Arc;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::TestsCommitConfig;

use self::correlation::{
    CorrelationConfig, analyze_commit, analyze_correlation, has_inline_test_changes,
    has_placeholder_test,
};
use self::diff::{ChangeType, get_base_changes, get_commits_since, get_staged_changes};

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
            // No change context available - pass silently
            return CheckResult::passed(self.name());
        };

        // Analyze correlation
        let mut result = analyze_correlation(&changes, correlation_config, ctx.root);

        // Check for inline test changes in Rust files
        let base_ref = if ctx.staged { None } else { ctx.base_branch };
        let allow_placeholders = config.placeholders == "allow";

        result.without_tests.retain(|path| {
            // If the file has inline test changes, move it to with_tests
            if path.extension().is_some_and(|e| e == "rs")
                && has_inline_test_changes(path, ctx.root, base_ref)
            {
                return false; // Remove from without_tests
            }

            // If placeholders are allowed, check for placeholder tests
            if allow_placeholders && let Some(base_name) = path.file_stem().and_then(|s| s.to_str())
            {
                // Check common test file locations for placeholders
                let test_paths = [
                    format!("tests/{}_tests.rs", base_name),
                    format!("tests/{}_test.rs", base_name),
                    format!("tests/{}.rs", base_name),
                    format!("test/{}_tests.rs", base_name),
                    format!("test/{}.rs", base_name),
                ];

                for test_path in &test_paths {
                    let test_file = std::path::Path::new(test_path);
                    if ctx.root.join(test_file).exists()
                        && has_placeholder_test(test_file, base_name, ctx.root).unwrap_or(false)
                    {
                        return false; // Placeholder test satisfies requirement
                    }
                }
            }

            true // Keep in without_tests
        });

        // Build violations for source files without tests
        let mut violations = Vec::new();
        for path in &result.without_tests {
            let change = changes
                .iter()
                .find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path).eq(path));

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

            let advice = format!(
                "Add tests in tests/{}_tests.rs or update inline #[cfg(test)] block",
                file_stem
            );

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

        // Build metrics
        let metrics = json!({
            "source_files_changed": result.with_tests.len() + result.without_tests.len(),
            "with_test_changes": result.with_tests.len(),
            "without_test_changes": result.without_tests.len(),
            "scope": "branch",
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
                if path.extension().is_some_and(|e| e == "rs")
                    && self.has_inline_test_changes_in_commit(path, &commit.hash, ctx.root)
                {
                    continue;
                }

                // Check for placeholder tests
                if allow_placeholders
                    && let Some(base_name) = path.file_stem().and_then(|s| s.to_str())
                {
                    let test_paths = [
                        format!("tests/{}_tests.rs", base_name),
                        format!("tests/{}_test.rs", base_name),
                        format!("tests/{}.rs", base_name),
                        format!("test/{}_tests.rs", base_name),
                        format!("test/{}.rs", base_name),
                    ];

                    let has_placeholder = test_paths.iter().any(|test_path| {
                        let test_file = std::path::Path::new(test_path);
                        ctx.root.join(test_file).exists()
                            && has_placeholder_test(test_file, base_name, ctx.root).unwrap_or(false)
                    });

                    if has_placeholder {
                        continue;
                    }
                }

                failing_commits.push(analysis.hash.clone());

                let short_hash = if analysis.hash.len() >= 7 {
                    &analysis.hash[..7]
                } else {
                    &analysis.hash
                };

                let advice = format!(
                    "Commit {} modifies {} without test changes",
                    short_hash,
                    path.display()
                );

                // Find the change info for this file in this commit
                let change = commit
                    .changes
                    .iter()
                    .find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path).eq(path));

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

        let metrics = json!({
            "commits_checked": commits.len(),
            "commits_failing": failing_commits.len(),
            "scope": "commit",
        });

        if violations.is_empty() {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else if config.check == "warn" {
            CheckResult::passed_with_warnings(self.name(), violations).with_metrics(metrics)
        } else {
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }

    /// Check if a file has inline test changes within a specific commit.
    fn has_inline_test_changes_in_commit(
        &self,
        file_path: &std::path::Path,
        commit_hash: &str,
        root: &std::path::Path,
    ) -> bool {
        use correlation::changes_in_cfg_test;
        use std::process::Command;

        let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
        let rel_path_str = match rel_path.to_str() {
            Some(s) => s,
            None => return false,
        };

        let range = format!("{}^..{}", commit_hash, commit_hash);
        let output = Command::new("git")
            .args(["diff", &range, "--", rel_path_str])
            .current_dir(root)
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let diff = String::from_utf8_lossy(&o.stdout);
                changes_in_cfg_test(&diff)
            }
            _ => false,
        }
    }
}

fn build_correlation_config(config: &TestsCommitConfig) -> CorrelationConfig {
    CorrelationConfig {
        test_patterns: config.test_patterns.clone(),
        source_patterns: config.source_patterns.clone(),
        exclude_patterns: config.exclude.clone(),
    }
}
