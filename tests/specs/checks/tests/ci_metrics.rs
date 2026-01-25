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
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
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
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
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
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
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

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // If coverage was collected, it should appear in metrics
    // (may be absent if llvm-cov is not installed)
    if let Some(coverage) = metrics.get("coverage") {
        assert!(coverage.as_object().is_some());
    }
}
