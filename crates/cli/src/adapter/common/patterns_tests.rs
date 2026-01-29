#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

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
    assert_eq!(
        normalized,
        vec!["vendor/**", "build/**", "**/*.pyc", ".venv/**"]
    );
}

#[test]
fn handles_empty_input() {
    let patterns: Vec<String> = vec![];
    let normalized = normalize_exclude_patterns(&patterns);
    assert!(normalized.is_empty());
}
