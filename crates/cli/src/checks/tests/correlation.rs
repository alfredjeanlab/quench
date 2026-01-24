// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Source/test file correlation logic.

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use super::diff::{ChangeType, CommitChanges, FileChange};

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

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            test_patterns: vec![
                "tests/**/*".to_string(),
                "test/**/*".to_string(),
                "spec/**/*".to_string(),
                "**/__tests__/**".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.test.*".to_string(),
                "**/*.spec.*".to_string(),
                "**/test_*.*".to_string(),
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

// Performance optimizations for O(1) test lookup and early termination paths.

/// Threshold for switching to parallel file classification.
/// Below this, sequential iteration is faster due to rayon overhead.
const PARALLEL_THRESHOLD: usize = 50;

/// Pre-computed test correlation index for O(1) lookups.
///
/// Build once per `analyze_correlation()` call, then use for all source files.
/// This avoids O(n*m) complexity when checking many source files against many tests.
pub struct TestIndex {
    /// All test file paths for direct matching
    all_paths: HashSet<PathBuf>,
    /// Normalized base names (stripped of _test/_tests suffixes)
    base_names: HashSet<String>,
}

impl TestIndex {
    /// Build a test index from a list of test file paths.
    ///
    /// The index enables O(1) lookups when checking if a source file has a corresponding test.
    pub fn new(test_changes: &[PathBuf]) -> Self {
        let mut base_names = HashSet::new();

        for path in test_changes {
            if let Some(base) = extract_base_name(path) {
                base_names.insert(base);
            }
        }

        Self {
            all_paths: test_changes.iter().cloned().collect(),
            base_names,
        }
    }

    /// O(1) check for correlated test by base name.
    ///
    /// Matching strategy:
    /// 1. Direct match: source "parser" matches test "parser"
    /// 2. Test suffix: source "parser" matches test "parser_test" or "parser_tests"
    /// 3. Test prefix: source "parser" matches test "test_parser"
    ///
    /// Note: Source files with test-like names (e.g., "test_utils.rs") are handled
    /// correctly because the source base name "test_utils" would need a test with
    /// base name "test_utils", "test_utils_test", "test_utils_tests", or
    /// "test_test_utils".
    pub fn has_test_for(&self, source_path: &Path) -> bool {
        let base_name = match source_path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => return false,
        };

        // Check direct base name match
        if self.base_names.contains(base_name) {
            return true;
        }

        // Check with common suffixes/prefixes
        self.base_names.contains(&format!("{}_test", base_name))
            || self.base_names.contains(&format!("{}_tests", base_name))
            || self.base_names.contains(&format!("test_{}", base_name))
    }

    /// Check if a test file exists at any of the expected locations for a source file.
    pub fn has_test_at_location(&self, source_path: &Path) -> bool {
        let expected_locations = find_test_locations(source_path);
        for test_path in &self.all_paths {
            if expected_locations
                .iter()
                .any(|loc| test_path.ends_with(loc))
            {
                return true;
            }
        }
        false
    }

    /// Check if the source path itself appears in test changes (for inline #[cfg(test)] blocks).
    pub fn has_inline_test(&self, rel_path: &Path) -> bool {
        self.all_paths.contains(rel_path)
    }
}

