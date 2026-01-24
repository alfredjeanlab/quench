//! Behavioral specs for the Placeholders check.
//!
//! Tests that quench correctly:
//! - Detects #[ignore] tests in Rust
//! - Detects todo!() in Rust test bodies
//! - Detects test.todo() in JavaScript
//! - Detects test.fixme() in JavaScript
//! - Respects check = "off" configuration
//! - Generates appropriate violations and metrics

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST PLACEHOLDER SPECS
// =============================================================================

/// Spec: Rust #[ignore] detection
///
/// > When a test has #[ignore] attribute, report a placeholder violation.
#[test]
fn placeholders_detects_rust_ignore() {
    check("placeholders")
        .on("placeholders/rust-ignore")
        .fails()
        .stdout_has("ignore")
        .stdout_has("test_parser");
}

/// Spec: Rust todo!() detection
///
/// > When a test body contains todo!(), report a placeholder violation.
#[test]
fn placeholders_detects_rust_todo_body() {
    check("placeholders")
        .on("placeholders/rust-todo")
        .fails()
        .stdout_has("todo")
        .stdout_has("test_lexer");
}

// =============================================================================
// JAVASCRIPT PLACEHOLDER SPECS
// =============================================================================

/// Spec: JavaScript test.todo() detection
///
/// > When a test file contains test.todo('...') or it.todo('...'),
/// > report placeholder violations.
#[test]
fn placeholders_detects_js_test_todo() {
    check("placeholders")
        .on("placeholders/javascript-todo")
        .fails()
        .stdout_has("todo")
        .stdout_has("should handle edge case");
}

/// Spec: JavaScript test.fixme() detection
///
/// > When a test file contains test.fixme('...') or it.fixme('...'),
/// > report placeholder violations.
#[test]
fn placeholders_detects_js_test_fixme() {
    check("placeholders")
        .on("placeholders/javascript-fixme")
        .fails()
        .stdout_has("fixme")
        .stdout_has("broken on empty input");
}

// =============================================================================
// CONFIGURATION SPECS
// =============================================================================

/// Spec: check = "off" disables placeholders check
///
/// > When configured with check = "off", the check passes even with placeholders.
#[test]
fn placeholders_off_config_passes() {
    check("placeholders").on("placeholders/allowed").passes();
}

/// Spec: warn mode reports but passes
///
/// > When configured with check = "warn", violations are reported but check passes.
#[test]
fn placeholders_warn_mode_passes_with_warnings() {
    let temp = default_project();
    temp.config(
        r#"
[check.placeholders]
check = "warn"
"#,
    );
    temp.file(
        "tests/parser_test.rs",
        r#"
#[test]
#[ignore = "TODO"]
fn test_parser() { todo!() }
"#,
    );

    check("placeholders")
        .pwd(temp.path())
        .passes()
        .stdout_has("placeholders: WARN")
        .stdout_has("PASS: placeholders");
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: JSON output includes metrics
///
/// > JSON output includes rust and javascript metrics.
#[test]
fn placeholders_json_includes_metrics() {
    let result = check("placeholders")
        .on("placeholders/rust-ignore")
        .json()
        .fails();

    let metrics = result.require("metrics");
    assert!(metrics.get("rust").is_some(), "missing rust metrics");
    assert!(
        metrics.get("javascript").is_some(),
        "missing javascript metrics"
    );
}

/// Spec: JSON violation structure
///
/// > Each violation has file, line, type, and advice fields.
#[test]
fn placeholders_json_violation_structure() {
    let result = check("placeholders")
        .on("placeholders/rust-ignore")
        .json()
        .fails();

    let violations = result.require("violations").as_array().unwrap();
    assert!(!violations.is_empty(), "should have violations");

    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("line").is_some(), "missing line");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("advice").is_some(), "missing advice");
    }
}

/// Spec: Metrics count placeholders correctly
///
/// > Rust ignore count in metrics matches actual ignored tests.
#[test]
fn placeholders_metrics_rust_ignore_count() {
    let result = check("placeholders")
        .on("placeholders/rust-ignore")
        .json()
        .fails();

    let metrics = result.require("metrics");
    let rust_ignore = metrics["rust"]["ignore"].as_u64().unwrap();
    assert_eq!(rust_ignore, 1, "should detect one #[ignore] test");
}

/// Spec: Metrics count JavaScript todo correctly
#[test]
fn placeholders_metrics_js_todo_count() {
    let result = check("placeholders")
        .on("placeholders/javascript-todo")
        .json()
        .fails();

    let metrics = result.require("metrics");
    let js_todo = metrics["javascript"]["todo"].as_u64().unwrap();
    assert_eq!(js_todo, 2, "should detect two test.todo() calls");
}
