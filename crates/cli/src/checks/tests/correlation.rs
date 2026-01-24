// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Source/test file correlation logic.

use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::diff::{ChangeType, CommitChanges, FileChange};

/// Configuration for correlation detection.
#[derive(Debug, Clone)]
pub struct CorrelationConfig {
    /// Patterns that identify test files.
    pub test_patterns: Vec<String>,
    /// Patterns that identify source files.
    pub source_patterns: Vec<String>,
    /// Files excluded from requiring tests.
    pub exclude_patterns: Vec<String>,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            test_patterns: vec![
                "tests/**/*".to_string(),
                "test/**/*".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.spec.*".to_string(),
            ],
            source_patterns: vec!["src/**/*".to_string()],
            exclude_patterns: vec![
                "**/mod.rs".to_string(),
                "**/lib.rs".to_string(),
                "**/main.rs".to_string(),
                "**/generated/**".to_string(),
            ],
        }
    }
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
    let test_patterns = build_glob_set(&config.test_patterns).unwrap_or_else(|_| empty_glob_set());
    let source_patterns =
        build_glob_set(&config.source_patterns).unwrap_or_else(|_| empty_glob_set());
    let exclude_patterns =
        build_glob_set(&config.exclude_patterns).unwrap_or_else(|_| empty_glob_set());

    // Classify changes
    let mut source_changes: Vec<&FileChange> = Vec::new();
    let mut test_changes: Vec<PathBuf> = Vec::new();

    for change in changes {
        // Skip deleted files - they don't require tests
        if change.change_type == ChangeType::Deleted {
            continue;
        }

        // Get relative path for pattern matching
        let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);

        if test_patterns.is_match(rel_path) {
            test_changes.push(rel_path.to_path_buf());
        } else if source_patterns.is_match(rel_path) {
            // Check if excluded
            if !exclude_patterns.is_match(rel_path) {
                source_changes.push(change);
            }
        }
    }

    // Extract test base names for matching
    let test_base_names: Vec<String> = test_changes
        .iter()
        .filter_map(|p| extract_base_name(p))
        .collect();

    // Analyze each source file
    let mut with_tests = Vec::new();
    let mut without_tests = Vec::new();

    for source in source_changes {
        let rel_path = source.path.strip_prefix(root).unwrap_or(&source.path);

        // Use the enhanced correlation check
        let has_test = has_correlated_test(rel_path, &test_changes, &test_base_names);

        // Also check if the source file itself appears in test changes
        // (for inline #[cfg(test)] blocks)
        let has_inline_test = test_changes.iter().any(|t| t == rel_path);

        if has_test || has_inline_test {
            with_tests.push(rel_path.to_path_buf());
        } else {
            without_tests.push(rel_path.to_path_buf());
        }
    }

    // Test-only changes (no corresponding source changes)
    let source_base_names: Vec<String> = with_tests
        .iter()
        .chain(without_tests.iter())
        .filter_map(|p| correlation_base_name(p).map(|s| s.to_string()))
        .collect();

    let test_only: Vec<PathBuf> = test_changes
        .into_iter()
        .filter(|t| {
            let test_base = extract_base_name(t).unwrap_or_default();
            !source_base_names.iter().any(|s| {
                test_base == *s
                    || test_base == format!("{}_test", s)
                    || test_base == format!("{}_tests", s)
                    || test_base == format!("test_{}", s)
            })
        })
        .collect();

    CorrelationResult {
        with_tests,
        without_tests,
        test_only,
    }
}

/// Extract the base name for correlation (e.g., "parser" from "src/parser.rs").
fn correlation_base_name(path: &Path) -> Option<&str> {
    path.file_stem()?.to_str()
}

/// Get candidate test file locations for a source file.
///
/// Returns a list of paths where a test file might exist for the given source file.
/// This implements the test location strategy from the spec:
/// 1. tests/{base}.rs
/// 2. tests/{base}_test.rs
/// 3. tests/{base}_tests.rs
/// 4. tests/test_{base}.rs
/// 5. test/{base}.rs (singular)
/// 6. test/{base}_test.rs
/// 7. test/{base}_tests.rs
/// 8. Sibling test files ({parent}/{base}_test.rs, {parent}/{base}_tests.rs)
pub fn find_test_locations(source_path: &Path) -> Vec<PathBuf> {
    let base_name = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return vec![],
    };
    let parent = source_path.parent().unwrap_or(Path::new(""));

    vec![
        // tests/ directory variants
        PathBuf::from(format!("tests/{}.rs", base_name)),
        PathBuf::from(format!("tests/{}_test.rs", base_name)),
        PathBuf::from(format!("tests/{}_tests.rs", base_name)),
        PathBuf::from(format!("tests/test_{}.rs", base_name)),
        // test/ directory variants (singular)
        PathBuf::from(format!("test/{}.rs", base_name)),
        PathBuf::from(format!("test/{}_test.rs", base_name)),
        PathBuf::from(format!("test/{}_tests.rs", base_name)),
        // Sibling test files (same directory as source)
        parent.join(format!("{}_test.rs", base_name)),
        parent.join(format!("{}_tests.rs", base_name)),
    ]
}

