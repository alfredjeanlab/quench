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
