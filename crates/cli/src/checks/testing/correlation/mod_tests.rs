// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for correlation analysis orchestration.

use std::path::{Path, PathBuf};

use crate::checks::testing::diff::{ChangeType, CommitChanges, FileChange};

use super::*;

fn make_change(path: &str, change_type: ChangeType) -> FileChange {
    FileChange {
        path: PathBuf::from(path),
        change_type,
        lines_added: 10,
        lines_deleted: 5,
    }
}

/// A Rust-like correlation config for unit tests (replaces former Default).
fn rust_correlation_config() -> CorrelationConfig {
    CorrelationConfig {
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
            "**/generated/**".to_string(),
            "**/mod.rs".to_string(),
            "**/lib.rs".to_string(),
            "**/main.rs".to_string(),
        ],
    }
}

// =============================================================================
// CORRELATION ANALYSIS TESTS
// =============================================================================

#[test]
fn analyze_correlation_source_with_test() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 0);
    assert_eq!(result.without_tests.len(), 0);
    assert_eq!(result.test_only.len(), 1);
}

#[test]
fn analyze_correlation_excludes_mod_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/mod.rs", ChangeType::Modified)];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    // mod.rs should be excluded - no violations
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_excludes_lib_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/lib.rs", ChangeType::Modified)];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_excludes_main_rs() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/main.rs", ChangeType::Modified)];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_skips_deleted_files() {
    let root = Path::new("/project");
    let changes = vec![make_change("/project/src/parser.rs", ChangeType::Deleted)];

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_sibling_test_file() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/parser_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    // Sibling test file should satisfy the requirement
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

// =============================================================================
// COMMIT ANALYSIS TESTS
// =============================================================================

#[test]
fn analyze_commit_detects_source_without_tests() {
    let root = Path::new("/project");
    let commit = CommitChanges {
        hash: "abc123def456".to_string(),
        message: "feat: add parser".to_string(),
        changes: vec![make_change("/project/src/parser.rs", ChangeType::Added)],
    };

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
    let analysis = analyze_commit(&commit, &config, root);

    assert_eq!(analysis.source_without_tests.len(), 0);
    assert!(!analysis.is_test_only);
}

// =============================================================================
// PERFORMANCE OPTIMIZATION TESTS
// =============================================================================

#[test]
fn analyze_correlation_empty_changes_fast_path() {
    let root = Path::new("/project");
    let changes: Vec<FileChange> = vec![];

    let config = rust_correlation_config();
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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    // Should use single source optimization
    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn analyze_correlation_source_only_no_tests_fast_path() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/lexer.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert!(result.with_tests.is_empty());
    assert_eq!(result.without_tests.len(), 2);
    assert!(result.test_only.is_empty());
}

#[test]
fn analyze_correlation_test_only_fast_path() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
        make_change("/project/tests/lexer_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert!(result.with_tests.is_empty());
    assert!(result.without_tests.is_empty());
    assert_eq!(result.test_only.len(), 2);
}

#[test]
fn analyze_correlation_many_sources_uses_index() {
    let root = Path::new("/project");

    let mut changes: Vec<FileChange> = (0..20)
        .map(|i| {
            make_change(
                &format!("/project/src/module{}.rs", i),
                ChangeType::Modified,
            )
        })
        .collect();

    for i in 0..10 {
        changes.push(make_change(
            &format!("/project/tests/module{}_tests.rs", i),
            ChangeType::Modified,
        ));
    }

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 10);
    assert_eq!(result.without_tests.len(), 10);
}

// =============================================================================
// DEFAULT PATTERN TESTS
// =============================================================================

#[test]
fn default_patterns_include_jest_conventions() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.ts", ChangeType::Modified),
        make_change("/project/__tests__/parser.test.ts", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

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

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

// =============================================================================
// TEST-ONLY FILTER TESTS
// =============================================================================

#[test]
fn test_only_filter_single_source_matches_multi_source() {
    let root = Path::new("/project");

    let single_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    let multi_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/lexer.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let single_result = analyze_correlation(&single_changes, &config, root);
    let multi_result = analyze_correlation(&multi_changes, &config, root);

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

// =============================================================================
// BIDIRECTIONAL MATCHING EDGE CASES
// =============================================================================

#[test]
fn source_with_normal_name_correlates_correctly() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/parser_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn source_file_without_matching_test_detected() {
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/lexer_tests.rs", ChangeType::Modified),
    ];

    let config = rust_correlation_config();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 0);
    assert_eq!(result.without_tests.len(), 1);
    assert_eq!(result.test_only.len(), 1);
}
