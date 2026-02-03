// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Output format specs: JSON and text output for escapes check.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > metrics: { source: {...}, test: {...} }
#[test]
fn escapes_json_includes_source_test_breakdown_per_pattern() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Source metrics by pattern name
    let source = metrics.get("source").unwrap();
    assert!(
        source.get("unwrap").is_some() || source.get("todo").is_some(),
        "source should have pattern counts"
    );

    // Test metrics by pattern name
    let test = metrics.get("test").unwrap();
    assert!(test.is_object(), "test should have pattern counts");
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > Violation types: missing_comment, forbidden, threshold_exceeded,
/// > suppress_forbidden, suppress_missing_comment, shellcheck_forbidden, shellcheck_missing_comment
#[test]
fn escapes_violation_type_is_one_of_expected_values() {
    let escapes = check("escapes").on("violations").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_comment",
        "forbidden",
        "threshold_exceeded",
        "suppress_forbidden",
        "suppress_missing_comment",
        "shellcheck_forbidden",
        "shellcheck_missing_comment",
    ];
    for violation in violations {
        let vtype = violation.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Text output shows violations with file path, line, and advice
#[test]
fn escapes_text_output_format_on_missing_comment() {
    check("escapes")
        .on("escapes/comment-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("lib.rs")
        .stdout_has("missing_comment");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Forbidden violations show pattern name and advice
#[test]
fn escapes_text_output_format_on_forbidden() {
    check("escapes")
        .on("escapes/forbid-source")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Threshold exceeded shows count vs limit
#[test]
fn escapes_text_output_format_on_threshold_exceeded() {
    check("escapes")
        .on("escapes/count-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("threshold_exceeded");
}

// =============================================================================
// JSON OUTPUT STRUCTURE SPECS
// =============================================================================

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON output includes all required fields for violations
#[test]
fn escapes_json_violation_structure_complete() {
    let escapes = check("escapes").on("escapes/forbid-source").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(!violations.is_empty(), "should have violations");

    // Each violation must have all required fields
    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("line").is_some(), "missing line");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("pattern").is_some(), "missing pattern");
        assert!(violation.get("advice").is_some(), "missing advice");
    }
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON metrics include source and test breakdowns per pattern
#[test]
fn escapes_json_metrics_structure_complete() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Verify structure
    assert!(metrics.get("source").is_some(), "missing source metrics");
    assert!(metrics.get("test").is_some(), "missing test metrics");

    // Source and test should be objects with pattern counts
    let source = metrics.get("source").unwrap();
    let test = metrics.get("test").unwrap();
    assert!(source.is_object(), "source should be object");
    assert!(test.is_object(), "test should be object");
}