/// Check if any changed test file correlates with a source file.
///
/// Uses two strategies:
/// 1. Check if any test path matches expected locations for this source
/// 2. Fall back to base name matching
pub fn has_correlated_test(
    source_path: &Path,
    test_changes: &[PathBuf],
    test_base_names: &[String],
) -> bool {
    let base_name = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Strategy 1: Check expected test locations
    let expected_locations = find_test_locations(source_path);
    for test_path in test_changes {
        if expected_locations
            .iter()
            .any(|loc| test_path.ends_with(loc))
        {
            return true;
        }
    }

    // Strategy 2: Base name matching (existing logic)
    test_base_names.iter().any(|test_name| {
        test_name == base_name
            || *test_name == format!("{}_test", base_name)
            || *test_name == format!("{}_tests", base_name)
            || *test_name == format!("test_{}", base_name)
    })
}

/// Extract base name from a test file, stripping test suffixes.
fn extract_base_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;

    // Strip common test suffixes
    let base = stem
        .strip_suffix("_tests")
        .or_else(|| stem.strip_suffix("_test"))
        .or_else(|| stem.strip_prefix("test_"))
        .unwrap_or(stem);

    Some(base.to_string())
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| e.to_string())?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}

/// Create an empty GlobSet that matches nothing.
fn empty_glob_set() -> GlobSet {
    GlobSet::empty()
}

/// Check if a test file contains placeholder tests for a given source file.
///
/// Placeholder tests are `#[test]` `#[ignore = "..."]` patterns that indicate
/// planned test implementation.
pub fn has_placeholder_test(
    test_path: &Path,
    source_base: &str,
    root: &Path,
) -> Result<bool, String> {
    let full_path = root.join(test_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let placeholders = find_placeholder_tests(&content);

    // Check if any placeholder test name relates to the source file
    Ok(placeholders.iter().any(|test_name| {
        test_name.contains(source_base)
            || test_name.contains(&format!("test_{}", source_base))
            || test_name.contains(&format!("{}_test", source_base))
    }))
}

/// Parse Rust test file for placeholder tests.
///
/// Looks for patterns like:
///
/// ```text
/// #[test]
/// #[ignore = "TODO: implement parser"]
/// fn test_parser() { ... }
/// ```
fn find_placeholder_tests(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut saw_test_attr = false;
    let mut saw_ignore_attr = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "#[test]" {
            saw_test_attr = true;
            saw_ignore_attr = false;
            continue;
        }

        if saw_test_attr && (trimmed.starts_with("#[ignore") || trimmed.starts_with("#[ignore =")) {
            saw_ignore_attr = true;
            continue;
        }

        if saw_test_attr
            && saw_ignore_attr
            && trimmed.starts_with("fn ")
            && let Some(name_part) = trimmed.strip_prefix("fn ")
            && let Some(name) = name_part.split('(').next()
        {
            result.push(name.to_string());
            saw_test_attr = false;
            saw_ignore_attr = false;
            continue;
        }

        // Reset if we see something else
        if !trimmed.starts_with('#') && !trimmed.is_empty() {
            saw_test_attr = false;
            saw_ignore_attr = false;
        }
    }

    result
}

/// Check if a Rust source file has inline test changes (#[cfg(test)] blocks).
///
/// Returns true if the file's diff contains changes within a #[cfg(test)] module.
pub fn has_inline_test_changes(file_path: &Path, root: &Path, base: Option<&str>) -> bool {
    let diff_content = match get_file_diff(file_path, root, base) {
        Ok(content) => content,
        Err(_) => return false,
    };

    changes_in_cfg_test(&diff_content)
}

/// Get the diff for a specific file.
fn get_file_diff(file_path: &Path, root: &Path, base: Option<&str>) -> Result<String, String> {
    use std::process::Command;

    let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
    let rel_path_str = rel_path
        .to_str()
        .ok_or_else(|| "invalid path".to_string())?;

    let range = base.map(|b| format!("{}..HEAD", b));
    let args: Vec<&str> = match &range {
        Some(r) => vec!["diff", r.as_str(), "--", rel_path_str],
        None => vec!["diff", "--cached", "--", rel_path_str],
    };

    let output = Command::new("git")
        .args(&args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse diff content to detect if changes are within #[cfg(test)] blocks.
///
/// Tracks state machine:
/// - Looking for `#[cfg(test)]` marker
/// - Once found, track brace depth to identify block extent
/// - Check if any `+` lines are within the block
pub fn changes_in_cfg_test(diff_content: &str) -> bool {
    let mut in_cfg_test = false;
    let mut brace_depth = 0;
    let mut found_changes_in_test = false;

    for line in diff_content.lines() {
        // Skip diff metadata lines
        if line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@ ")
        {
            continue;
        }

        // Get the actual content (strip +/- prefix for analysis)
        let content = line
            .strip_prefix('+')
            .or_else(|| line.strip_prefix('-'))
            .or_else(|| line.strip_prefix(' '))
            .unwrap_or(line);

        let trimmed = content.trim();

        // Detect #[cfg(test)] marker
        if trimmed.contains("#[cfg(test)]") {
            in_cfg_test = true;
            brace_depth = 0;
            continue;
        }

        // Track brace depth when inside cfg(test)
        if in_cfg_test {
            // Count braces in content
            for ch in content.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth <= 0 {
                            in_cfg_test = false;
                        }
                    }
                    _ => {}
                }
            }

            // Check if this is an added line within the test block
            if line.starts_with('+') && brace_depth > 0 {
                found_changes_in_test = true;
            }
        }
    }

    found_changes_in_test
}

#[cfg(test)]
#[path = "correlation_tests.rs"]
mod tests;
