// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for placeholder metrics in the tests check.
//!
//! Tests that quench correctly:
//! - Collects #[ignore] counts from Rust test files
//! - Collects todo!() counts from Rust test bodies
//! - Collects test.todo() counts from JavaScript
//! - Collects test.fixme() counts from JavaScript
//! - Collects test.skip() counts from JavaScript
//! - Includes placeholder metrics in tests check JSON output

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST PLACEHOLDER METRICS SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#placeholder-metrics
///
/// > Rust #[ignore] tests are counted in metrics.placeholders.rust.ignore
#[test]
fn tests_check_includes_rust_ignore_metrics() {
    let result = check("tests").on("placeholders/rust-ignore").json().passes();

    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("missing placeholders metrics");
    let rust_ignore = placeholders["rust"]["ignore"].as_u64().unwrap();
    assert_eq!(rust_ignore, 1, "should detect one #[ignore] test");
}

/// Spec: docs/specs/checks/tests.md#placeholder-metrics
///
/// > Rust todo!() in test bodies are counted in metrics.placeholders.rust.todo
#[test]
fn tests_check_includes_rust_todo_metrics() {
    let result = check("tests").on("placeholders/rust-todo").json().passes();

    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("missing placeholders metrics");
    let rust_todo = placeholders["rust"]["todo"].as_u64().unwrap();
    assert_eq!(rust_todo, 1, "should detect one todo!() in test body");
}

// =============================================================================
// JAVASCRIPT PLACEHOLDER METRICS SPECS
// =============================================================================

/// Spec: docs/specs/checks/tests.md#placeholder-metrics
///
/// > JavaScript test.todo() calls are counted in metrics.placeholders.javascript.todo
#[test]
fn tests_check_includes_js_todo_metrics() {
    let result = check("tests").on("placeholders/javascript-todo").json().passes();

    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("missing placeholders metrics");
    let js_todo = placeholders["javascript"]["todo"].as_u64().unwrap();
    assert_eq!(js_todo, 2, "should detect two test.todo() calls");
}

/// Spec: docs/specs/checks/tests.md#placeholder-metrics
///
/// > JavaScript test.fixme() calls are counted in metrics.placeholders.javascript.fixme
#[test]
fn tests_check_includes_js_fixme_metrics() {
    let result = check("tests").on("placeholders/javascript-fixme").json().passes();

    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("missing placeholders metrics");
    let js_fixme = placeholders["javascript"]["fixme"].as_u64().unwrap();
    assert_eq!(js_fixme, 2, "should detect two test.fixme() calls");
}

// =============================================================================
// PLACEHOLDER METRICS STRUCTURE SPEC
// =============================================================================

/// Spec: docs/specs/checks/tests.md#metrics-structure
///
/// > JSON output includes placeholders object with rust and javascript subobjects
#[test]
fn tests_check_placeholder_metrics_structure() {
    let result = check("tests").on("placeholders/rust-ignore").json().passes();

    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("missing placeholders metrics");

    // Verify rust structure
    let rust = placeholders.get("rust").expect("missing rust metrics");
    assert!(rust.get("ignore").is_some(), "missing rust.ignore");
    assert!(rust.get("todo").is_some(), "missing rust.todo");

    // Verify javascript structure
    let js = placeholders.get("javascript").expect("missing javascript metrics");
    assert!(js.get("todo").is_some(), "missing javascript.todo");
    assert!(js.get("fixme").is_some(), "missing javascript.fixme");
    assert!(js.get("skip").is_some(), "missing javascript.skip");
}

// =============================================================================
// CORRELATION BEHAVIOR SPECS (preserved from original)
// =============================================================================

/// Spec: docs/specs/checks/tests.md#placeholder-tests
///
/// > When placeholders = "allow", placeholder tests satisfy correlation.
/// > This behavior is preserved after the refactor.
#[test]
fn tests_check_placeholders_allow_satisfies_correlation() {
    let temp = default_project();
    temp.config(
        r#"
version = 1

[check.tests.commit]
check = "error"
placeholders = "allow"
"#,
    );
    // Source file that would normally need tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    // Placeholder test satisfies correlation
    temp.file(
        "tests/parser_tests.rs",
        r#"
#[test]
#[ignore = "TODO: implement parser tests"]
fn test_parser() { todo!() }
"#,
    );

    // Should pass because placeholder satisfies correlation
    check("tests").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/tests.md#correlation-vs-metrics
///
/// > Placeholders are always counted in metrics, regardless of correlation setting.
#[test]
fn tests_check_metrics_collected_regardless_of_correlation() {
    let temp = default_project();
    temp.config(
        r#"
version = 1

[check.tests.commit]
check = "error"
placeholders = "forbid"
"#,
    );
    temp.file(
        "tests/parser_tests.rs",
        r#"
#[test]
#[ignore = "TODO"]
fn test_parser() { todo!() }
"#,
    );

    // Even with placeholders = "forbid", metrics should still be collected
    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");
    let placeholders = metrics.get("placeholders").expect("metrics should include placeholders");
    assert!(
        placeholders["rust"]["ignore"].as_u64().unwrap() >= 1,
        "should count #[ignore] even when correlation disabled"
    );
}
