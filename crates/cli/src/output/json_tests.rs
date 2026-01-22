#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{JsonFormatter, create_output};
use crate::check::{CheckResult, Violation};

#[test]
fn json_formatter_creates_successfully() {
    let mut buffer = Vec::new();
    let _formatter = JsonFormatter::new(&mut buffer);
}

#[test]
fn json_formatter_outputs_valid_json() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    assert!(json.get("passed").is_some());
    assert!(json.get("checks").is_some());
    assert!(json.get("timestamp").is_some());
}

#[test]
fn json_output_has_iso8601_timestamp() {
    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);

    // Check timestamp format: 2024-01-21T10:30:00Z
    assert!(output.timestamp.contains('T'));
    assert!(output.timestamp.ends_with('Z'));
}

#[test]
fn json_output_passed_is_true_when_all_pass() {
    let checks = vec![CheckResult::passed("cloc"), CheckResult::passed("escapes")];
    let output = create_output(checks);
    assert!(output.passed);
}

#[test]
fn json_output_passed_is_false_when_any_fails() {
    let checks = vec![
        CheckResult::passed("cloc"),
        CheckResult::failed("escapes", vec![]),
    ];
    let output = create_output(checks);
    assert!(!output.passed);
}

#[test]
fn json_violation_has_required_fields() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let violations = vec![Violation::file(
        "src/main.rs",
        42,
        "file_too_large",
        "Split into modules.",
    )];
    let checks = vec![CheckResult::failed("cloc", violations)];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();
    let violations = checks[0].get("violations").unwrap().as_array().unwrap();
    let violation = &violations[0];

    assert!(violation.get("type").is_some());
    assert!(violation.get("advice").is_some());
}

#[test]
fn json_check_has_required_fields() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();
    let check = &checks[0];

    assert!(check.get("name").is_some());
    assert!(check.get("passed").is_some());
}
