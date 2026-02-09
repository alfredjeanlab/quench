// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Source/test file correlation logic.
//!
//! Orchestrates file classification and test matching to determine
//! whether changed source files have corresponding test changes.

mod check;
mod classify;
mod diff;
mod matching;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::diff::{CommitChanges, FileChange};
use classify::{CompiledPatterns, classify_changes};
use matching::{correlation_base_name, extract_base_name, is_test_only};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

// Re-export check functions
pub use check::{check_branch_scope, check_commit_scope, missing_tests_advice};

// Re-export diff analysis
pub use diff::{DiffRange, changes_in_cfg_test, has_inline_test_changes};

// Re-export matching types and functions
pub use matching::{
    TestIndex, candidate_js_test_paths, candidate_test_paths, find_test_locations,
    has_correlated_test,
};

/// Configuration for correlation detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationConfig {
    /// Patterns that identify test files.
    pub test_patterns: Vec<String>,
    /// Patterns that identify source files.
    pub source_patterns: Vec<String>,
    /// Files excluded from requiring tests.
    pub exclude_patterns: Vec<String>,
}

/// Result of correlation analysis.
#[derive(Debug)]
pub struct CorrelationResult {
    /// Source files that have corresponding test changes.
    pub with_tests: Vec<PathBuf>,
    /// Source files missing test changes.
    pub without_tests: Vec<PathBuf>,
    /// Test-only changes (TDD workflow).
    pub test_only: Vec<PathBuf>,
}

/// Result of analyzing a single commit for correlation.
#[derive(Debug)]
pub struct CommitAnalysis {
    /// Commit hash.
    pub hash: String,
    /// Commit message (first line).
    pub message: String,
    /// Source files in this commit without corresponding test changes.
    pub source_without_tests: Vec<PathBuf>,
    /// True if this commit contains only test changes (TDD workflow).
    pub is_test_only: bool,
}

/// Analyze a single commit for source/test correlation.
///
/// Returns analysis of whether the commit follows proper test hygiene:
/// - TDD commits (test-only) are considered valid
/// - Commits with source changes must have corresponding test changes
pub fn analyze_commit(
    commit: &CommitChanges,
    config: &CorrelationConfig,
    root: &Path,
) -> CommitAnalysis {
    let result = analyze_correlation(&commit.changes, config, root);

    // A TDD commit has test changes but no source changes
    let is_test_only = !result.test_only.is_empty()
        && result.with_tests.is_empty()
        && result.without_tests.is_empty();

    CommitAnalysis {
        hash: commit.hash.clone(),
        message: commit.message.clone(),
        source_without_tests: result.without_tests,
        is_test_only,
    }
}

/// Analyze changes for source/test correlation.
pub fn analyze_correlation(
    changes: &[FileChange],
    config: &CorrelationConfig,
    root: &Path,
) -> CorrelationResult {
    // Early termination: empty changes
    if changes.is_empty() {
        return CorrelationResult { with_tests: vec![], without_tests: vec![], test_only: vec![] };
    }

    let patterns =
        CompiledPatterns::from_config(config).unwrap_or_else(|_| CompiledPatterns::empty());

    // Classify changes (parallel for large sets)
    let (source_changes, test_changes) = classify_changes(changes, &patterns, root);

    // Early termination: no source changes
    if source_changes.is_empty() {
        return CorrelationResult {
            with_tests: vec![],
            without_tests: vec![],
            test_only: test_changes,
        };
    }

    // Early termination: single source file (inline lookup, skip index build)
    if source_changes.len() == 1 {
        return analyze_single_source(source_changes[0], test_changes, root);
    }

    // Build test index for O(1) lookups
    let test_index = TestIndex::new(&test_changes);

    // Analyze each source file
    let mut with_tests = Vec::new();
    let mut without_tests = Vec::new();

    for source in &source_changes {
        let rel_path = source.path.strip_prefix(root).unwrap_or(&source.path);

        // Use indexed lookups (O(1) base name + location check)
        let has_test =
            test_index.has_test_for(rel_path) || test_index.has_test_at_location(rel_path);

        // Check if the source file itself appears in test changes (inline #[cfg(test)] blocks)
        let has_inline_test = test_index.has_inline_test(rel_path);

        if has_test || has_inline_test {
            with_tests.push(rel_path.to_path_buf());
        } else {
            without_tests.push(rel_path.to_path_buf());
        }
    }

    // Test-only changes (no corresponding source changes)
    let source_base_names: HashSet<String> = with_tests
        .iter()
        .chain(without_tests.iter())
        .filter_map(|p| correlation_base_name(p).map(|s| s.to_string()))
        .collect();

    let test_only: Vec<PathBuf> = test_changes
        .into_iter()
        .filter(|t| {
            let test_base = extract_base_name(t).unwrap_or_default();
            is_test_only(&test_base, &source_base_names)
        })
        .collect();

    CorrelationResult { with_tests, without_tests, test_only }
}

/// Optimized analysis for a single source file.
///
/// Avoids building TestIndex when there's only one source file to check.
fn analyze_single_source(
    source: &FileChange,
    test_changes: Vec<PathBuf>,
    root: &Path,
) -> CorrelationResult {
    let rel_path = source.path.strip_prefix(root).unwrap_or(&source.path);

    // Extract test base names for matching
    let test_base_names: Vec<String> =
        test_changes.iter().filter_map(|p| extract_base_name(p)).collect();

    // Use the existing correlation check (efficient for single file)
    let has_test = has_correlated_test(rel_path, &test_changes, &test_base_names);

    // Check if the source file itself appears in test changes
    let has_inline_test = test_changes.iter().any(|t| t == rel_path);

    let (with_tests, without_tests) = if has_test || has_inline_test {
        (vec![rel_path.to_path_buf()], vec![])
    } else {
        (vec![], vec![rel_path.to_path_buf()])
    };

    // Determine test-only changes using shared helper
    let source_base_names: HashSet<String> = correlation_base_name(rel_path)
        .map(|s| {
            let mut set = HashSet::new();
            set.insert(s.to_string());
            set
        })
        .unwrap_or_default();

    let test_only: Vec<PathBuf> = test_changes
        .into_iter()
        .filter(|t| {
            let test_base = extract_base_name(t).unwrap_or_default();
            is_test_only(&test_base, &source_base_names)
        })
        .collect();

    CorrelationResult { with_tests, without_tests, test_only }
}
