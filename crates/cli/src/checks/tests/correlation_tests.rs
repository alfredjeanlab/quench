// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for source/test correlation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::Path;

fn make_change(path: &str, change_type: ChangeType) -> FileChange {
    FileChange {
        path: PathBuf::from(path),
        change_type,
        lines_added: 10,
        lines_deleted: 5,
    }
}

#[test]
fn correlation_base_name_extracts_stem() {
    assert_eq!(
        correlation_base_name(Path::new("src/parser.rs")),
        Some("parser")
    );
    assert_eq!(
        correlation_base_name(Path::new("src/foo/bar.rs")),
        Some("bar")
    );
}

#[test]
fn extract_base_name_strips_test_suffix() {
    assert_eq!(
        extract_base_name(Path::new("tests/parser_tests.rs")),
        Some("parser".to_string())
    );
    assert_eq!(
        extract_base_name(Path::new("tests/parser_test.rs")),
        Some("parser".to_string())
    );
    assert_eq!(
        extract_base_name(Path::new("tests/test_parser.rs")),
        Some("parser".to_string())
    );
    assert_eq!(
        extract_base_name(Path::new("tests/parser.rs")),
        Some("parser".to_string())
    );
}

#[test]
fn analyze_correlation_source_with_test() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
    assert!(
        result
            .with_tests
            .iter()
            .any(|p| p.to_string_lossy().contains("parser.rs"))
    );
}

#[test]
fn analyze_correlation_source_without_test() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/parser.rs", ChangeType::Modified)];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 0);
    assert_eq!(result.without_tests.len(), 1);
    assert!(
        result
            .without_tests
            .iter()
            .any(|p| p.to_string_lossy().contains("parser.rs"))
    );
}

#[test]
fn analyze_correlation_test_only_tdd() {
    let root = Path::new("/project");
    let changes = vec![make_change(
        "/project/tests/parser_tests.rs",
        ChangeType::Added,
    )];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 0);
    assert_eq!(result.without_tests.len(), 0);
    assert_eq!(result.test_only.len(), 1);
}

#[test]
fn analyze_correlation_excludes_mod_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/mod.rs", ChangeType::Modified)];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // mod.rs should be excluded - no violations
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_excludes_lib_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/lib.rs", ChangeType::Modified)];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_excludes_main_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/main.rs", ChangeType::Modified)];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_skips_deleted_files() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/parser.rs", ChangeType::Deleted)];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // Deleted files don't require tests
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_matches_test_in_test_dir() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/test/parser.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn build_glob_set_valid_patterns() {
    let patterns = vec!["**/*.rs".to_string(), "src/**/*".to_string()];
    let result = build_glob_set(&patterns);
    assert!(result.is_ok());
}

#[test]
fn build_glob_set_invalid_pattern() {
    let patterns = vec!["[invalid".to_string()];
    let result = build_glob_set(&patterns);
    assert!(result.is_err());
}

// =============================================================================
// INLINE TEST DETECTION TESTS
// =============================================================================

#[test]
fn changes_in_cfg_test_detects_test_additions() {
    let diff = r#"diff --git a/src/parser.rs b/src/parser.rs
index abc123..def456 100644
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1,3 +1,15 @@
 pub fn parse() -> bool {
     true
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn test_parse() {
+        assert!(parse());
+    }
+}
"#;

    assert!(changes_in_cfg_test(diff));
}

#[test]
fn changes_in_cfg_test_false_for_non_test_changes() {
    let diff = r#"diff --git a/src/parser.rs b/src/parser.rs
index abc123..def456 100644
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1,3 +1,4 @@
 pub fn parse() -> bool {
-    true
+    // Updated implementation
+    false
 }
"#;

    assert!(!changes_in_cfg_test(diff));
}

#[test]
fn changes_in_cfg_test_tracks_brace_depth() {
    let diff = r#"diff --git a/src/parser.rs b/src/parser.rs
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1,5 +1,12 @@
 pub fn parse() -> bool { true }

 #[cfg(test)]
 mod tests {
+    use super::*;
+
+    #[test]
+    fn nested() {
+        assert!(true);
+    }
 }
"#;

    assert!(changes_in_cfg_test(diff));
}

#[test]
fn changes_in_cfg_test_empty_diff() {
    assert!(!changes_in_cfg_test(""));
}

#[test]
fn changes_in_cfg_test_context_only() {
    // Context lines (prefixed with space) shouldn't count as changes
    let diff = r#"diff --git a/src/parser.rs b/src/parser.rs
--- a/src/parser.rs
+++ b/src/parser.rs
@@ -1,5 +1,5 @@
 pub fn parse() -> bool { true }

 #[cfg(test)]
 mod tests {
     fn test_parse() { }
 }
"#;

    assert!(!changes_in_cfg_test(diff));
}

