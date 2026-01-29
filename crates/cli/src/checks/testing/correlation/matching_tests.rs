// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for source-to-test file matching.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::*;

// =============================================================================
// BASE NAME TESTS
// =============================================================================

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

// =============================================================================
// TEST LOCATION TESTS
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

// =============================================================================
// TEST INDEX TESTS
// =============================================================================

#[test]
fn test_index_has_test_for_direct_match() {
    let test_changes = vec![
        PathBuf::from("tests/parser_tests.rs"),
        PathBuf::from("tests/lexer_tests.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_test_for(Path::new("src/parser.rs")));
    assert!(index.has_test_for(Path::new("src/lexer.rs")));
    assert!(!index.has_test_for(Path::new("src/codegen.rs")));
}

#[test]
fn test_index_has_test_for_suffixed_names() {
    let test_changes = vec![
        PathBuf::from("tests/parser_test.rs"),
        PathBuf::from("tests/test_lexer.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_test_for(Path::new("src/parser.rs")));
    assert!(index.has_test_for(Path::new("src/lexer.rs")));
}

#[test]
fn test_index_has_inline_test() {
    let test_changes = vec![
        PathBuf::from("src/parser.rs"),
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
        PathBuf::from("src/lexer_tests.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_test_at_location(Path::new("src/parser.rs")));
    assert!(index.has_test_at_location(Path::new("src/lexer.rs")));
    assert!(!index.has_test_at_location(Path::new("src/codegen.rs")));
}

#[test]
fn test_index_handles_test_like_source_name() {
    let test_changes = vec![PathBuf::from("tests/test_utils_tests.rs")];
    let index = TestIndex::new(&test_changes);

    assert!(
        index.has_test_for(Path::new("src/test_utils.rs")),
        "test_utils.rs should match test_utils_tests.rs"
    );
}

#[test]
fn test_index_handles_source_with_test_suffix() {
    let test_changes = vec![PathBuf::from("tests/parser_test_tests.rs")];
    let index = TestIndex::new(&test_changes);

    assert!(
        index.has_test_for(Path::new("src/parser_test.rs")),
        "parser_test.rs should match parser_test_tests.rs"
    );
}

#[test]
fn test_index_handles_confusing_names() {
    let test_changes = vec![
        PathBuf::from("tests/helper_tests.rs"),
        PathBuf::from("tests/utils_test.rs"),
    ];
    let index = TestIndex::new(&test_changes);

    assert!(index.has_test_for(Path::new("src/helper.rs")));
    assert!(index.has_test_for(Path::new("src/utils.rs")));

    assert!(!index.has_test_for(Path::new("src/parser.rs")));
    assert!(!index.has_test_for(Path::new("src/lexer.rs")));
}

// =============================================================================
// TEST-ONLY FILTER TESTS
// =============================================================================

#[test]
fn is_test_only_direct_match() {
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());
    assert!(!is_test_only("parser", &sources));
}

#[test]
fn is_test_only_with_suffix() {
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());
    assert!(!is_test_only("parser_test", &sources));
    assert!(!is_test_only("parser_tests", &sources));
}

#[test]
fn is_test_only_with_prefix() {
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());
    assert!(!is_test_only("test_parser", &sources));
}

#[test]
fn is_test_only_no_match() {
    let mut sources = HashSet::new();
    sources.insert("parser".to_string());
    assert!(is_test_only("lexer", &sources));
    assert!(is_test_only("lexer_tests", &sources));
    assert!(is_test_only("test_lexer", &sources));
}
