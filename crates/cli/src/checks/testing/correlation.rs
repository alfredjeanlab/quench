// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Source/test file correlation logic.

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use serde_json::json;

use crate::adapter::{Adapter, FileKind, GenericAdapter};
use crate::check::{CheckContext, CheckResult, Violation};
use crate::checks::placeholders::{
    PlaceholderMetrics, collect_placeholder_metrics, default_js_patterns, default_rust_patterns,
};

use super::diff::{ChangeType, CommitChanges, FileChange, get_base_changes, get_commits_since, get_staged_changes};
use super::patterns::{self, Language, detect_language, candidate_test_paths_for};
use super::placeholder::{has_js_placeholder_test, has_placeholder_test};

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

        // Check with common suffixes
        for suffix in patterns::TEST_SUFFIXES {
            if self
                .base_names
                .contains(&format!("{}{}", base_name, suffix))
            {
                return true;
            }
        }

        // Check with common prefixes
        for prefix in patterns::TEST_PREFIXES {
            if self
                .base_names
                .contains(&format!("{}{}", prefix, base_name))
            {
                return true;
            }
        }

        false
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
    !source_base_names
        .iter()
        .any(|source_base| patterns::matches_base_name(test_base, source_base))
}

/// Get candidate test file paths for a base name (Rust).
///
/// Returns patterns like: tests/{base}_tests.rs, tests/{base}_test.rs, etc.
///
/// Prefer using `patterns::candidate_test_paths_for()` for language-aware path generation.
pub fn candidate_test_paths(base_name: &str) -> Vec<String> {
    // Delegate to patterns module's internal Rust path generator
    // by creating a fake Rust path
    patterns::candidate_test_paths_for(Path::new(&format!("{}.rs", base_name)))
}

/// Get candidate test file paths for a base name (JavaScript/TypeScript).
///
/// Prefer using `patterns::candidate_test_paths_for()` for language-aware path generation.
pub fn candidate_js_test_paths(base_name: &str) -> Vec<String> {
    // Delegate to patterns module's internal JS path generator
    // by creating a fake JS path
    patterns::candidate_test_paths_for(Path::new(&format!("{}.ts", base_name)))
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

    // Strategy 2: Base name matching
    test_base_names
        .iter()
        .any(|test_name| patterns::matches_base_name(test_name, base_name))
}

/// Extract the normalized base name from a file path.
///
/// Returns the file stem with test affixes stripped.
/// This is the canonical function for extracting base names from test file paths.
///
/// Examples:
/// - "tests/parser_tests.rs" -> "parser"
/// - "test_utils.rs" -> "utils"
/// - "src/parser.rs" -> "parser"
fn file_base_name(path: &Path) -> Option<&str> {
    let stem = path.file_stem()?.to_str()?;
    Some(strip_test_affixes(stem))
}

/// Extract base name from a test file, stripping test suffixes.
///
/// This is a convenience wrapper that returns an owned String.
fn extract_base_name(path: &Path) -> Option<String> {
    file_base_name(path).map(|s| s.to_string())
}

/// Strip test-related suffixes and prefixes from a file stem.
///
/// Examples:
/// - "parser_tests" -> "parser"
/// - "test_parser" -> "parser"
/// - "parser.test" -> "parser"
/// - "parser" -> "parser" (unchanged)
fn strip_test_affixes(stem: &str) -> &str {
    for suffix in patterns::ALL_TEST_SUFFIXES {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            return stripped;
        }
    }
    for prefix in patterns::TEST_PREFIXES {
        if let Some(stripped) = stem.strip_prefix(prefix) {
            return stripped;
        }
    }
    stem
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

// =============================================================================
// Public Correlation Checking API
// =============================================================================

const RUST_EXT: &str = "rs";
const SHORT_HASH_LEN: usize = 7;

fn truncate_hash(hash: &str) -> &str {
    if hash.len() >= SHORT_HASH_LEN { &hash[..SHORT_HASH_LEN] } else { hash }
}

pub(super) fn missing_tests_advice(file_stem: &str, lang: Language) -> String {
    match lang {
        Language::Rust => format!("Add tests in tests/{}_tests.rs or update inline #[cfg(test)] block", file_stem),
        Language::Go => format!("Add tests in {}_test.go", file_stem),
        Language::JavaScript => format!("Add tests in {}.test.ts or __tests__/{}.test.ts", file_stem, file_stem),
        Language::Python => format!("Add tests in test_{}.py or tests/test_{}.py", file_stem, file_stem),
        Language::Unknown => format!("Add tests for {}", file_stem),
    }
}

fn should_skip_path(path: &Path, allow_placeholders: bool, diff_range: DiffRange, root: &Path) -> bool {
    (path.extension().is_some_and(|e| e == RUST_EXT) && has_inline_test_changes(path, root, diff_range))
        || (allow_placeholders && has_placeholder_for_source(path, root))
}