/// Cached GlobSets for common pattern configurations.
#[derive(Clone)]
struct CompiledPatterns {
    test_patterns: GlobSet,
    source_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl CompiledPatterns {
    fn from_config(config: &CorrelationConfig) -> Result<Self, String> {
        Ok(Self {
            test_patterns: build_glob_set(&config.test_patterns)?,
            source_patterns: build_glob_set(&config.source_patterns)?,
            exclude_patterns: build_glob_set(&config.exclude_patterns)?,
        })
    }

    fn empty() -> Self {
        Self {
            test_patterns: GlobSet::empty(),
            source_patterns: GlobSet::empty(),
            exclude_patterns: GlobSet::empty(),
        }
    }
}

/// Get cached patterns for the default configuration.
fn default_patterns() -> &'static CompiledPatterns {
    static PATTERNS: OnceLock<CompiledPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        // Default patterns are hardcoded and known to be valid, but we handle
        // the error case defensively by returning empty patterns.
        CompiledPatterns::from_config(&CorrelationConfig::default())
            .unwrap_or_else(|_| CompiledPatterns::empty())
    })
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
        return CorrelationResult {
            with_tests: vec![],
            without_tests: vec![],
            test_only: vec![],
        };
    }

    // Use cached patterns for default config, otherwise compile
    let patterns: Cow<'_, CompiledPatterns> = if *config == CorrelationConfig::default() {
        Cow::Borrowed(default_patterns())
    } else {
        Cow::Owned(
            CompiledPatterns::from_config(config).unwrap_or_else(|_| CompiledPatterns::empty()),
        )
    };

    // Classify changes (parallel for large sets)
    let (source_changes, test_changes) = classify_changes(changes, patterns.as_ref(), root);

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

    CorrelationResult {
        with_tests,
        without_tests,
        test_only,
    }
}

/// Classify changes into source and test files.
///
/// Uses parallel processing for large change sets (>= PARALLEL_THRESHOLD files).
fn classify_changes<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    if changes.len() >= PARALLEL_THRESHOLD {
        classify_changes_parallel(changes, patterns, root)
    } else {
        classify_changes_sequential(changes, patterns, root)
    }
}

/// Sequential classification for small change sets.
fn classify_changes_sequential<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    let mut source_changes: Vec<&FileChange> = Vec::new();
    let mut test_changes: Vec<PathBuf> = Vec::new();

    for change in changes {
        // Skip deleted files - they don't require tests
        if change.change_type == ChangeType::Deleted {
            continue;
        }

        // Get relative path for pattern matching
        let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);

        if patterns.test_patterns.is_match(rel_path) {
            test_changes.push(rel_path.to_path_buf());
        } else if patterns.source_patterns.is_match(rel_path)
            && !patterns.exclude_patterns.is_match(rel_path)
        {
            source_changes.push(change);
        }
    }

    (source_changes, test_changes)
}

/// Parallel classification for large change sets.
fn classify_changes_parallel<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    // Use rayon to classify in parallel
    let classified: Vec<_> = changes
        .par_iter()
        .filter(|c| c.change_type != ChangeType::Deleted)
        .filter_map(|change| {
            let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);

            if patterns.test_patterns.is_match(rel_path) {
                Some((None, Some(rel_path.to_path_buf())))
            } else if patterns.source_patterns.is_match(rel_path)
                && !patterns.exclude_patterns.is_match(rel_path)
            {
                Some((Some(change), None))
            } else {
                None
            }
        })
        .collect();

    // Separate into source and test changes
    let mut source_changes = Vec::new();
    let mut test_changes = Vec::new();

    for (source, test) in classified {
        if let Some(s) = source {
            source_changes.push(s);
        }
        if let Some(t) = test {
            test_changes.push(t);
        }
    }

    (source_changes, test_changes)
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
    let test_base_names: Vec<String> = test_changes
        .iter()
        .filter_map(|p| extract_base_name(p))
        .collect();

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

/// Check if a test file is considered "test-only" (no corresponding source change).
///
/// A test is test-only if its base name doesn't match any source file's base name,
/// even when accounting for common test suffixes/prefixes.
fn is_test_only(test_base: &str, source_base_names: &HashSet<String>) -> bool {
    // Direct match: test "parser" matches source "parser"
    if source_base_names.contains(test_base) {
        return false;
    }

    // Test has suffix that matches source
    // e.g., "parser_test" matches source "parser"
    for suffix in ["_test", "_tests"] {
        if let Some(stripped) = test_base.strip_suffix(suffix)
            && source_base_names.contains(stripped)
        {
            return false;
        }
    }

    // Test has prefix that matches source
    // e.g., "test_parser" matches source "parser"
    if let Some(stripped) = test_base.strip_prefix("test_")
        && source_base_names.contains(stripped)
    {
        return false;
    }

    // Source has suffix/prefix that matches test
    // e.g., source "parser" matches test "parser" (via _test, _tests, test_ variations)
    for source in source_base_names {
        if test_base == format!("{}_test", source)
            || test_base == format!("{}_tests", source)
            || test_base == format!("test_{}", source)
        {
            return false;
        }
    }

    true
}