// =============================================================================
// ENHANCED TEST LOCATION TESTS
// =============================================================================

#[test]
fn find_test_locations_for_source_file() {
    let source = Path::new("src/parser.rs");
    let locations = find_test_locations(source);

    // Should include tests/ directory variants
    assert!(locations.contains(&PathBuf::from("tests/parser.rs")));
    assert!(locations.contains(&PathBuf::from("tests/parser_test.rs")));
    assert!(locations.contains(&PathBuf::from("tests/parser_tests.rs")));
    assert!(locations.contains(&PathBuf::from("tests/test_parser.rs")));

    // Should include test/ directory variants (singular)
    assert!(locations.contains(&PathBuf::from("test/parser.rs")));
    assert!(locations.contains(&PathBuf::from("test/parser_test.rs")));
    assert!(locations.contains(&PathBuf::from("test/parser_tests.rs")));

    // Should include sibling test files
    assert!(locations.contains(&PathBuf::from("src/parser_test.rs")));
    assert!(locations.contains(&PathBuf::from("src/parser_tests.rs")));
}

#[test]
fn find_test_locations_for_nested_source_file() {
    let source = Path::new("src/foo/bar/lexer.rs");
    let locations = find_test_locations(source);

    // Should include tests/ directory variants
    assert!(locations.contains(&PathBuf::from("tests/lexer.rs")));
    assert!(locations.contains(&PathBuf::from("tests/lexer_tests.rs")));

    // Should include sibling test files in the same directory
    assert!(locations.contains(&PathBuf::from("src/foo/bar/lexer_test.rs")));
    assert!(locations.contains(&PathBuf::from("src/foo/bar/lexer_tests.rs")));
}

#[test]
fn has_correlated_test_with_location_match() {
    let source = Path::new("src/parser.rs");
    let test_changes = vec![PathBuf::from("tests/parser_tests.rs")];
    let test_base_names = vec!["parser".to_string()];

    assert!(has_correlated_test(source, &test_changes, &test_base_names));
}

#[test]
fn has_correlated_test_with_sibling_test() {
    let source = Path::new("src/parser.rs");
    let test_changes = vec![PathBuf::from("src/parser_tests.rs")];
    let test_base_names = vec!["parser".to_string()];

    assert!(has_correlated_test(source, &test_changes, &test_base_names));
}

#[test]
fn has_correlated_test_with_base_name_only() {
    let source = Path::new("src/parser.rs");
    let test_changes = vec![PathBuf::from("tests/something/parser_tests.rs")];
    let test_base_names = vec!["parser".to_string()];

    // Should match via base name even if location doesn't match exactly
    assert!(has_correlated_test(source, &test_changes, &test_base_names));
}

#[test]
fn has_correlated_test_no_match() {
    let source = Path::new("src/parser.rs");
    let test_changes = vec![PathBuf::from("tests/lexer_tests.rs")];
    let test_base_names = vec!["lexer".to_string()];

    assert!(!has_correlated_test(
        source,
        &test_changes,
        &test_base_names
    ));
}

#[test]
fn analyze_correlation_sibling_test_file() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/parser_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // Sibling test file should satisfy the requirement
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

// =============================================================================
// COMMIT ANALYSIS TESTS
// =============================================================================

use crate::checks::tests::diff::CommitChanges;

#[test]
fn analyze_commit_detects_source_without_tests() {
    let root = Path::new("/project");
    let commit = CommitChanges {
        hash: "abc123def456".to_string(),
        message: "feat: add parser".to_string(),
        changes: vec![make_change("/project/src/parser.rs", ChangeType::Added)],
    };

    let config = CorrelationConfig::default();
    let analysis = analyze_commit(&commit, &config, root);

    assert_eq!(analysis.hash, "abc123def456");
    assert_eq!(analysis.message, "feat: add parser");
    assert_eq!(analysis.source_without_tests.len(), 1);
    assert!(!analysis.is_test_only);
}

#[test]
fn analyze_commit_detects_test_only_tdd() {
    let root = Path::new("/project");
    let commit = CommitChanges {
        hash: "def456abc123".to_string(),
        message: "test: add parser tests".to_string(),
        changes: vec![make_change(
            "/project/tests/parser_tests.rs",
            ChangeType::Added,
        )],
    };

    let config = CorrelationConfig::default();
    let analysis = analyze_commit(&commit, &config, root);

    assert_eq!(analysis.source_without_tests.len(), 0);
    assert!(analysis.is_test_only);
}