fn has_placeholder_for_source(source_path: &Path, root: &Path) -> bool {
    let Some(base_name) = source_path.file_stem().and_then(|s| s.to_str()) else { return false };
    let lang = detect_language(source_path);
    candidate_test_paths_for(source_path).iter().any(|test_path| {
        let test_file = Path::new(test_path);
        root.join(test_file).exists() && match lang {
            Language::JavaScript => has_js_placeholder_test(test_file, base_name, root).unwrap_or(false),
            Language::Rust => has_placeholder_test(test_file, base_name, root).unwrap_or(false),
            _ => false,
        }
    })
}

fn get_diff_range<'a>(ctx: &'a CheckContext) -> DiffRange<'a> {
    if ctx.staged { DiffRange::Staged } else { ctx.base_branch.map(DiffRange::Branch).unwrap_or(DiffRange::Staged) }
}

fn build_violations(paths: &[PathBuf], changes: &[FileChange], ctx: &CheckContext, commit_hash: Option<&str>) -> Vec<Violation> {
    let mut violations = Vec::new();
    for path in paths {
        let change = changes.iter().find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path) == path);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let lang = detect_language(path);
        let test_advice = missing_tests_advice(file_stem, lang);
        let advice = commit_hash.map_or(test_advice.clone(), |hash| {
            format!("Commit {} modifies {} without test changes. {}", hash, path.display(), test_advice)
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
        if ctx.limit.is_some_and(|l| violations.len() >= l) { break; }
    }
    violations
}

fn collect_test_file_placeholder_metrics(ctx: &CheckContext) -> PlaceholderMetrics {
    let test_patterns = if ctx.config.project.tests.is_empty() {
        vec!["**/tests/**".to_string(), "**/test/**".to_string(), "**/*_test.*".to_string(), "**/*_tests.*".to_string(), "**/*.test.*".to_string(), "**/*.spec.*".to_string()]
    } else {
        ctx.config.project.tests.clone()
    };
    let file_adapter = GenericAdapter::new(&[], &test_patterns);
    let test_files: Vec<PathBuf> = ctx.files.iter()
        .filter(|f| file_adapter.classify(f.path.strip_prefix(ctx.root).unwrap_or(&f.path)) == FileKind::Test)
        .map(|f| f.path.clone()).collect();
    collect_placeholder_metrics(&test_files, &default_rust_patterns(), &default_js_patterns())
}

fn finalize_with_placeholders(violations: Vec<Violation>, ctx: &CheckContext, mut metrics: serde_json::Value, check_name: &str) -> CheckResult {
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

pub fn check_branch_scope(check_name: &str, ctx: &CheckContext, correlation_config: &CorrelationConfig) -> CheckResult {
    let config = &ctx.config.check.tests.commit;
    let changes = if ctx.staged {
        match get_staged_changes(ctx.root) { Ok(c) => c, Err(e) => return CheckResult::skipped(check_name, e) }
    } else if let Some(base) = ctx.base_branch {
        match get_base_changes(ctx.root, base) { Ok(c) => c, Err(e) => return CheckResult::skipped(check_name, e) }
    } else {
        return finalize_with_placeholders(vec![], ctx, json!({}), check_name);
    };

    let mut result = analyze_correlation(&changes, correlation_config, ctx.root);
    result.without_tests.retain(|path| !should_skip_path(path, config.placeholders == "allow", get_diff_range(ctx), ctx.root));
    let violations = build_violations(&result.without_tests, &changes, ctx, None);
    let metrics = json!({
        "source_files_changed": result.with_tests.len() + result.without_tests.len(),
        "with_test_changes": result.with_tests.len(),
        "without_test_changes": result.without_tests.len(),
        "scope": "branch",
    });
    finalize_with_placeholders(violations, ctx, metrics, check_name)
}

pub fn check_commit_scope(check_name: &str, ctx: &CheckContext, base: &str, correlation_config: &CorrelationConfig) -> CheckResult {
    let config = &ctx.config.check.tests.commit;
    let commits = match get_commits_since(ctx.root, base) { Ok(c) => c, Err(e) => return CheckResult::skipped(check_name, e) };
    let mut violations = Vec::new();
    let mut failing_commits = Vec::new();

    for commit in &commits {
        let analysis = analyze_commit(commit, correlation_config, ctx.root);
        if analysis.is_test_only { continue; }
        let paths: Vec<PathBuf> = analysis.source_without_tests.iter()
            .filter(|p| !should_skip_path(p, config.placeholders == "allow", DiffRange::Commit(&commit.hash), ctx.root))
            .cloned().collect();
        if !paths.is_empty() {
            failing_commits.push(analysis.hash.clone());
            violations.extend(build_violations(&paths, &commit.changes, ctx, Some(truncate_hash(&analysis.hash))));
        }
        if ctx.limit.is_some_and(|l| violations.len() >= l) { break; }
    }

    failing_commits.sort();
    failing_commits.dedup();
    let metrics = json!({ "commits_checked": commits.len(), "commits_failing": failing_commits.len(), "scope": "commit" });
    finalize_with_placeholders(violations, ctx, metrics, check_name)
}

#[cfg(test)]
#[path = "correlation_tests.rs"]
mod tests;
