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

/// Find all areas that match the changed files based on source patterns.
fn find_areas_from_source<'a>(
    changed_files: &[String],
    areas: &'a HashMap<String, DocsAreaConfig>,
) -> Vec<(&'a str, &'a DocsAreaConfig)> {
    areas
        .iter()
        .filter_map(|(name, area)| {
            // Only consider areas with source patterns
            let source = area.source.as_ref()?;

            // Check if any changed file matches this area's source pattern
            if has_changes_matching(changed_files, source) {
                Some((name.as_str(), area))
            } else {
                None
            }
        })
        .collect()
}

/// Result of checking if a commit has required docs.
#[derive(Debug)]
pub struct DocCheckResult {
    /// Whether the commit has all required documentation.
    pub has_docs: bool,
    /// Areas that matched (by scope or source).
    pub matched_areas: Vec<MatchedArea>,
}

/// An area that was matched for a commit.
#[derive(Debug, Clone)]
pub struct MatchedArea {
    /// Area name (e.g., "api").
    pub name: String,
    /// Required docs pattern (e.g., "docs/api/**").
    pub docs_pattern: String,
    /// How this area was matched.
    pub match_type: AreaMatchType,
    /// Whether docs were found for this area.
    pub has_docs: bool,
}

/// How an area was matched to a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaMatchType {
    /// Matched by commit scope (e.g., `feat(api):` → "api" area).
    Scope,
    /// Matched by source file changes (e.g., `src/api/**` changed).
    Source,
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
    let mut result = CommitValidation { violations: Vec::new(), feature_commits: 0, with_docs: 0 };

    // Get branch commits
    let commits = match get_branch_commits(root, base) {
        Ok(c) => c,
        Err(_) => return result, // Git error, skip check
    };

    // Filter to feature commits
    let feature_commits: Vec<_> =
        commits.into_iter().filter(|c| config.types.contains(&c.commit_type)).collect();

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
        let doc_result = check_commit_has_docs(commit, &changed_files, areas);
        if doc_result.has_docs {
            result.with_docs += 1;
        } else {
            // Create violations for each missing area
            let violations = create_violations_for_commit(commit, &doc_result);
            result.violations.extend(violations);
        }
    }

    result
}

fn check_commit_has_docs(
    commit: &ConventionalCommit,
    changed_files: &[String],
    areas: &HashMap<String, DocsAreaConfig>,
) -> DocCheckResult {
    let mut matched_areas = Vec::new();

    // Priority 1: Check scope-based matching
    if let Some(scope) = &commit.scope
        && let Some(area) = areas.get(scope)
    {
        let has_docs = has_changes_matching(changed_files, &area.docs);
        matched_areas.push(MatchedArea {
            name: scope.clone(),
            docs_pattern: area.docs.clone(),
            match_type: AreaMatchType::Scope,
            has_docs,
        });

        // Scope match takes priority - don't add source matches for same area
        return DocCheckResult { has_docs, matched_areas };
    }

    // Priority 2: Check source-based matching
    let source_matches = find_areas_from_source(changed_files, areas);
    if !source_matches.is_empty() {
        for (name, area) in source_matches {
            let has_docs = has_changes_matching(changed_files, &area.docs);
            matched_areas.push(MatchedArea {
                name: name.to_string(),
                docs_pattern: area.docs.clone(),
                match_type: AreaMatchType::Source,
                has_docs,
            });
        }

        let all_have_docs = matched_areas.iter().all(|a| a.has_docs);
        return DocCheckResult { has_docs: all_have_docs, matched_areas };
    }

    // Fallback: No area matched, require generic docs/
    let has_docs = has_changes_matching(changed_files, "docs/**");
    DocCheckResult {
        has_docs,
        matched_areas, // Empty - no specific area
    }
}

fn create_area_violation(commit: &ConventionalCommit, area: &MatchedArea) -> Violation {
    let match_desc = match area.match_type {
        AreaMatchType::Scope => format!("feat({}):", area.name),
        AreaMatchType::Source => format!("changes in {} area", area.name),
    };

    let advice = format!(
        "Commit {} requires documentation update.\nUpdate {} with the new functionality.",
        match_desc, area.docs_pattern
    );

    Violation::commit_violation(&commit.hash, &commit.message, "missing_docs", advice)
        .with_expected_docs(&area.docs_pattern)
        .with_area(
            &area.name,
            match area.match_type {
                AreaMatchType::Scope => "scope",
                AreaMatchType::Source => "source",
            },
        )
}

fn create_violations_for_commit(
    commit: &ConventionalCommit,
    result: &DocCheckResult,
) -> Vec<Violation> {
    if result.has_docs {
        return Vec::new();
    }

    if result.matched_areas.is_empty() {
        // No specific area, generic docs/ violation
        return vec![create_violation(commit, None)];
    }

    // Create violation for each area missing docs
    result
        .matched_areas
        .iter()
        .filter(|a| !a.has_docs)
        .map(|a| create_area_violation(commit, a))
        .collect()
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