#[test]
fn analyze_commit_source_with_tests_passes() {
    let root = Path::new("/project");
    let commit = CommitChanges {
        hash: "123abc456def".to_string(),
        message: "feat: add parser with tests".to_string(),
        changes: vec![
            make_change("/project/src/parser.rs", ChangeType::Added),
            make_change("/project/tests/parser_tests.rs", ChangeType::Added),
        ],
    };

    let config = CorrelationConfig::default();
    let analysis = analyze_commit(&commit, &config, root);

    assert_eq!(analysis.source_without_tests.len(), 0);
    assert!(!analysis.is_test_only);
}

// =============================================================================
// PERFORMANCE OPTIMIZATION TESTS (Phase 1-4)
// =============================================================================

#[test]
fn analyze_correlation_empty_changes_fast_path() {
    let root = Path::new("/project");
    let changes: Vec<FileChange> = vec![];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert!(result.with_tests.is_empty());
    assert!(result.without_tests.is_empty());
    assert!(result.test_only.is_empty());
}

#[test]
fn analyze_correlation_single_source_fast_path() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // Should use single source optimization
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_source_only_no_tests_fast_path() {
    let root = Path::new("/project");
    // Only source changes, no test changes
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/lexer.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert!(result.with_tests.is_empty());
    assert_eq!(result.without_tests.len(), 2);
    assert!(result.test_only.is_empty());
}

