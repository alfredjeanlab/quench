// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn violation_file_constructor() {
    let v = Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.");
    assert_eq!(v.file, Some(PathBuf::from("src/main.rs")));
    assert_eq!(v.line, Some(42));
    assert_eq!(v.violation_type, "file_too_large");
    assert_eq!(v.advice, "Split into modules.");
}

#[test]
fn violation_with_threshold() {
    let v = Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.")
        .with_threshold(800, 750);
    assert_eq!(v.value, Some(800));
    assert_eq!(v.threshold, Some(750));
}

#[test]
fn check_result_passed() {
    let result = CheckResult::passed("cloc");
    assert!(result.passed);
    assert!(!result.skipped);
    assert!(result.violations.is_empty());
}

#[test]
fn check_result_failed() {
    let violations = vec![Violation::file(
        "src/main.rs",
        42,
        "file_too_large",
        "Split into modules.",
    )];
    let result = CheckResult::failed("cloc", violations);
    assert!(!result.passed);
    assert!(!result.skipped);
    assert_eq!(result.violations.len(), 1);
}

#[test]
fn check_result_skipped() {
    let result = CheckResult::skipped("cloc", "failed to read config");
    assert!(!result.passed);
    assert!(result.skipped);
    assert_eq!(result.error, Some("failed to read config".to_string()));
}

#[test]
fn check_output_passed_when_all_pass() {
    let checks = vec![CheckResult::passed("cloc"), CheckResult::passed("escapes")];
    let output = CheckOutput::new("2024-01-01T00:00:00Z".to_string(), checks);
    assert!(output.passed);
}

#[test]
fn check_output_failed_when_any_fails() {
    let checks = vec![
        CheckResult::passed("cloc"),
        CheckResult::failed("escapes", vec![]),
    ];
    let output = CheckOutput::new("2024-01-01T00:00:00Z".to_string(), checks);
    assert!(!output.passed);
}

#[test]
fn violation_serializes_to_json() {
    let v = Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.")
        .with_threshold(800, 750);
    let json = serde_json::to_value(&v).unwrap();

    assert_eq!(json["file"], "src/main.rs");
    assert_eq!(json["line"], 42);
    assert_eq!(json["type"], "file_too_large");
    assert_eq!(json["advice"], "Split into modules.");
    assert_eq!(json["value"], 800);
    assert_eq!(json["threshold"], 750);
}

#[test]
fn violation_omits_none_fields() {
    let v = Violation::file("src/main.rs", 42, "file_too_large", "Split into modules.");
    let json = serde_json::to_value(&v).unwrap();

    assert!(json.get("value").is_none());
    assert!(json.get("threshold").is_none());
    assert!(json.get("pattern").is_none());
}

#[test]
fn check_result_omits_empty_violations() {
    let result = CheckResult::passed("cloc");
    let json = serde_json::to_value(&result).unwrap();

    assert!(json.get("violations").is_none());
    assert!(json.get("skipped").is_none());
    assert!(json.get("error").is_none());
}

#[test]
fn violation_with_change_info_serializes_correctly() {
    let v = Violation::file_only("src/foo.rs", "missing_tests", "Add tests")
        .with_change_info("modified", 42);

    let json = serde_json::to_value(&v).unwrap();

    assert_eq!(json["change_type"], "modified");
    assert_eq!(json["lines_changed"], 42);
}

#[test]
fn violation_without_change_info_omits_fields() {
    let v = Violation::file_only("src/foo.rs", "missing_tests", "Add tests");

    let json = serde_json::to_value(&v).unwrap();

    assert!(json.get("change_type").is_none());
    assert!(json.get("lines_changed").is_none());
}
