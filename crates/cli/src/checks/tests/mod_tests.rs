// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the tests check module.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn tests_check_name() {
    let check = TestsCheck;
    assert_eq!(check.name(), "tests");
}

#[test]
fn tests_check_description() {
    let check = TestsCheck;
    assert_eq!(check.description(), "Test correlation");
}

#[test]
fn tests_check_default_enabled() {
    let check = TestsCheck;
    assert!(check.default_enabled());
}

#[test]
fn build_correlation_config_uses_user_settings() {
    let config = TestsCommitConfig {
        check: "error".to_string(),
        scope: "branch".to_string(),
        placeholders: "allow".to_string(),
        test_patterns: vec!["custom/tests/**".to_string()],
        source_patterns: vec!["custom/src/**".to_string()],
        exclude: vec!["**/ignore_me.rs".to_string()],
    };

    let correlation = build_correlation_config(&config);

    assert_eq!(correlation.test_patterns, vec!["custom/tests/**"]);
    assert_eq!(correlation.source_patterns, vec!["custom/src/**"]);
    assert_eq!(correlation.exclude_patterns, vec!["**/ignore_me.rs"]);
}

#[test]
fn tests_commit_config_defaults() {
    let config = TestsCommitConfig::default();

    assert_eq!(config.check, "off");
    assert_eq!(config.scope, "branch");
    assert_eq!(config.placeholders, "allow");
    assert!(!config.test_patterns.is_empty());
    assert!(!config.source_patterns.is_empty());
    assert!(!config.exclude.is_empty());
}
