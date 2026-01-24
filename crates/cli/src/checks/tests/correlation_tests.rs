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
// PLACEHOLDER TEST DETECTION TESTS
// =============================================================================

#[test]
fn find_placeholder_tests_detects_ignored_tests() {
    let content = r#"
#[test]
#[ignore = "TODO: implement parser"]
fn test_parser() { todo!() }

#[test]
fn test_other() { }
"#;

    let placeholders = find_placeholder_tests(content);
    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0], "test_parser");
}

#[test]
fn find_placeholder_tests_empty_content() {
    let placeholders = find_placeholder_tests("");
    assert!(placeholders.is_empty());
}

#[test]
fn find_placeholder_tests_no_ignored() {
    let content = r#"
#[test]
fn test_parser() { assert!(true); }
"#;

    let placeholders = find_placeholder_tests(content);
    assert!(placeholders.is_empty());
}

#[test]
fn find_placeholder_tests_multiple() {
    let content = r#"
#[test]
#[ignore = "TODO"]
fn test_one() { todo!() }

#[test]
#[ignore]
fn test_two() { todo!() }
"#;

    let placeholders = find_placeholder_tests(content);
    assert_eq!(placeholders.len(), 2);
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
