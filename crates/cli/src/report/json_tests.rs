// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for JSON report formatter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::baseline::EscapesMetrics;
use crate::report::test_support::{
    AllChecks, ExcludeChecks, assert_buffered_matches_streamed, create_test_baseline,
};

// =============================================================================
// BASIC FORMATTING
// =============================================================================

#[test]
fn json_format_empty_baseline() {
    let formatter = JsonFormatter::default();
    let empty = formatter.format_empty();
    let json: serde_json::Value = serde_json::from_str(&empty).unwrap();
    assert!(json.get("metrics").is_some());
}

#[test]
fn json_format_includes_metadata() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json.get("updated").is_some());
    assert_eq!(json["commit"], "abc1234");
}

#[test]
fn json_format_includes_coverage() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(json["metrics"]["coverage"]["total"], 85.5);
}

#[test]
fn json_format_includes_escapes() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let escapes = &json["metrics"]["escapes"]["source"];
    assert!(escapes.is_object());
}

#[test]
fn json_format_includes_build_time() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(json["metrics"]["build_time"]["cold"], 45.0);
    assert_eq!(json["metrics"]["build_time"]["hot"], 12.5);
}

#[test]
fn json_format_includes_binary_size() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["metrics"]["binary_size"].is_object());
}

#[test]
fn json_format_includes_test_time() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(json["metrics"]["test_time"]["total"], 30.5);
    assert_eq!(json["metrics"]["test_time"]["avg"], 0.5);
    assert_eq!(json["metrics"]["test_time"]["max"], 2.0);
}

// =============================================================================
// COMPACT MODE
// =============================================================================

#[test]
fn compact_mode_produces_single_line() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::new(true); // compact = true
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    // Compact JSON should not have embedded newlines (except possibly trailing)
    let trimmed = output.trim();
    assert!(
        !trimmed.contains('\n'),
        "Compact output should be single line"
    );
}

#[test]
fn non_compact_mode_is_pretty_printed() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::new(false); // compact = false
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    // Pretty-printed JSON should have newlines
    assert!(
        output.contains('\n'),
        "Non-compact output should be multi-line"
    );
}

#[test]
fn default_mode_is_pretty_printed() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    assert!(
        output.contains('\n'),
        "Default output should be pretty-printed"
    );
}

// =============================================================================
// STREAMING CONSISTENCY
// =============================================================================

#[test]
fn buffered_matches_streamed_output() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    assert_buffered_matches_streamed(&formatter, &baseline, &AllChecks);
}

#[test]
fn empty_buffered_matches_streamed() {
    let formatter = JsonFormatter::default();

    let buffered = formatter.format_empty();

    let mut streamed = Vec::new();
    formatter.format_empty_to(&mut streamed).unwrap();
    let streamed_str = String::from_utf8(streamed).unwrap();

    assert_eq!(buffered, streamed_str);
}

#[test]
fn compact_buffered_matches_streamed() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::new(true);
    assert_buffered_matches_streamed(&formatter, &baseline, &AllChecks);
}

// =============================================================================
// VALID JSON OUTPUT
// =============================================================================

#[test]
fn output_is_valid_json() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let result: Result<serde_json::Value, _> = serde_json::from_str(&output);
    assert!(result.is_ok(), "Output should be valid JSON");
}

#[test]
fn empty_output_is_valid_json() {
    let formatter = JsonFormatter::default();
    let output = formatter.format_empty();

    let result: Result<serde_json::Value, _> = serde_json::from_str(&output);
    assert!(result.is_ok(), "Empty output should be valid JSON");
}

#[test]
fn compact_output_is_valid_json() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::new(true);
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let result: Result<serde_json::Value, _> = serde_json::from_str(&output);
    assert!(result.is_ok(), "Compact output should be valid JSON");
}

// =============================================================================
// FILTERING
// =============================================================================

#[test]
fn excludes_coverage_when_tests_filtered() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    // Coverage is associated with the "tests" check
    let filter = ExcludeChecks(vec!["tests"]);
    let output = formatter.format(&baseline, &filter).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["metrics"].get("coverage").is_none());
}

#[test]
fn excludes_escapes_when_filtered() {
    let baseline = create_test_baseline();
    let formatter = JsonFormatter::default();
    let filter = ExcludeChecks(vec!["escapes"]);
    let output = formatter.format(&baseline, &filter).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["metrics"].get("escapes").is_none());
}

// =============================================================================
// ESCAPES DETAIL
// =============================================================================

#[test]
fn json_format_escapes_includes_all_patterns() {
    let mut baseline = create_test_baseline();
    baseline.metrics.escapes = Some(EscapesMetrics {
        source: [
            ("unwrap".to_string(), 10),
            ("expect".to_string(), 5),
            ("panic".to_string(), 2),
        ]
        .into_iter()
        .collect(),
        test: None,
    });

    let formatter = JsonFormatter::default();
    let output = formatter.format(&baseline, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let escapes = &json["metrics"]["escapes"]["source"];
    assert_eq!(escapes["unwrap"], 10);
    assert_eq!(escapes["expect"], 5);
    assert_eq!(escapes["panic"], 2);
}
