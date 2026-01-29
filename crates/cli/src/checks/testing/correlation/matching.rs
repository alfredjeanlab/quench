// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Source-to-test file matching.
//!
//! Given a source file, determine whether a corresponding test exists.
//! [`TestIndex`] provides O(1) base-name lookups for bulk matching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::super::patterns;

#[cfg(test)]
#[path = "matching_tests.rs"]
mod tests;

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

    /// Check if the source path itself appears in test changes (for inline tests).
    pub fn has_inline_test(&self, rel_path: &Path) -> bool {
        self.all_paths.contains(rel_path)
    }
}

/// Get candidate test file locations for a source file.
pub fn find_test_locations(source_path: &Path) -> Vec<PathBuf> {
    let Some(b) = source_path.file_stem().and_then(|s| s.to_str()) else {
        return vec![];
    };
    let p = source_path.parent().unwrap_or(Path::new(""));
    let mut paths = Vec::with_capacity(11);
    for d in ["tests", "test"] {
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

/// Get candidate test file paths for a base name (Rust).
///
/// Prefer using `patterns::candidate_test_paths_for()` for language-aware path generation.
pub fn candidate_test_paths(base_name: &str) -> Vec<String> {
    patterns::candidate_test_paths_for(Path::new(&format!("{}.rs", base_name)))
}

/// Get candidate test file paths for a base name (JavaScript/TypeScript).
///
/// Prefer using `patterns::candidate_test_paths_for()` for language-aware path generation.
pub fn candidate_js_test_paths(base_name: &str) -> Vec<String> {
    patterns::candidate_test_paths_for(Path::new(&format!("{}.ts", base_name)))
}

/// Extract the base name for correlation (e.g., "parser" from "src/parser.rs").
pub(super) fn correlation_base_name(path: &Path) -> Option<&str> {
    path.file_stem()?.to_str()
}

/// Check if a test file is considered "test-only" (no corresponding source change).
///
/// A test is test-only if its base name doesn't match any source file's base name,
/// even when accounting for common test suffixes/prefixes.
pub(super) fn is_test_only(test_base: &str, source_base_names: &HashSet<String>) -> bool {
    !source_base_names
        .iter()
        .any(|source_base| patterns::matches_base_name(test_base, source_base))
}

/// Extract base name from a test file, stripping test suffixes.
///
/// This is a convenience wrapper that returns an owned String.
pub(super) fn extract_base_name(path: &Path) -> Option<String> {
    file_base_name(path).map(|s| s.to_string())
}

/// Extract the normalized base name from a file path with test affixes stripped.
///
/// Examples:
/// - "tests/parser_tests.rs" -> "parser"
/// - "test_utils.rs" -> "utils"
/// - "src/parser.rs" -> "parser"
fn file_base_name(path: &Path) -> Option<&str> {
    let stem = path.file_stem()?.to_str()?;
    Some(strip_test_affixes(stem))
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
