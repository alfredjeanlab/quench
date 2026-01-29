// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for test runner execution.
//!
//! Reference: docs/specs/11-test-runners.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CARGO RUNNER SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#cargo
///
/// > cargo test --release -- --format json
#[test]
fn cargo_runner_executes_cargo_test() {
    let temp = Project::cargo("test_project");

    // Runner should execute cargo test and report results
    check("tests")
        .pwd(temp.path())
        .passes()
        .stdout_has("PASS: tests");
}

/// Spec: docs/specs/11-test-runners.md#cargo
///
/// > Parses Rust's JSON test output for per-test timing.
#[test]
fn cargo_runner_reports_test_count() {
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
    temp.file("src/lib.rs", "");
    temp.file("tests/a.rs", "#[test] fn t1() {} #[test] fn t2() {}");
    temp.file("tests/b.rs", "#[test] fn t3() {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report test count
    assert_eq!(metrics.get("test_count").and_then(|v| v.as_i64()), Some(3));
}

/// Spec: Integration test on fixtures/rust-simple
#[test]
fn cargo_runner_on_rust_simple_fixture() {
    // rust-simple fixture has quench.toml with [[check.tests.suite]] runner = "cargo"
    check("tests").on("rust-simple").passes();
}

// =============================================================================
// BATS RUNNER SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > bats --timing tests/
#[test]
fn bats_runner_executes_bats_with_timing() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#,
    );
    temp.file(
        "tests/basic.bats",
        r#"
@test "example test" {
    [ 1 -eq 1 ]
}
"#,
    );

    // Runner should execute bats --timing
    check("tests")
        .pwd(temp.path())
        .passes()
        .stdout_has("PASS: tests");
}

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > Parses BATS TAP output with timing information.
#[test]
fn bats_runner_parses_tap_timing() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#,
    );
    temp.file(
        "tests/a.bats",
        r#"
@test "test one" { sleep 0.1; }
@test "test two" { true; }
"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report test count from TAP output
    assert_eq!(metrics.get("test_count").and_then(|v| v.as_i64()), Some(2));
}

/// Spec: Integration test on fixtures/shell-scripts
#[test]
fn bats_runner_on_shell_scripts_fixture() {
    // shell-scripts fixture has quench.toml with [[check.tests.suite]] runner = "bats"
    check("tests").on("shell-scripts").passes();
}

// =============================================================================
// GO RUNNER SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#go
///
/// > go test -json ./...
#[test]
fn go_runner_executes_go_test() {
    // go-simple fixture has quench.toml with [[check.tests.suite]] runner = "go"
    check("tests")
        .on("go-simple")
        .passes()
        .stdout_has("PASS: tests");
}

/// Spec: docs/specs/11-test-runners.md#go
///
/// > Parses Go's JSON test output for per-test timing.
#[test]
fn go_runner_reports_test_count() {
    let result = check("tests").on("go-simple").json().passes();
    let metrics = result.require("metrics");

    // Should report test count
    let test_count = metrics.get("test_count").and_then(|v| v.as_i64());
    assert!(test_count.is_some(), "Expected test_count in metrics");
    assert!(test_count.unwrap() > 0, "Expected at least 1 test");
}

/// Spec: Integration test on fixtures/go-simple
#[test]
fn go_runner_on_go_simple_fixture() {
    // go-simple fixture has quench.toml with [[check.tests.suite]] runner = "go"
    check("tests").on("go-simple").passes();
}

/// Spec: Integration test on fixtures/go-multi
#[test]
fn go_runner_on_go_multi_fixture() {
    // go-multi fixture has quench.toml with [[check.tests.suite]] runner = "go"
    check("tests").on("go-multi").passes();
}

/// Spec: docs/specs/11-test-runners.md#go-coverage
///
/// > Go runner provides coverage via `go test -coverprofile`.
#[test]
fn go_runner_collects_coverage() {
    let result = check("tests")
        .on("go-simple")
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report Go coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some(), "Expected coverage in metrics");

    let go_coverage = coverage.unwrap().get("go").and_then(|v| v.as_f64());
    assert!(go_coverage.is_some(), "Expected 'go' in coverage");

    // go-simple has a simple test that should cover the Add function
    let pct = go_coverage.unwrap();
    assert!(
        pct > 0.0,
        "Expected positive coverage percentage, got {}",
        pct
    );
}

/// Spec: docs/specs/11-test-runners.md#go-coverage
///
/// > Go runner reports per-package coverage.
#[test]
fn go_runner_reports_package_coverage() {
    let result = check("tests")
        .on("go-multi")
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report package coverage
    let coverage_by_package = metrics
        .get("coverage_by_package")
        .and_then(|v| v.as_object());
    assert!(
        coverage_by_package.is_some(),
        "Expected coverage_by_package in metrics"
    );

    let packages = coverage_by_package.unwrap();
    // go-multi has pkg/api, pkg/storage, and internal/core packages
    // At least one package should have coverage data
    assert!(
        !packages.is_empty(),
        "Expected at least one package with coverage"
    );
}
