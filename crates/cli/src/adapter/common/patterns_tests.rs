#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::adapter::glob::build_glob_set;

#[test]
fn normalizes_trailing_slash() {
    let patterns = vec!["vendor/".to_string()];
    let normalized = normalize_exclude_patterns(&patterns);
    assert_eq!(normalized, vec!["vendor/**"]);
}

#[test]
fn normalizes_bare_directory() {
    let patterns = vec!["build".to_string()];
    let normalized = normalize_exclude_patterns(&patterns);
    assert_eq!(normalized, vec!["build/**"]);
}

#[test]
fn preserves_existing_globs() {
    let patterns = vec!["**/*.pyc".to_string(), "dist/**".to_string()];
    let normalized = normalize_exclude_patterns(&patterns);
    assert_eq!(normalized, vec!["**/*.pyc", "dist/**"]);
}

#[test]
fn normalizes_mixed_patterns() {
    let patterns = vec![
        "vendor/".to_string(),
        "build".to_string(),
        "**/*.pyc".to_string(),
        ".venv".to_string(),
    ];
    let normalized = normalize_exclude_patterns(&patterns);
    assert_eq!(normalized, vec!["vendor/**", "build/**", "**/*.pyc", ".venv/**"]);
}

#[test]
fn handles_empty_input() {
    let patterns: Vec<String> = vec![];
    let normalized = normalize_exclude_patterns(&patterns);
    assert!(normalized.is_empty());
}

// =============================================================================
// check_exclude_patterns tests
// =============================================================================

#[test]
fn check_exclude_matches_globset() {
    let patterns = build_glob_set(&["vendor/**".to_string()]);
    assert!(check_exclude_patterns(Path::new("vendor/lib.go"), &patterns, None));
}

#[test]
fn check_exclude_no_match_returns_false() {
    let patterns = build_glob_set(&["vendor/**".to_string()]);
    assert!(!check_exclude_patterns(Path::new("src/main.go"), &patterns, None));
}

#[test]
fn check_exclude_fast_prefix_matches() {
    let patterns = build_glob_set(&[]);
    assert!(check_exclude_patterns(
        Path::new("node_modules/pkg/index.js"),
        &patterns,
        Some(&["node_modules", "dist"]),
    ));
}

#[test]
fn check_exclude_fast_prefix_no_match() {
    let patterns = build_glob_set(&[]);
    assert!(!check_exclude_patterns(
        Path::new("src/index.js"),
        &patterns,
        Some(&["node_modules", "dist"]),
    ));
}

#[test]
fn check_exclude_fast_prefix_and_globset_combined() {
    let patterns = build_glob_set(&["build/**".to_string()]);
    // Fast prefix match
    assert!(check_exclude_patterns(Path::new("dist/bundle.js"), &patterns, Some(&["dist"]),));
    // GlobSet match
    assert!(check_exclude_patterns(Path::new("build/output.js"), &patterns, Some(&["dist"]),));
    // No match
    assert!(!check_exclude_patterns(Path::new("src/lib.js"), &patterns, Some(&["dist"]),));
}

#[test]
fn check_exclude_empty_patterns_no_prefixes() {
    let patterns = build_glob_set(&[]);
    assert!(!check_exclude_patterns(Path::new("anything/file.rs"), &patterns, None));
}