#[test]
fn analyze_correlation_test_only_fast_path() {
    let root = Path::new("/project");
    // Only test changes, no source changes
    let changes = vec![
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
        make_change("/project/tests/lexer_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert!(result.with_tests.is_empty());
    assert!(result.without_tests.is_empty());
    assert_eq!(result.test_only.len(), 2);
}

#[test]
fn test_index_has_test_for_direct_match() {
    let test_changes = vec![
        PathBuf::from("tests/parser_tests.rs"),
        PathBuf::from("tests/lexer_tests.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    // Should find test by base name
    assert!(index.has_test_for(Path::new("src/parser.rs")));
    assert!(index.has_test_for(Path::new("src/lexer.rs")));
    assert!(!index.has_test_for(Path::new("src/codegen.rs")));
}

#[test]
fn test_index_has_test_for_suffixed_names() {
    let test_changes = vec![
        PathBuf::from("tests/parser_test.rs"), // _test suffix
        PathBuf::from("tests/test_lexer.rs"),  // test_ prefix
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_test_for(Path::new("src/parser.rs")));
    assert!(index.has_test_for(Path::new("src/lexer.rs")));
}

#[test]
fn test_index_has_inline_test() {
    let test_changes = vec![
        PathBuf::from("src/parser.rs"), // Inline test in source file
        PathBuf::from("tests/lexer_tests.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_inline_test(Path::new("src/parser.rs")));
    assert!(!index.has_inline_test(Path::new("src/lexer.rs")));
}

#[test]
fn test_index_has_test_at_location() {
    let test_changes = vec![
        PathBuf::from("tests/parser_tests.rs"),
        PathBuf::from("src/lexer_tests.rs"), // Sibling test
    ];
    let index = TestIndex::new(&test_changes);

    // Should find test at expected location
    assert!(index.has_test_at_location(Path::new("src/parser.rs")));
    assert!(index.has_test_at_location(Path::new("src/lexer.rs")));
    assert!(!index.has_test_at_location(Path::new("src/codegen.rs")));
}

#[test]
fn analyze_correlation_many_sources_uses_index() {
    let root = Path::new("/project");

    // Create many source and test changes
    let mut changes: Vec<FileChange> = (0..20)
        .map(|i| {
            make_change(
                &format!("/project/src/module{}.rs", i),
                ChangeType::Modified,
            )
        })
        .collect();

    // Add matching tests for half of them
    for i in 0..10 {
        changes.push(make_change(
            &format!("/project/tests/module{}_tests.rs", i),
            ChangeType::Modified,
        ));
    }

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // First 10 modules should have tests, last 10 should not
    assert_eq!(result.with_tests.len(), 10);
    assert_eq!(result.without_tests.len(), 10);
}

// =============================================================================
// ENHANCED DEFAULT PATTERN TESTS
// =============================================================================

#[test]
fn default_patterns_include_jest_conventions() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.ts", ChangeType::Modified),
        make_change("/project/__tests__/parser.test.ts", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // __tests__ pattern should match
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn default_patterns_include_dot_test_suffix() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.ts", ChangeType::Modified),
        make_change("/project/src/parser.test.ts", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // .test.ts pattern should match
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn default_patterns_include_spec_directory() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rb", ChangeType::Modified),
        make_change("/project/spec/parser_spec.rb", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // spec/ directory pattern should match
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn default_patterns_include_test_prefix() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.py", ChangeType::Modified),
        make_change("/project/tests/test_parser.py", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // test_ prefix pattern should match
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

// =============================================================================
// TEST-ONLY FILTER CONSISTENCY TESTS
// =============================================================================

#[test]
fn test_only_filter_single_source_matches_multi_source() {
    // Verify same test files are identified as test-only
    // in both single-source and multi-source paths
    let root = Path::new("/project");

    // Single source case
    let single_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    // Multi source case (add another source)
    let multi_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/lexer.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let single_result = analyze_correlation(&single_changes, &config, root);
    let multi_result = analyze_correlation(&multi_changes, &config, root);

    // Both should identify other_tests.rs as test-only
    assert_eq!(
        single_result.test_only.len(),
        1,
        "Single source path should find 1 test-only"
    );
    assert_eq!(
        multi_result.test_only.len(),
        1,
        "Multi source path should find 1 test-only"
    );
}

#[test]
fn is_test_only_direct_match() {
    use std::collections::HashSet;
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());

    // Test base matches source directly
    assert!(!is_test_only("parser", &sources));
}

#[test]
fn is_test_only_with_suffix() {
    use std::collections::HashSet;
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());

    // Test base is source + _test/_tests suffix
    assert!(!is_test_only("parser_test", &sources));
    assert!(!is_test_only("parser_tests", &sources));
}

#[test]
fn is_test_only_with_prefix() {
    use std::collections::HashSet;
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());

    // Test base is test_ + source prefix
    assert!(!is_test_only("test_parser", &sources));
}

#[test]
fn is_test_only_no_match() {
    use std::collections::HashSet;
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());

    // Test base doesn't match any source
    assert!(is_test_only("lexer", &sources));
    assert!(is_test_only("lexer_tests", &sources));
    assert!(is_test_only("test_lexer", &sources));
}

// =============================================================================
// BIDIRECTIONAL MATCHING EDGE CASE TESTS
// =============================================================================

// Note: Files in src/ that match test patterns (like test_utils.rs matching
// **/test_*.*) are classified as tests, not sources. This is expected behavior.
// The tests below verify the TestIndex logic works correctly for edge cases.

#[test]
fn test_index_handles_test_like_source_name() {
    // If a source file had base name "test_utils", it should match a test
    // file with base name "test_utils" or "test_utils_test/tests"
    let test_changes = vec![PathBuf::from("tests/test_utils_tests.rs")];
    let index = TestIndex::new(&test_changes);

    // Source "test_utils" should find the test "test_utils" (from test_utils_tests.rs)
    assert!(
        index.has_test_for(Path::new("src/test_utils.rs")),
        "test_utils.rs should match test_utils_tests.rs"
    );
}

#[test]
fn test_index_handles_source_with_test_suffix() {
    // Source file with _test suffix should match test with additional _tests suffix
    let test_changes = vec![PathBuf::from("tests/parser_test_tests.rs")];
    let index = TestIndex::new(&test_changes);

    // Source "parser_test" should find the test "parser_test" (from parser_test_tests.rs)
    assert!(
        index.has_test_for(Path::new("src/parser_test.rs")),
        "parser_test.rs should match parser_test_tests.rs"
    );
}

#[test]
fn test_index_handles_confusing_names() {
    // Test various confusing naming patterns
    let test_changes = vec![
        PathBuf::from("tests/helper_tests.rs"), // test for "helper"
        PathBuf::from("tests/utils_test.rs"),   // test for "utils"
    ];
    let index = TestIndex::new(&test_changes);

    // These should match
    assert!(index.has_test_for(Path::new("src/helper.rs")));
    assert!(index.has_test_for(Path::new("src/utils.rs")));

    // These should NOT match (no corresponding test)
    assert!(!index.has_test_for(Path::new("src/parser.rs")));
    assert!(!index.has_test_for(Path::new("src/lexer.rs")));
}

#[test]
fn source_with_normal_name_correlates_correctly() {
    // Normal source files (no test-like patterns in name) work correctly
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn source_file_without_matching_test_detected() {
    // Source files without matching tests are properly detected
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/lexer_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // parser.rs has no test, lexer_tests.rs is test-only
    assert_eq!(result.with_tests.len(), 0);
    assert_eq!(result.without_tests.len(), 1);
    assert_eq!(result.test_only.len(), 1);
}
