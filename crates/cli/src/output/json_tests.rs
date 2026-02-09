// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::{JsonFormatter, create_output};
use crate::check::{CheckResult, Violation};
use crate::timing::{PhaseTiming, TimingInfo};

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
    let checks = vec![CheckResult::passed("cloc"), CheckResult::failed("escapes", vec![])];
    let output = create_output(checks);
    assert!(!output.passed);
}

#[test]
fn json_violation_has_required_fields() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let violations =
        vec![Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.")];
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

#[test]
fn json_formats_empty_violations() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();

    // Violations array is always present for consistency (even when empty)
    let violations = checks[0].get("violations").expect("violations should be present");
    assert!(violations.as_array().unwrap().is_empty());
}

#[test]
fn json_formats_multiple_violations() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let violations = vec![
        Violation::file("src/a.rs", 10, "file_too_large", "Split into modules."),
        Violation::file("src/b.rs", 20, "too_many_lines", "Reduce line count."),
        Violation::file("src/c.rs", 30, "file_too_large", "Split into modules."),
    ];
    let checks = vec![CheckResult::failed("cloc", violations)];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();
    let violations = checks[0].get("violations").unwrap().as_array().unwrap();

    assert_eq!(violations.len(), 3);
    assert_eq!(violations[0].get("file").unwrap(), "src/a.rs");
    assert_eq!(violations[1].get("file").unwrap(), "src/b.rs");
    assert_eq!(violations[2].get("file").unwrap(), "src/c.rs");
}

#[test]
fn json_violation_includes_scope_when_set() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let violation = Violation::commit_violation(
        "abc123",
        "feat(api): add endpoint",
        "missing_docs",
        "Add documentation for api changes.",
    )
    .with_scope("api");

    let checks = vec![CheckResult::failed("docs", vec![violation])];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();
    let violations = checks[0].get("violations").unwrap().as_array().unwrap();
    let violation = &violations[0];

    assert_eq!(violation.get("scope").unwrap(), "api");
    assert_eq!(violation.get("commit").unwrap(), "abc123");
}

#[test]
fn json_violation_omits_scope_when_not_set() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let violation = Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.");

    let checks = vec![CheckResult::failed("cloc", vec![violation])];
    let output = create_output(checks);
    formatter.write(&output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
    let checks = json.get("checks").unwrap().as_array().unwrap();
    let violations = checks[0].get("violations").unwrap().as_array().unwrap();
    let violation = &violations[0];

    // scope should be omitted (skip_serializing_if = "Option::is_none")
    assert!(violation.get("scope").is_none());
}

#[test]
fn json_output_includes_timing_when_provided() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);

    let timing = TimingInfo {
        phases: PhaseTiming { discovery_ms: 10, checking_ms: 50, output_ms: 5, total_ms: 65 },
        files: 100,
        cache_hits: 80,
        checks: [("cloc".to_string(), 25)].into_iter().collect(),
    };

    formatter.write_with_timing(&output, None, Some(&timing)).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();

    // Timing is nested under "timing" key
    let timing_obj = json.get("timing").expect("timing should be present");

    // Phase timing is flattened into the timing object
    assert_eq!(timing_obj.get("discovery_ms").unwrap(), 10);
    assert_eq!(timing_obj.get("checking_ms").unwrap(), 50);
    assert_eq!(timing_obj.get("total_ms").unwrap(), 65);
    assert_eq!(timing_obj.get("files").unwrap(), 100);
    assert_eq!(timing_obj.get("cache_hits").unwrap(), 80);

    // Per-check timing
    let checks_timing = timing_obj.get("checks").unwrap();
    assert_eq!(checks_timing.get("cloc").unwrap(), 25);
}

#[test]
fn json_output_omits_timing_when_not_provided() {
    let mut buffer = Vec::new();
    let mut formatter = JsonFormatter::new(&mut buffer);

    let checks = vec![CheckResult::passed("cloc")];
    let output = create_output(checks);

    formatter.write_with_timing(&output, None, None).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&buffer).unwrap();

    // Timing fields should not be present
    assert!(json.get("discovery_ms").is_none());
    assert!(json.get("total_ms").is_none());
    assert!(json.get("files").is_none());
}
