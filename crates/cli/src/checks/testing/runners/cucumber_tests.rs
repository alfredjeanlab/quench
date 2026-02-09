// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::time::Duration;

use super::*;
use yare::parameterized;

// =============================================================================
// JSON TEST DATA
// =============================================================================

const PASSING_JSON: &str = r#"[
    {
        "uri": "features/math.feature",
        "name": "Math operations",
        "elements": [
            {
                "type": "scenario",
                "name": "Adding numbers",
                "steps": [
                    {
                        "name": "I have entered 50 into the calculator",
                        "result": {
                            "status": "passed",
                            "duration": 1000000
                        }
                    },
                    {
                        "name": "I press add",
                        "result": {
                            "status": "passed",
                            "duration": 500000
                        }
                    }
                ]
            }
        ]
    }
]"#;

const FAILING_JSON: &str = r#"[
    {
        "uri": "features/math.feature",
        "name": "Math operations",
        "elements": [
            {
                "type": "scenario",
                "name": "Division by zero",
                "steps": [
                    {
                        "name": "I divide by zero",
                        "result": {
                            "status": "failed",
                            "duration": 2000000
                        }
                    }
                ]
            }
        ]
    }
]"#;

const PENDING_JSON: &str = r#"[
    {
        "uri": "features/future.feature",
        "name": "Future features",
        "elements": [
            {
                "type": "scenario",
                "name": "Unimplemented feature",
                "steps": [
                    {
                        "name": "a pending step",
                        "result": {
                            "status": "pending",
                            "duration": 0
                        }
                    }
                ]
            }
        ]
    }
]"#;

const UNDEFINED_JSON: &str = r#"[
    {
        "uri": "features/new.feature",
        "name": "New feature",
        "elements": [
            {
                "type": "scenario",
                "name": "Missing steps",
                "steps": [
                    {
                        "name": "an undefined step",
                        "result": {
                            "status": "undefined",
                            "duration": 0
                        }
                    }
                ]
            }
        ]
    }
]"#;

// =============================================================================
// PARAMETERIZED SCENARIO PARSING TESTS
// =============================================================================

#[parameterized(
    passing = { PASSING_JSON, true, 1, false, "Adding numbers" },
    failing = { FAILING_JSON, false, 1, false, "Division by zero" },
    pending = { PENDING_JSON, true, 1, true, "Unimplemented feature" },
    undefined = { UNDEFINED_JSON, true, 1, true, "Missing steps" },
)]
fn parses_scenario_results(
    json: &str,
    expect_passed: bool,
    test_count: usize,
    first_skipped: bool,
    first_name: &str,
) {
    let result = parse_cucumber_json(json, Duration::from_secs(1));
    assert_eq!(result.passed, expect_passed, "passed mismatch");
    assert_eq!(result.tests.len(), test_count, "test count mismatch");
    assert_eq!(result.tests[0].skipped, first_skipped, "skipped mismatch");
    assert_eq!(result.tests[0].name, first_name, "name mismatch");
}

#[test]
fn parses_passing_scenario_details() {
    let result = parse_cucumber_json(PASSING_JSON, Duration::from_secs(1));

    assert!(result.tests[0].passed);
    // Duration is sum of step durations in nanoseconds
    assert_eq!(result.tests[0].duration, Duration::from_nanos(1500000));
}

#[test]
fn parses_failing_scenario_details() {
    let result = parse_cucumber_json(FAILING_JSON, Duration::from_secs(1));

    assert!(!result.tests[0].passed);
}

#[test]
fn parses_multiple_features() {
    let output = r#"[
        {
            "uri": "features/one.feature",
            "name": "Feature One",
            "elements": [
                {
                    "type": "scenario",
                    "name": "Scenario A",
                    "steps": [{"name": "step", "result": {"status": "passed", "duration": 100}}]
                }
            ]
        },
        {
            "uri": "features/two.feature",
            "name": "Feature Two",
            "elements": [
                {
                    "type": "scenario",
                    "name": "Scenario B",
                    "steps": [{"name": "step", "result": {"status": "passed", "duration": 200}}]
                }
            ]
        }
    ]"#;
    let result = parse_cucumber_json(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
    assert_eq!(result.tests[0].name, "Scenario A");
    assert_eq!(result.tests[1].name, "Scenario B");
}

#[test]
fn ignores_background_elements() {
    let output = r#"[
        {
            "uri": "features/test.feature",
            "name": "Test",
            "elements": [
                {
                    "type": "background",
                    "name": "Setup",
                    "steps": [{"name": "setup step", "result": {"status": "passed", "duration": 100}}]
                },
                {
                    "type": "scenario",
                    "name": "Actual test",
                    "steps": [{"name": "test step", "result": {"status": "passed", "duration": 200}}]
                }
            ]
        }
    ]"#;
    let result = parse_cucumber_json(output, Duration::from_secs(1));

    assert_eq!(result.tests.len(), 1);
    assert_eq!(result.tests[0].name, "Actual test");
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[parameterized(
    empty_string = { "" },
    empty_array = { "[]" },
    non_json = { "Feature: Test\n  Scenario: example\n    Then it runs" },
)]
fn handles_empty_or_invalid_input(json: &str) {
    let result = parse_cucumber_json(json, Duration::from_secs(1));
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn handles_unnamed_scenario() {
    let output = r#"[
        {
            "uri": "features/test.feature",
            "name": "Test Feature",
            "elements": [
                {
                    "type": "scenario",
                    "name": "",
                    "steps": [{"name": "step", "result": {"status": "passed", "duration": 100}}]
                }
            ]
        }
    ]"#;
    let result = parse_cucumber_json(output, Duration::from_secs(1));

    assert_eq!(result.tests[0].name, "Scenario in Test Feature");
}

#[test]
fn handles_missing_result() {
    let output = r#"[
        {
            "uri": "features/test.feature",
            "name": "Test",
            "elements": [
                {
                    "type": "scenario",
                    "name": "No result",
                    "steps": [
                        {"name": "step without result"}
                    ]
                }
            ]
        }
    ]"#;
    let result = parse_cucumber_json(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
    assert_eq!(result.tests[0].duration, Duration::ZERO);
}

// =============================================================================
// JSON ARRAY FINDER TESTS
// =============================================================================

#[parameterized(
    simple = { r#"prefix [1, 2, 3] suffix"#, Some(r#"[1, 2, 3]"#) },
    nested = { r#"[[1], [2]]"#, Some(r#"[[1], [2]]"#) },
    no_array = { "no array here", None },
    empty = { "", None },
)]
fn find_json_array_cases(input: &str, expected: Option<&str>) {
    assert_eq!(find_json_array(input), expected);
}

// =============================================================================
// SCENARIO STATUS TESTS
// =============================================================================

#[parameterized(
    all_passed = {
        vec![("passed", 100), ("passed", 200)],
        "passed",
        300
    },
    one_failed = {
        vec![("passed", 100), ("failed", 50)],
        "failed",
        150
    },
    one_pending = {
        vec![("pending", 0)],
        "pending",
        0
    },
)]
fn scenario_status_from_steps(
    steps: Vec<(&str, u64)>,
    expected_status: &str,
    expected_duration_nanos: u64,
) {
    let cucumber_steps: Vec<CucumberStep> = steps
        .into_iter()
        .map(|(status, duration)| CucumberStep {
            result: Some(CucumberStepResult { status: status.to_string(), duration }),
        })
        .collect();

    let (status, duration) = scenario_status(&cucumber_steps);
    assert_eq!(status, expected_status);
    assert_eq!(duration, Duration::from_nanos(expected_duration_nanos));
}
