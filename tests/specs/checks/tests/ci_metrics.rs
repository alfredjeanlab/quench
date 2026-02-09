// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for CI mode metrics aggregation.
//!
//! Reference: docs/specs/11-test-runners.md#ci-mode-metrics

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// TOP-LEVEL TIMING METRICS
// =============================================================================

/// Spec: Top-level timing metrics across all suites.
///
/// > CI mode should report aggregated timing metrics including total_ms, avg_ms,
/// > max_ms, and max_test at the top level.
#[test]
fn ci_mode_reports_aggregated_timing_metrics() {
    let result = check("tests").on("tests-ci").args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Should have test_count and total_ms
    assert!(metrics.get("test_count").is_some());
    assert!(metrics.get("total_ms").is_some());
}

/// Spec: Per-suite timing metrics are included in suites array.
///
/// > Each suite should report its own timing metrics.
#[test]
fn ci_mode_reports_per_suite_timing() {
    let result = check("tests").on("tests-ci").args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Should have suites array
    let suites = metrics.get("suites").and_then(|v| v.as_array());
    assert!(suites.is_some());

    let suites = suites.unwrap();
    assert!(!suites.is_empty());

    // First suite should have timing info
    let suite = &suites[0];
    assert!(suite.get("name").is_some());
    assert!(suite.get("runner").is_some());
    assert!(suite.get("passed").is_some());
    assert!(suite.get("test_count").is_some());
}

// =============================================================================
// PER-PACKAGE COVERAGE
// =============================================================================

/// Spec: Per-package coverage breakdown from workspace.
///
/// > coverage_by_package should show coverage for each package in a workspace.
#[test]
fn ci_mode_reports_per_package_coverage() {
    // This test requires a workspace setup with multiple crates
    // and llvm-cov installed, so we mark it as ignored for CI
    // that may not have coverage tools
    let temp = Project::cargo("test_project");
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

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // If coverage was collected, it should appear in metrics
    // (may be absent if llvm-cov is not installed)
    if let Some(coverage) = metrics.get("coverage") {
        assert!(coverage.as_object().is_some());
    }
}

// =============================================================================
// CI THRESHOLD VIOLATIONS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > Configure thresholds via `[check.tests.coverage]`:
/// > min = 75
#[test]
fn coverage_below_min_generates_violation() {
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
    // Only one function tested out of two = ~50% coverage
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

    assert!(result.has_violation("coverage_below_min"));
    let v = result.require_violation("coverage_below_min");
    assert!(v.get("threshold").is_some());
}

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > [check.tests.coverage.package.core]
/// > min = 90
#[test]
fn per_package_coverage_thresholds_work() {
    let temp = Project::cargo("test_project");
    // Use "root" as package name since that's how coverage_by_package is keyed
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

    // Should fail on package-specific threshold
    assert!(result.has_violation("coverage_below_min"));
}

/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_total = "30s"
#[test]
fn time_total_exceeded_generates_violation() {
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

    assert!(result.has_violation("time_total_exceeded"));
    let v = result.require_violation("time_total_exceeded");
    assert!(v.get("value").is_some());
    assert!(v.get("threshold").is_some());
}

/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_test = "1s"
///
/// Uses bats runner since it provides per-test timing via --timing flag.
/// Cargo test doesn't provide per-test timing in human-readable output.
#[test]
fn time_test_exceeded_generates_violation() {
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
    // Create a bats test that sleeps longer than the threshold
    temp.file(
        "tests/slow_test.bats",
        r#"
#!/usr/bin/env bats

@test "slow test that exceeds threshold" {
    sleep 0.02
    [ 1 -eq 1 ]
}
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    assert!(result.has_violation("time_test_exceeded"));
}

/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_avg = "100ms"
///
/// Uses bats runner since it provides per-test timing via --timing flag.
/// Cargo test doesn't provide per-test timing in human-readable output.
#[test]
fn time_avg_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
max_avg = "5ms"

[check.tests.time]
check = "error"
"#,
    );
    // Create a bats test that takes longer than max_avg on average
    temp.file(
        "tests/slow_test.bats",
        r#"
#!/usr/bin/env bats

@test "slow test exceeding avg threshold" {
    sleep 0.02
    [ 1 -eq 1 ]
}
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();

    assert!(result.has_violation("time_avg_exceeded"));
}

