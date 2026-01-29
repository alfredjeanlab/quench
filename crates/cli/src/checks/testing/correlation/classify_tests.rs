// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for file change classification.

use super::build_glob_set;

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
