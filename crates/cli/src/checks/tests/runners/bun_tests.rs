#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use super::*;

// Bun uses the same JSON format as Jest, so we test the parsing logic here
// to ensure it works for Bun's output as well.

#[test]
fn parses_passing_tests() {
    let output = r#"{
        "success": true,
        "numTotalTests": 2,
        "numPassedTests": 2,
        "numFailedTests": 0,
        "testResults": [
            {
                "name": "/path/to/utils.test.ts",
                "assertionResults": [
                    {"fullName": "adds numbers", "status": "passed", "duration": 45},
                    {"fullName": "multiplies numbers", "status": "passed", "duration": 23}
                ]
            }
        ]
    }"#;
    let result = parse_jest_json(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
}

#[test]
fn parses_failing_tests() {
    let output = r#"{
        "success": false,
        "numTotalTests": 2,
        "numPassedTests": 1,
        "numFailedTests": 1,
        "testResults": [
            {
                "name": "/path/to/utils.test.ts",
                "assertionResults": [
                    {"fullName": "adds numbers", "status": "passed", "duration": 45},
                    {"fullName": "handles errors", "status": "failed", "duration": 23}
                ]
            }
        ]
    }"#;
    let result = parse_jest_json(output, Duration::from_secs(1));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 2);
    assert!(!result.tests[1].passed);
}

#[test]
fn handles_empty_output() {
    let result = parse_jest_json("", Duration::from_secs(0));
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn handles_bun_specific_output_prefix() {
    // Bun may include some output before the JSON
    let output = r#"
bun test v1.0.0

test.ts:
  ✓ example test [10.00ms]

{"success": true, "testResults": [{"name": "test.ts", "assertionResults": [{"fullName": "example test", "status": "passed", "duration": 10}]}]}
"#;
    let result = parse_jest_json(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
}
