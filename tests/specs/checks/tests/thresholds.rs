// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for CI mode threshold violations.
//!
//! Reference: docs/specs/checks/tests.md#coverage, #test-time
//!
//! This module covers:
//! - Check level behavior (error/warn/off)
//! - Threshold violation output format
//!
//! Core threshold specs (coverage_below_min, time_total_exceeded, etc.)
//! are in ci_metrics.rs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CHECK LEVEL BEHAVIOR: COVERAGE
// =============================================================================

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > [check.tests.coverage]
/// > check = "warn" - report but don't fail
#[test]
fn coverage_warn_level_reports_but_passes() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "warn"
min = 99
"#,
    );
    // Only one function tested out of two = ~50% coverage, well below 99%
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    // Should PASS (exit 0) even with low coverage when check = "warn"
    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();

    // But should still report the violation
    assert!(result.has_violation("coverage_below_min"));
}

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > [check.tests.coverage]
/// > check = "off" - disable coverage checking
#[test]
fn coverage_off_skips_threshold_checking() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "off"
min = 99
"#,
    );
    // Very low coverage but should not generate violations
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
pub fn also_uncovered() -> i32 { 1 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    // Should PASS with no coverage violations when check = "off"
    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();

    // No coverage_below_min violation should be generated
    assert!(
        !result.has_violation("coverage_below_min"),
        "check = 'off' should not generate coverage violations"
    );
}

// =============================================================================
// CHECK LEVEL BEHAVIOR: TIME
// =============================================================================

/// Spec: docs/specs/checks/tests.md#test-time
///
/// > [check.tests.time]
/// > check = "warn" - report but don't fail
#[test]
fn time_warn_level_reports_but_passes() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "warn"
"#,
    );

    // Should PASS (exit 0) even with exceeded time when check = "warn"
    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();

    // But should still report the violation
    assert!(result.has_violation("time_total_exceeded"));
}

/// Spec: docs/specs/checks/tests.md#test-time
///
/// > [check.tests.time]
/// > check = "off" - disable time checking
#[test]
fn time_off_skips_threshold_checking() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "off"
"#,
    );

    // Should PASS with no time violations when check = "off"
    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();

    // No time_total_exceeded violation should be generated
    assert!(
        !result.has_violation("time_total_exceeded"),
        "check = 'off' should not generate time violations"
    );
}

// =============================================================================
// VIOLATION OUTPUT FORMAT
// =============================================================================

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > Coverage violation includes package name for per-package thresholds.
#[test]
fn package_coverage_violation_includes_package_name() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 50

[check.tests.coverage.package.root]
min = 95
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    let v = result.require_violation("coverage_below_min");
    let advice = v.get("advice").and_then(|v| v.as_str()).unwrap();
    // Advice should mention the package name
    assert!(
        advice.contains("root") || advice.contains("Package"),
        "advice should reference the package: {}",
        advice
    );
}

/// Spec: docs/specs/checks/tests.md#test-time
///
/// > Time violation includes test name for max_test exceeded.
#[test]
fn time_test_violation_includes_test_name() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
max_test = "5ms"

[check.tests.time]
check = "error"
"#,
    );
    // Create a bats test with a recognizable name that exceeds threshold
    temp.file(
        "tests/named_slow_test.bats",
        r#"
#!/usr/bin/env bats

@test "the_slow_test_name" {
    sleep 0.02
    [ 1 -eq 1 ]
}
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    let v = result.require_violation("time_test_exceeded");
    let advice = v.get("advice").and_then(|v| v.as_str()).unwrap();
    // Advice should include the test name
    assert!(advice.contains("the_slow_test_name"), "advice should include test name: {}", advice);
}

/// Spec: docs/specs/checks/tests.md#test-time
///
/// > Suite name appears in time_total_exceeded violations.
#[test]
fn time_total_violation_includes_suite_name() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    let v = result.require_violation("time_total_exceeded");
    let advice = v.get("advice").and_then(|v| v.as_str()).unwrap();
    // Advice should mention the suite
    assert!(
        advice.contains("Suite") || advice.contains("cargo"),
        "advice should reference suite: {}",
        advice
    );
}

// =============================================================================
// THRESHOLD FIELD STRUCTURE
// =============================================================================

/// Spec: docs/specs/checks/tests.md#json-output
///
/// > Threshold violations include `value` and `threshold` fields for ratcheting.
#[test]
fn coverage_violation_has_threshold_field() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    let v = result.require_violation("coverage_below_min");

    // Violation should have value (actual coverage) and threshold (min required)
    assert!(v.get("value").is_some(), "coverage violation should have value field");
    assert!(v.get("threshold").is_some(), "coverage violation should have threshold field");
}

/// Spec: docs/specs/checks/tests.md#json-output
///
/// > Time violations include both `value` and `threshold` fields.
#[test]
fn time_violation_has_threshold_and_value_fields() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    let v = result.require_violation("time_total_exceeded");

    // Should have value and threshold for ratcheting
    assert!(v.get("value").is_some(), "time violation should have value field");
    assert!(v.get("threshold").is_some(), "time violation should have threshold field");
}

// =============================================================================
// TEXT OUTPUT FORMAT
// =============================================================================

/// Spec: docs/specs/checks/tests.md#output
///
/// > Coverage violation text format includes percentage.
#[test]
fn coverage_violation_text_shows_percentage() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("coverage_below_min")
        .stdout_has("%");
}

/// Spec: docs/specs/checks/tests.md#output
///
/// > Time violation text format includes milliseconds.
#[test]
fn time_violation_text_shows_duration() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#,
    );

    check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("time_total_exceeded")
        .stdout_has("ms");
}

// =============================================================================
// COMBINED CHECK LEVELS
// =============================================================================

/// Spec: Combined coverage warn + time error behaves correctly.
///
/// > When coverage.check = "warn" and time.check = "error",
/// > coverage violations are reported but only time violations fail.
#[test]
fn mixed_check_levels_behave_correctly() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.coverage]
check = "warn"
min = 99

[check.tests.time]
check = "error"
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    // Should FAIL due to time error, not coverage warn
    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    // Both violations should be present
    assert!(result.has_violation("coverage_below_min"));
    assert!(result.has_violation("time_total_exceeded"));
}
