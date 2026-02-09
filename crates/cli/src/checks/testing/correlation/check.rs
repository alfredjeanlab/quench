// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Branch-scope and commit-scope correlation checks with placeholder metrics.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::adapter::{Adapter, FileKind, GenericAdapter};
use crate::check::{CheckContext, CheckResult, Violation};
use crate::checks::placeholders::{
    PlaceholderMetrics, collect_placeholder_metrics, default_js_patterns, default_rust_patterns,
};

use super::super::diff::{
    ChangeType, FileChange, get_base_changes, get_commits_since, get_staged_changes,
};
use super::super::patterns::{Language, candidate_test_paths_for, detect_language};
use super::super::placeholder::{has_js_placeholder_test, has_placeholder_test};
use super::diff::{DiffRange, has_inline_test_changes};
use super::{CorrelationConfig, analyze_commit, analyze_correlation};

const RUST_EXT: &str = "rs";
const SHORT_HASH_LEN: usize = 7;

fn truncate_hash(hash: &str) -> &str {
    if hash.len() >= SHORT_HASH_LEN { &hash[..SHORT_HASH_LEN] } else { hash }
}

pub fn missing_tests_advice(file_stem: &str, lang: Language) -> String {
    match lang {
        Language::Rust => {
            format!("Add tests in tests/{}.rs or a sibling {}_tests.rs file", file_stem, file_stem)
        }
        Language::Go => format!("Add tests in {}_test.go", file_stem),
        Language::JavaScript => {
            format!("Add tests in {}.test.ts or __tests__/{}.test.ts", file_stem, file_stem)
        }
        Language::Python => {
            format!("Add tests in test_{}.py or tests/test_{}.py", file_stem, file_stem)
        }
        Language::Unknown => format!("Add tests for {}", file_stem),
    }
}

fn should_skip_path(
    path: &Path,
    allow_placeholders: bool,
    diff_range: DiffRange,
    root: &Path,
) -> bool {
    (path.extension().is_some_and(|e| e == RUST_EXT)
        && has_inline_test_changes(path, root, diff_range))
        || (allow_placeholders && has_placeholder_for_source(path, root))
}

fn has_placeholder_for_source(source_path: &Path, root: &Path) -> bool {
    let Some(base_name) = source_path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let lang = detect_language(source_path);
    candidate_test_paths_for(source_path).iter().any(|test_path| {
        let test_file = Path::new(test_path);
        root.join(test_file).exists()
            && match lang {
                Language::JavaScript => {
                    has_js_placeholder_test(test_file, base_name, root).unwrap_or(false)
                }
                Language::Rust => has_placeholder_test(test_file, base_name, root).unwrap_or(false),
                _ => false,
            }
    })
}

fn get_diff_range<'a>(ctx: &'a CheckContext) -> DiffRange<'a> {
    if ctx.staged {
        DiffRange::Staged
    } else {
        ctx.base_branch.map(DiffRange::Branch).unwrap_or(DiffRange::Staged)
    }
}

fn build_violations(
    paths: &[PathBuf],
    changes: &[FileChange],
    ctx: &CheckContext,
    commit_hash: Option<&str>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for path in paths {
        let change =
            changes.iter().find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path) == path);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let lang = detect_language(path);
        let test_advice = missing_tests_advice(file_stem, lang);
        let advice = commit_hash.map_or(test_advice.clone(), |hash| {
            format!(
                "Commit {} modifies {} without test changes. {}",
                hash,
                path.display(),
                test_advice
            )
        });
        let mut v = Violation::file_only(path, "missing_tests", advice);
        if let Some(c) = change {
            let ct = match c.change_type {
                ChangeType::Added => "added",
                ChangeType::Modified => "modified",
                ChangeType::Deleted => "deleted",
            };
            v = v.with_change_info(ct, c.lines_changed() as i64);
        }
        violations.push(v);
        if ctx.limit.is_some_and(|l| violations.len() >= l) {
            break;
        }
    }
    violations
}

