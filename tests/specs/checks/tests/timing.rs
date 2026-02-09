// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for per-test timing extraction.
//!
//! Reference: docs/specs/11-test-runners.md#timing-metrics

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CARGO TIMING EXTRACTION
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#per-test-timing
///
/// > Average: Mean time per test
#[test]
#[ignore = "Blocked: Cargo test doesn't provide per-test timing in stable output"]
fn cargo_runner_extracts_average_timing() {
    let temp = Project::cargo("test_project");
    temp.file("src/lib.rs", "");
    temp.file(
        "tests/timing.rs",
        r#"
use std::thread::sleep;
use std::time::Duration;

#[test] fn fast_test() { sleep(Duration::from_millis(10)); }
#[test] fn slow_test() { sleep(Duration::from_millis(100)); }
"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should have average timing
    assert!(metrics.get("avg_ms").is_some());
}

/// Spec: docs/specs/11-test-runners.md#per-test-timing
///
/// > Max: Slowest individual test (with name)
#[test]
#[ignore = "Blocked: Cargo test doesn't provide per-test timing in stable output"]
fn cargo_runner_extracts_max_timing_with_name() {
    let temp = Project::cargo("test_project");
    temp.file("src/lib.rs", "");
    temp.file(
        "tests/timing.rs",
        r#"
use std::thread::sleep;
use std::time::Duration;

#[test] fn fast_test() { sleep(Duration::from_millis(10)); }
#[test] fn slowest_test() { sleep(Duration::from_millis(200)); }
#[test] fn medium_test() { sleep(Duration::from_millis(50)); }
"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should identify slowest test by name
    let max_test = metrics.get("max_test").and_then(|v| v.as_str());
    assert!(max_test.unwrap().contains("slowest_test"));
}

/// Spec: docs/specs/11-test-runners.md#timing-metrics
///
/// > Total Time: Wall-clock time for entire test suite.
#[test]
fn runner_reports_total_time() {
    let temp = Project::cargo("test_project");
    temp.file("src/lib.rs", "");
    temp.file("tests/basic.rs", "#[test] fn t() {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report total time
    assert!(metrics.get("total_ms").is_some());
}

// =============================================================================
// BATS TIMING EXTRACTION
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > Parses BATS TAP output with timing information.
#[test]
fn bats_runner_extracts_per_test_timing() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#,
    );
    temp.file(
        "tests/timing.bats",
        r#"
@test "fast test" { true; }
@test "slow test" { sleep 0.2; }
"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should extract timing from TAP output
    assert!(metrics.get("max_ms").is_some());
    let max_test = metrics.get("max_test").and_then(|v| v.as_str());
    assert!(max_test.unwrap().contains("slow test"));
}

// =============================================================================
// TIMING THRESHOLD SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_test = "500ms" - Max time for slowest individual test
#[test]
#[ignore = "Blocked: Cargo test doesn't provide per-test timing in stable output"]
fn runner_fails_when_test_exceeds_max_time() {
    let temp = Project::cargo("test_project");
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_test = "50ms"
"#,
    );
    temp.file("src/lib.rs", "");
    temp.file(
        "tests/slow.rs",
        r#"
use std::thread::sleep;
use std::time::Duration;

#[test] fn slow_test() { sleep(Duration::from_millis(200)); }
"#,
    );

    // Should fail because test exceeds max_test threshold
    check("tests").pwd(temp.path()).fails().stdout_has("slow_test").stdout_has("exceeded");
}
