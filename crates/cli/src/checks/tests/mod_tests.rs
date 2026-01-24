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

// =============================================================================
// LANGUAGE DETECTION TESTS
// =============================================================================

#[test]
fn detect_language_rust() {
    assert_eq!(detect_language(Path::new("src/parser.rs")), Language::Rust);
    assert_eq!(detect_language(Path::new("lib.rs")), Language::Rust);
}

#[test]
fn detect_language_go() {
    assert_eq!(detect_language(Path::new("main.go")), Language::Go);
    assert_eq!(
        detect_language(Path::new("pkg/parser/parser.go")),
        Language::Go
    );
}

#[test]
fn detect_language_javascript() {
    assert_eq!(
        detect_language(Path::new("src/parser.ts")),
        Language::JavaScript
    );
    assert_eq!(
        detect_language(Path::new("src/parser.tsx")),
        Language::JavaScript
    );
    assert_eq!(
        detect_language(Path::new("src/parser.js")),
        Language::JavaScript
    );
    assert_eq!(
        detect_language(Path::new("src/parser.jsx")),
        Language::JavaScript
    );
    assert_eq!(
        detect_language(Path::new("src/parser.mjs")),
        Language::JavaScript
    );
    assert_eq!(
        detect_language(Path::new("src/parser.mts")),
        Language::JavaScript
    );
}

#[test]
fn detect_language_python() {
    assert_eq!(detect_language(Path::new("main.py")), Language::Python);
    assert_eq!(
        detect_language(Path::new("src/parser.py")),
        Language::Python
    );
}

#[test]
fn detect_language_unknown() {
    assert_eq!(detect_language(Path::new("file.cpp")), Language::Unknown);
    assert_eq!(detect_language(Path::new("file.java")), Language::Unknown);
    assert_eq!(
        detect_language(Path::new("no_extension")),
        Language::Unknown
    );
}

// =============================================================================
// ADVICE MESSAGE TESTS
// =============================================================================

#[test]
fn advice_message_rust() {
    let advice = missing_tests_advice("parser", Language::Rust);
    assert!(advice.contains("tests/parser_tests.rs"));
    assert!(advice.contains("#[cfg(test)]"));
}

#[test]
fn advice_message_go() {
    let advice = missing_tests_advice("parser", Language::Go);
    assert!(advice.contains("parser_test.go"));
}

#[test]
fn advice_message_javascript() {
    let advice = missing_tests_advice("parser", Language::JavaScript);
    assert!(advice.contains("parser.test.ts"));
    assert!(advice.contains("__tests__/parser.test.ts"));
}

#[test]
fn advice_message_python() {
    let advice = missing_tests_advice("parser", Language::Python);
    assert!(advice.contains("test_parser.py"));
}

#[test]
fn advice_message_unknown() {
    let advice = missing_tests_advice("parser", Language::Unknown);
    assert!(advice.contains("parser"));
}
