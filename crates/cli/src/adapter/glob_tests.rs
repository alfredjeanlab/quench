//! Tests for glob utilities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
