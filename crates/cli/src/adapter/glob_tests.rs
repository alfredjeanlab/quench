// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests for glob utilities.

use super::*;

#[test]
fn builds_empty_set_from_empty_patterns() {
    let set = build_glob_set(&[]);
    assert!(!set.is_match("anything.rs"));
}

#[test]
fn matches_simple_pattern() {
    let set = build_glob_set(&["*.rs".to_string()]);
    assert!(set.is_match("foo.rs"));
    assert!(!set.is_match("foo.txt"));
}

#[test]
fn matches_glob_star_pattern() {
    let set = build_glob_set(&["**/*.rs".to_string()]);
    assert!(set.is_match("src/foo.rs"));
    assert!(set.is_match("src/deep/nested/bar.rs"));
    assert!(!set.is_match("foo.txt"));
}

#[test]
fn skips_invalid_pattern() {
    // Invalid pattern (unclosed bracket) should be skipped
    let set = build_glob_set(&["[invalid".to_string(), "*.rs".to_string()]);
    // Valid pattern should still work
    assert!(set.is_match("foo.rs"));
}

#[test]
fn matches_multiple_patterns() {
    let set = build_glob_set(&["*.rs".to_string(), "*.toml".to_string()]);
    assert!(set.is_match("lib.rs"));
    assert!(set.is_match("Cargo.toml"));
    assert!(!set.is_match("README.md"));
}

#[test]
fn glob_star_matches_root_level() {
    // Verify that **/*.rs matches root-level files (zero path components)
    let set = build_glob_set(&["**/*.rs".to_string()]);
    assert!(set.is_match("foo.rs"), "**/*.rs should match foo.rs");
    assert!(set.is_match("src/foo.rs"));

    // Verify **/*_test.sh matches root-level files
    let set2 = build_glob_set(&["**/*_test.sh".to_string()]);
    assert!(set2.is_match("foo_test.sh"), "**/*_test.sh should match foo_test.sh");
    assert!(set2.is_match("tests/foo_test.sh"));
}
