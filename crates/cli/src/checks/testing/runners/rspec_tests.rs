// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::time::Duration;

use super::*;
use yare::parameterized;

// =============================================================================
// JSON TEST DATA
// =============================================================================

const PASSING_JSON: &str = r#"{
    "version": "3.12.0",
    "examples": [
        {
            "id": "./spec/math_spec.rb[1:1:1]",
            "description": "adds two numbers",
            "full_description": "Math .add adds two numbers",
            "status": "passed",
            "run_time": 0.001234
        },
        {
            "id": "./spec/math_spec.rb[1:1:2]",
            "description": "multiplies numbers",
            "full_description": "Math .multiply multiplies numbers",
            "status": "passed",
            "run_time": 0.000567
        }
    ],
    "summary": {
        "duration": 0.05,
        "example_count": 2,
        "failure_count": 0,
        "pending_count": 0
    }
}"#;

const FAILING_JSON: &str = r#"{
    "examples": [
        {
            "full_description": "Math .add adds two numbers",
            "status": "passed",
            "run_time": 0.001
        },
        {
            "full_description": "Math .divide handles division by zero",
            "status": "failed",
            "run_time": 0.002
        }
    ],
    "summary": {
        "duration": 0.05,
        "example_count": 2,
        "failure_count": 1,
        "pending_count": 0
    }
}"#;

const PENDING_JSON: &str = r#"{
    "examples": [
        {
            "full_description": "Math .add adds numbers",
            "status": "passed",
            "run_time": 0.001
        },
        {
            "full_description": "Math .sqrt handles negative numbers",
            "status": "pending",
            "run_time": 0.0
        }
    ],
    "summary": {
        "duration": 0.03,
        "example_count": 2,
        "failure_count": 0,
        "pending_count": 1
    }
}"#;

// =============================================================================
// PARAMETERIZED PARSING TESTS
// =============================================================================

#[parameterized(
    passing = { PASSING_JSON, true, 2, 0 },
    failing = { FAILING_JSON, false, 2, 1 },
    pending = { PENDING_JSON, true, 2, 0 },  // pending doesn't fail overall
)]
fn parses_test_results(json: &str, expect_passed: bool, test_count: usize, fail_count: usize) {
    let result = parse_rspec_json(json, Duration::from_secs(1));
    assert_eq!(result.passed, expect_passed, "passed mismatch");
    assert_eq!(result.tests.len(), test_count, "test count mismatch");
    let actual_fails = result.tests.iter().filter(|t| !t.passed).count();
    assert_eq!(actual_fails, fail_count, "failure count mismatch");
}

#[test]
fn parses_passing_test_details() {
    let result = parse_rspec_json(PASSING_JSON, Duration::from_secs(1));

    assert!(result.tests[0].passed);
    assert_eq!(result.tests[0].name, "Math .add adds two numbers");
    assert_eq!(result.tests[0].duration, Duration::from_secs_f64(0.001234));
    assert!(result.tests[1].passed);
    assert_eq!(result.tests[1].name, "Math .multiply multiplies numbers");
}

#[test]
fn parses_failing_test_details() {
    let result = parse_rspec_json(FAILING_JSON, Duration::from_secs(1));

    assert!(result.tests[0].passed);
    assert!(!result.tests[1].passed);
    assert_eq!(result.tests[1].name, "Math .divide handles division by zero");
}

#[test]
fn parses_pending_test_details() {
    let result = parse_rspec_json(PENDING_JSON, Duration::from_secs(1));

    assert!(result.tests[0].passed);
    assert!(!result.tests[0].skipped);
    assert!(result.tests[1].passed); // skipped tests count as passed
    assert!(result.tests[1].skipped);
    assert_eq!(result.tests[1].name, "Math .sqrt handles negative numbers");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[parameterized(
    empty_string = { "" },
    empty_examples = { r#"{"examples": [], "summary": {"duration": 0, "example_count": 0, "failure_count": 0, "pending_count": 0}}"# },
)]
fn handles_empty_input(json: &str) {
    let result = parse_rspec_json(json, Duration::from_secs(1));
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn handles_missing_summary() {
    let output = r#"{
        "examples": [
            {
                "full_description": "test passes",
                "status": "passed",
                "run_time": 0.001
            }
        ]
    }"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));
    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
}

#[test]
fn handles_missing_run_time() {
    let output = r#"{
        "examples": [
            {
                "full_description": "test passes",
                "status": "passed"
            }
        ],
        "summary": {"duration": 0.01, "example_count": 1, "failure_count": 0, "pending_count": 0}
    }"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));

    assert_eq!(result.tests.len(), 1);
    assert_eq!(result.tests[0].duration, Duration::ZERO);
}

#[test]
fn finds_json_in_output_with_prefix() {
    let output = r#"
Running RSpec...
Loading spec helper
{"examples": [{"full_description": "test", "status": "passed", "run_time": 0.01}], "summary": {"duration": 0.1, "example_count": 1, "failure_count": 0, "pending_count": 0}}
"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
}

#[test]
fn detects_failure_without_json() {
    let output = "Failures:\n\n  1) Math fails\n     expected: true\n     got: false";
    let result = parse_rspec_json(output, Duration::from_secs(1));
    assert!(!result.passed);
}