#[rustfmt::skip]
fn collect_test_file_placeholder_metrics(ctx: &CheckContext) -> PlaceholderMetrics {
    let test_patterns = if ctx.config.project.tests.is_empty() {
        vec!["**/tests/**".to_string(), "**/test/**".to_string(), "**/*_test.*".to_string(), "**/*_tests.*".to_string(), "**/*.test.*".to_string(), "**/*.spec.*".to_string()]
    } else {
        ctx.config.project.tests.clone()
    };
    let file_adapter = GenericAdapter::new(&[], &test_patterns);
    let test_files: Vec<PathBuf> = ctx.files.iter().filter(|f| file_adapter.classify(f.path.strip_prefix(ctx.root).unwrap_or(&f.path)) == FileKind::Test).map(|f| f.path.clone()).collect();
    collect_placeholder_metrics(&test_files, &default_rust_patterns(), &default_js_patterns())
}

fn finalize_with_placeholders(
    violations: Vec<Violation>,
    ctx: &CheckContext,
    mut metrics: serde_json::Value,
    check_name: &str,
) -> CheckResult {
    metrics["placeholders"] = json!(collect_test_file_placeholder_metrics(ctx).to_json());
    let config_check = &ctx.config.check.tests.commit.check;
    if violations.is_empty() {
        CheckResult::passed(check_name).with_metrics(metrics)
    } else if config_check == "warn" {
        CheckResult::passed_with_warnings(check_name, violations).with_metrics(metrics)
    } else {
        CheckResult::failed(check_name, violations).with_metrics(metrics)
    }
}

pub fn check_branch_scope(
    check_name: &str,
    ctx: &CheckContext,
    correlation_config: &CorrelationConfig,
) -> CheckResult {
    let config = &ctx.config.check.tests.commit;
    let changes = if ctx.staged {
        match get_staged_changes(ctx.root) {
            Ok(c) => c,
            Err(e) => return CheckResult::skipped(check_name, e),
        }
    } else if let Some(base) = ctx.base_branch {
        match get_base_changes(ctx.root, base) {
            Ok(c) => c,
            Err(e) => return CheckResult::skipped(check_name, e),
        }
    } else {
        return finalize_with_placeholders(vec![], ctx, json!({}), check_name);
    };

    let mut result = analyze_correlation(&changes, correlation_config, ctx.root);
    result.without_tests.retain(|path| {
        !should_skip_path(path, config.placeholders == "allow", get_diff_range(ctx), ctx.root)
    });
    let violations = build_violations(&result.without_tests, &changes, ctx, None);
    let metrics = json!({
        "source_files_changed": result.with_tests.len() + result.without_tests.len(),
        "with_test_changes": result.with_tests.len(),
        "without_test_changes": result.without_tests.len(),
        "scope": "branch",
    });
    finalize_with_placeholders(violations, ctx, metrics, check_name)
}

pub fn check_commit_scope(
    check_name: &str,
    ctx: &CheckContext,
    base: &str,
    correlation_config: &CorrelationConfig,
) -> CheckResult {
    let config = &ctx.config.check.tests.commit;
    let commits = match get_commits_since(ctx.root, base) {
        Ok(c) => c,
        Err(e) => return CheckResult::skipped(check_name, e),
    };
    let mut violations = Vec::new();
    let mut failing_commits = Vec::new();

    for commit in &commits {
        let analysis = analyze_commit(commit, correlation_config, ctx.root);
        if analysis.is_test_only {
            continue;
        }
        let paths: Vec<PathBuf> = analysis
            .source_without_tests
            .iter()
            .filter(|p| {
                !should_skip_path(
                    p,
                    config.placeholders == "allow",
                    DiffRange::Commit(&commit.hash),
                    ctx.root,
                )
            })
            .cloned()
            .collect();
        if !paths.is_empty() {
            failing_commits.push(analysis.hash.clone());
            violations.extend(build_violations(
                &paths,
                &commit.changes,
                ctx,
                Some(truncate_hash(&analysis.hash)),
            ));
        }
        if ctx.limit.is_some_and(|l| violations.len() >= l) {
            break;
        }
    }

    failing_commits.sort();
    failing_commits.dedup();
    let metrics = json!({ "commits_checked": commits.len(), "commits_failing": failing_commits.len(), "scope": "commit" });
    finalize_with_placeholders(violations, ctx, metrics, check_name)
}
