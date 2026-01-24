// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Commit checking for documentation validation.
//!
//! Validates that feature commits (feat:, breaking:, etc.) have corresponding
//! documentation updates. Only runs in CI mode.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use regex::Regex;

use crate::check::{CheckContext, Violation};
use crate::config::{DocsAreaConfig, DocsCommitConfig};

/// A parsed conventional commit.
#[derive(Debug)]
pub struct ConventionalCommit {
    /// Short commit hash (7 chars).
    pub hash: String,
    /// Commit type (feat, fix, etc.).
    pub commit_type: String,
    /// Optional scope in parentheses.
    pub scope: Option<String>,
    /// Full commit message (type(scope): description).
    pub message: String,
}

/// Get commits on current branch not in base branch.
pub fn get_branch_commits(root: &Path, base: &str) -> Result<Vec<ConventionalCommit>, String> {
    let output = Command::new("git")
        .args(["log", "--format=%H %s", &format!("{}..HEAD", base)])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run git log: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter_map(parse_commit_line).collect())
}

/// Parse a commit line into conventional commit parts.
fn parse_commit_line(line: &str) -> Option<ConventionalCommit> {
    let (hash, message) = line.split_once(' ')?;

    // Parse conventional commit: type(scope): message or type: message
    let re = Regex::new(r"^(\w+)(?:\(([^)]+)\))?:\s*(.+)$").ok()?;
    let caps = re.captures(message)?;

    Some(ConventionalCommit {
        hash: hash.chars().take(7).collect(), // Short hash
        commit_type: caps.get(1)?.as_str().to_lowercase(),
        scope: caps.get(2).map(|m| m.as_str().to_string()),
        message: message.to_string(),
    })
}

/// Get files changed on current branch vs base.
pub fn get_changed_files(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{}..HEAD", base)])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(String::from).collect())
}

/// Check if any changed files match a glob pattern.
pub fn has_changes_matching(changed_files: &[String], pattern: &str) -> bool {
    let Ok(glob) = globset::Glob::new(pattern) else {
        return false;
    };
    let matcher = glob.compile_matcher();
    changed_files.iter().any(|f| matcher.is_match(f))
}

/// Entry point for commit validation from docs check.
pub fn validate_commit_docs(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let commit_config = &ctx.config.check.docs.commit;

    // Skip if disabled
    if commit_config.check == "off" {
        return;
    }

    // Need base branch for comparison
    let Some(base) = ctx.base_branch else {
        return;
    };

    let areas = &ctx.config.check.docs.area;
    let result = validate_commits(ctx.root, base, commit_config, areas);

    // Collect violations
    for v in result.violations {
        if ctx.limit.is_some_and(|l| violations.len() >= l) {
            break;
        }
        violations.push(v);
    }
}

/// Result of commit validation.
pub struct CommitValidation {
    pub violations: Vec<Violation>,
    pub feature_commits: usize,
    pub with_docs: usize,
}

/// Validate that feature commits have documentation.
pub fn validate_commits(
    root: &Path,
    base: &str,
    config: &DocsCommitConfig,
    areas: &HashMap<String, DocsAreaConfig>,
) -> CommitValidation {
    let mut result = CommitValidation {
        violations: Vec::new(),
        feature_commits: 0,
        with_docs: 0,
    };

    // Get branch commits
    let commits = match get_branch_commits(root, base) {
        Ok(c) => c,
        Err(_) => return result, // Git error, skip check
    };

    // Filter to feature commits
    let feature_commits: Vec<_> = commits
        .into_iter()
        .filter(|c| config.types.contains(&c.commit_type))
        .collect();

    result.feature_commits = feature_commits.len();

    if feature_commits.is_empty() {
        return result;
    }

    // Get changed files
    let changed_files = match get_changed_files(root, base) {
        Ok(f) => f,
        Err(_) => return result,
    };

    // Check each feature commit
    for commit in &feature_commits {
        let (has_docs, expected_pattern) = check_commit_has_docs(commit, &changed_files, areas);
        if has_docs {
            result.with_docs += 1;
        } else {
            result
                .violations
                .push(create_violation(commit, expected_pattern.as_deref()));
        }
    }

    result
}

fn check_commit_has_docs(
    commit: &ConventionalCommit,
    changed_files: &[String],
    areas: &HashMap<String, DocsAreaConfig>,
) -> (bool, Option<String>) {
    // If commit has scope, check for matching area
    if let Some(scope) = &commit.scope
        && let Some(area) = areas.get(scope)
    {
        let has_docs = has_changes_matching(changed_files, &area.docs);
        return (has_docs, Some(area.docs.clone()));
    }

    // Default: any change in docs/ satisfies requirement
    let has_docs = has_changes_matching(changed_files, "docs/**");
    (has_docs, None)
}

fn create_violation(commit: &ConventionalCommit, expected_docs: Option<&str>) -> Violation {
    let advice = match expected_docs {
        Some(pattern) => format!("Update {} with the new functionality.", pattern),
        None => "Update docs/ with the new functionality.".to_string(),
    };

    let mut v = Violation::commit_violation(&commit.hash, &commit.message, "missing_docs", advice);

    if let Some(docs) = expected_docs {
        v = v.with_expected_docs(docs);
    }

    v
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod tests;
