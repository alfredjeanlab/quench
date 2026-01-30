// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the tests check module.

use std::path::Path;

use crate::config::{TestsCommitConfig, TestsConfig};

use super::correlation::missing_tests_advice;
use super::patterns::{Language, detect_language};
use super::suite::{SuiteResult, SuiteResults};
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
fn tests_config_auto_defaults_to_false() {
    let config = TestsConfig::default();
    assert!(!config.auto);
}

#[test]
fn tests_commit_config_defaults() {
    let config = TestsCommitConfig::default();

    assert_eq!(config.check, "off");
    assert_eq!(config.scope, "branch");
    assert_eq!(config.placeholders, "allow");
    assert!(config.exclude.is_empty());
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
    assert!(advice.contains("tests/parser.rs"));
    assert!(advice.contains("parser_tests.rs"));
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

// =============================================================================
// METRIC AGGREGATION TESTS
// =============================================================================

#[test]
fn aggregated_metrics_sums_test_counts() {
    let suites = SuiteResults {
        passed: true,
        suites: vec![
            SuiteResult {
                name: "cargo".to_string(),
                runner: "cargo".to_string(),
                passed: true,
                test_count: 47,
                total_ms: 2341,
                avg_ms: Some(50),
                max_ms: Some(245),
                max_test: Some("test_a".to_string()),
                ..Default::default()
            },
            SuiteResult {
                name: "bats".to_string(),
                runner: "bats".to_string(),
                passed: true,
                test_count: 12,
                total_ms: 1200,
                avg_ms: Some(100),
                max_ms: Some(350),
                max_test: Some("test_b".to_string()),
                ..Default::default()
            },
        ],
    };

    let agg = suites.aggregated_metrics();

    assert_eq!(agg.test_count, 59);
    assert_eq!(agg.total_ms, 3541);
}

#[test]
fn aggregated_metrics_calculates_weighted_average() {
    let suites = SuiteResults {
        passed: true,
        suites: vec![
            SuiteResult {
                name: "cargo".to_string(),
                runner: "cargo".to_string(),
                passed: true,
                test_count: 47,
                avg_ms: Some(50),
                ..Default::default()
            },
            SuiteResult {
                name: "bats".to_string(),
                runner: "bats".to_string(),
                passed: true,
                test_count: 12,
                avg_ms: Some(100),
                ..Default::default()
            },
        ],
    };

    let agg = suites.aggregated_metrics();

    // Weighted avg: (47*50 + 12*100) / 59 = (2350 + 1200) / 59 = 60
    assert_eq!(agg.avg_ms, Some(60));
}

#[test]
fn aggregated_metrics_finds_max_test_across_suites() {
    let suites = SuiteResults {
        passed: true,
        suites: vec![
            SuiteResult {
                name: "cargo".to_string(),
                runner: "cargo".to_string(),
                passed: true,
                max_ms: Some(245),
                max_test: Some("cargo_slow".to_string()),
                ..Default::default()
            },
            SuiteResult {
                name: "bats".to_string(),
                runner: "bats".to_string(),
                passed: true,
                max_ms: Some(350),
                max_test: Some("bats_slow".to_string()),
                ..Default::default()
            },
        ],
    };

    let agg = suites.aggregated_metrics();

    assert_eq!(agg.max_ms, Some(350));
    assert_eq!(agg.max_test, Some("bats_slow".to_string()));
}

#[test]
fn aggregated_metrics_handles_empty_suites() {
    let suites = SuiteResults {
        passed: true,
        suites: vec![],
    };

    let agg = suites.aggregated_metrics();

    assert_eq!(agg.test_count, 0);
    assert_eq!(agg.total_ms, 0);
    assert_eq!(agg.avg_ms, None);
    assert_eq!(agg.max_ms, None);
    assert_eq!(agg.max_test, None);
}

#[test]
fn aggregated_metrics_handles_missing_avg_ms() {
    let suites = SuiteResults {
        passed: true,
        suites: vec![
            SuiteResult {
                name: "cargo".to_string(),
                runner: "cargo".to_string(),
                passed: true,
                test_count: 10,
                avg_ms: Some(50),
                ..Default::default()
            },
            SuiteResult {
                name: "skipped".to_string(),
                runner: "bats".to_string(),
                skipped: true,
                test_count: 0,
                avg_ms: None,
                ..Default::default()
            },
        ],
    };

    let agg = suites.aggregated_metrics();

    // Only counts suites with avg_ms
    assert_eq!(agg.avg_ms, Some(50));
}