/// Get candidate test file paths for a base name (Rust).
///
/// Returns patterns like: tests/{base}_tests.rs, tests/{base}_test.rs, etc.
/// Used for placeholder test checking.
pub fn candidate_test_paths(base_name: &str) -> Vec<String> {
    vec![
        format!("tests/{}_tests.rs", base_name),
        format!("tests/{}_test.rs", base_name),
        format!("tests/{}.rs", base_name),
        format!("test/{}_tests.rs", base_name),
        format!("test/{}_test.rs", base_name),
        format!("test/{}.rs", base_name),
    ]
}

/// Get candidate test file paths for a base name (JavaScript/TypeScript).
pub fn candidate_js_test_paths(base_name: &str) -> Vec<String> {
    let b = base_name;
    let exts = ["ts", "js"];
    let mut paths = Vec::with_capacity(16);
    for ext in &exts {
        paths.push(format!("{b}.test.{ext}"));
        paths.push(format!("{b}.spec.{ext}"));
        paths.push(format!("__tests__/{b}.test.{ext}"));
        paths.push(format!("tests/{b}.test.{ext}"));
    }
    paths
}

/// Get candidate test file locations for a source file.
pub fn find_test_locations(source_path: &Path) -> Vec<PathBuf> {
    let b = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return vec![],
    };
    let p = source_path.parent().unwrap_or(Path::new(""));
    let dirs = ["tests", "test"];
    let mut paths = Vec::with_capacity(11);
    for d in &dirs {
        paths.push(PathBuf::from(format!("{d}/{b}.rs")));
        paths.push(PathBuf::from(format!("{d}/{b}_test.rs")));
        paths.push(PathBuf::from(format!("{d}/{b}_tests.rs")));
    }
    paths.push(PathBuf::from(format!("tests/test_{b}.rs")));
    paths.push(p.join(format!("{b}_test.rs")));
    paths.push(p.join(format!("{b}_tests.rs")));
    paths
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

    // Strip common test suffixes/prefixes
    // Rust/Go style: _test, _tests, test_
    // JS/TS style: .test, .spec (part of stem for files like parser.test.ts)
    let base = stem
        .strip_suffix("_tests")
        .or_else(|| stem.strip_suffix("_test"))
        .or_else(|| stem.strip_suffix(".test"))
        .or_else(|| stem.strip_suffix(".spec"))
        .or_else(|| stem.strip_suffix("_spec"))
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

/// Specifies the git diff range for inline test detection.
#[derive(Debug, Clone, Copy)]
pub enum DiffRange<'a> {
    /// Staged changes (--cached)
    Staged,
    /// Branch changes (base..HEAD)
    Branch(&'a str),
    /// Single commit (hash^..hash)
    Commit(&'a str),
}

/// Check if a Rust source file has inline test changes (#[cfg(test)] blocks).
///
/// Returns true if the file's diff contains changes within a #[cfg(test)] module.
pub fn has_inline_test_changes(file_path: &Path, root: &Path, range: DiffRange<'_>) -> bool {
    let diff_content = match get_file_diff(file_path, root, range) {
        Ok(content) => content,
        Err(_) => return false,
    };

    changes_in_cfg_test(&diff_content)
}

/// Get the diff for a specific file.
fn get_file_diff(file_path: &Path, root: &Path, range: DiffRange<'_>) -> Result<String, String> {
    use std::process::Command;

    let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
    let rel_path_str = rel_path
        .to_str()
        .ok_or_else(|| "invalid path".to_string())?;

    let range_str = match range {
        DiffRange::Staged => String::new(),
        DiffRange::Branch(base) => format!("{}..HEAD", base),
        DiffRange::Commit(hash) => format!("{}^..{}", hash, hash),
    };

    let args: Vec<&str> = if range_str.is_empty() {
        vec!["diff", "--cached", "--", rel_path_str]
    } else {
        vec!["diff", &range_str, "--", rel_path_str]
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
