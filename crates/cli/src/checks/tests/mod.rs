// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check implementation.
//!
//! Reference: docs/specs/checks/tests.md

pub mod correlation;
pub mod diff;
pub mod placeholder;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod unit_tests;

use std::sync::Arc;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::TestsCommitConfig;

use self::correlation::{
    CorrelationConfig, DiffRange, analyze_commit, analyze_correlation, candidate_js_test_paths,
    candidate_test_paths, has_inline_test_changes,
};
use self::diff::{ChangeType, get_base_changes, get_commits_since, get_staged_changes};
use self::placeholder::{has_js_placeholder_test, has_placeholder_test};
use std::path::Path;

/// File extension for Rust source files.
const RUST_EXT: &str = "rs";

/// Length for truncating git hashes in display.
const SHORT_HASH_LEN: usize = 7;

/// Detected language of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Rust,
    Go,
    JavaScript,
    Python,
    Unknown,
}

/// Detect the language of a source file from its extension.
fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Language::Rust,
        Some("go") => Language::Go,
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "mts") => Language::JavaScript,
        Some("py") => Language::Python,
        _ => Language::Unknown,
    }
}

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

    // Determine if this is a JS/TS file
    let is_js = source_path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts"));

    if is_js {
        // Check JS/TS placeholder tests
        candidate_js_test_paths(base_name).iter().any(|test_path| {
            let test_file = Path::new(test_path);
            root.join(test_file).exists()
                && has_js_placeholder_test(test_file, base_name, root).unwrap_or(false)
        })
    } else {
        // Check Rust placeholder tests
        candidate_test_paths(base_name).iter().any(|test_path| {
            let test_file = Path::new(test_path);
            root.join(test_file).exists()
                && has_placeholder_test(test_file, base_name, root).unwrap_or(false)
        })
    }
}