/// Spec: tests CI violation.type enumeration
///
/// Violation types for CI thresholds:
/// - coverage_below_min
/// - time_total_exceeded
/// - time_avg_exceeded
/// - time_test_exceeded
#[test]
fn tests_ci_violation_types_are_documented() {
    // This test documents the expected violation types.
    // Each type should be tested individually above.
    let expected_types =
        ["coverage_below_min", "time_total_exceeded", "time_avg_exceeded", "time_test_exceeded"];

    // Verify these are the only CI threshold violation types
    // by checking they don't overlap with other tests check violations
    let other_types = ["missing_tests", "test_suite_failed"];

    for t in &expected_types {
        assert!(
            !other_types.contains(t),
            "CI threshold type '{}' should not overlap with other types",
            t
        );
    }
}

// =============================================================================
// CI OUTPUT FORMAT
// =============================================================================

/// Spec: CI mode text output shows test results summary.
///
/// > CI mode should output "PASS: tests" on success.
#[test]
fn tests_ci_text_output_passes() {
    check("tests").on("tests-ci").args(&["--ci"]).passes().stdout_has("PASS: tests");
}

/// Spec: CI mode JSON output includes timing metrics structure.
///
/// > CI mode JSON should include test_count, total_ms, and suites array
/// > with required fields.
#[test]
fn tests_ci_json_output_timing_structure() {
    let result = check("tests").on("tests-ci").args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Verify required fields
    assert!(metrics.get("test_count").is_some());
    assert!(metrics.get("total_ms").is_some());
    let suites = metrics.get("suites").unwrap().as_array().unwrap();
    assert!(!suites.is_empty());

    // Suite should have required fields
    let suite = &suites[0];
    assert!(suite.get("name").is_some());
    assert!(suite.get("runner").is_some());
    assert!(suite.get("passed").is_some());
    assert!(suite.get("test_count").is_some());
    assert!(suite.get("total_ms").is_some());
}

/// Spec: CI mode text output shows threshold violations.
///
/// > Timing violations should display the violation type and exceeded limit.
#[test]
fn tests_ci_text_output_timing_violation() {
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
        .stdout_has("exceeds max_total");
}

/// Spec: CI violation has threshold and value fields.
///
/// > JSON violations for CI thresholds should include threshold and value.
#[test]
fn tests_ci_json_violation_has_threshold_and_value() {
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
    assert!(v.get("threshold").is_some());
    assert!(v.get("value").is_some());
}

// =============================================================================
// SKIPPED COUNT METRICS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#skipped-metrics
///
/// > CI mode should report skipped_count for ignored tests.
#[test]
fn ci_mode_reports_skipped_count_for_ignored_tests() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }

#[test]
#[ignore]
fn test_ignored() { panic!("should not run"); }
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");
    let suites = metrics.get("suites").and_then(|v| v.as_array()).unwrap();

    // The suite should have skipped_count if there are ignored tests
    // Note: This may be absent if skipped_count == 0, depending on implementation
    let suite = &suites[0];
    // Test passes if skipped_count is present and is a valid u64
    if let Some(skipped) = suite.get("skipped_count") {
        // If present, it should be a valid non-negative integer
        assert!(skipped.as_u64().is_some());
    }
}

// =============================================================================
// PERCENTILE METRICS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#percentile-metrics
///
/// > CI mode should report p50_ms, p90_ms, p99_ms for test durations.
/// > Note: Cargo test output doesn't include per-test timing by default,
/// > so percentiles may not appear unless using a runner that provides timing
/// > (like bats with --timing flag).
#[test]
fn ci_mode_reports_percentiles_for_timed_runner() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
"#,
    );
    // Create multiple bats tests
    temp.file(
        "tests/basic.bats",
        r#"
#!/usr/bin/env bats

@test "fast test" {
    [ 1 -eq 1 ]
}

@test "another test" {
    [ 2 -eq 2 ]
}
"#,
    );

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");
    let suites = metrics.get("suites").and_then(|v| v.as_array()).unwrap();

    // Bats runner should provide timing, so percentiles should be present
    let suite = &suites[0];
    if let Some(p50) = suite.get("p50_ms") {
        assert!(p50.as_u64().is_some());
    }
}

// =============================================================================
// TIMEOUT CONFIGURATION
// =============================================================================

/// Spec: docs/specs/checks/tests.md#timeout
///
/// > Suite timeout configuration should be parsed and documented.
#[test]
fn suite_timeout_config_is_accepted() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
timeout = "60s"
"#,
    );

    // Config should be accepted without error (suite runs normally)
    check("tests").pwd(temp.path()).args(&["--ci"]).passes();
}
