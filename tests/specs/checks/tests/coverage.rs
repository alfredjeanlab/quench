//! Behavioral specs for test coverage collection.
//!
//! Reference: docs/specs/11-test-runners.md#coverage-targets

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST COVERAGE (llvm-cov)
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > `cargo` runner provides implicit Rust coverage via llvm-cov.
#[test]
fn cargo_runner_collects_rust_coverage() {
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

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report Rust coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let rust_coverage = coverage.unwrap().get("rust").and_then(|v| v.as_f64());
    assert!(rust_coverage.is_some());
    // Coverage should be ~50% (one function covered, one not)
    let pct = rust_coverage.unwrap();
    assert!(
        pct > 40.0 && pct < 60.0,
        "Expected ~50% coverage, got {}",
        pct
    );
}

// =============================================================================
// SHELL COVERAGE (kcov)
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#explicit-coverage
///
/// > Shell scripts via kcov: targets = ["scripts/*.sh"]
#[test]
#[ignore = "TODO: Phase 940 - Requires runner integration"]
fn bats_runner_collects_shell_coverage_via_kcov() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
targets = ["scripts/*.sh"]  # Shell scripts via kcov
"#,
    );
    temp.file(
        "scripts/helper.sh",
        r#"#!/bin/bash
covered_function() { echo "covered"; }
uncovered_function() { echo "uncovered"; }
"#,
    );
    temp.file(
        "tests/helper.bats",
        r#"
setup() { source scripts/helper.sh; }

@test "calls covered function" {
    run covered_function
    [ "$output" = "covered" ]
}
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report shell coverage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let shell_coverage = coverage.unwrap().get("shell").and_then(|v| v.as_f64());
    assert!(shell_coverage.is_some());
}

/// Spec: docs/specs/11-test-runners.md#explicit-coverage
///
/// > targets = ["myapp"] - Instrument Rust binary for coverage
#[test]
#[ignore = "TODO: Phase 940 - Requires runner integration"]
fn bats_runner_collects_rust_binary_coverage() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
targets = ["myapp"]  # Instrument Rust binary
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#,
    );
    temp.file(
        "src/main.rs",
        r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "greet" {
        println!("Hello!");
    } else {
        println!("Usage: myapp greet");
    }
}
"#,
    );
    temp.file(
        "tests/cli/basic.bats",
        r#"
@test "greet command" {
    run ./target/debug/myapp greet
    [ "$output" = "Hello!" ]
}
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report coverage for Rust binary
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.unwrap().get("rust").is_some());
}

// =============================================================================
// COVERAGE MERGING
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#aggregation
///
/// > Coverage: Merged across suites covering the same language
#[test]
#[ignore = "TODO: Phase 940 - Requires runner integration"]
fn multiple_suite_coverages_merged() {
    let temp = Project::empty();
    temp.config(
        r#"
# Suite 1: Unit tests (covers internal functions)
[[check.tests.suite]]
runner = "cargo"

# Suite 2: Integration tests (covers main binary)
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
targets = ["myapp"]
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[lib]
name = "myapp"
path = "src/lib.rs"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn helper() -> i32 { 42 }
pub fn other() -> i32 { 0 }
"#,
    );
    temp.file(
        "src/main.rs",
        r#"
use myapp::helper;
fn main() { println!("{}", helper()); }
"#,
    );
    temp.file(
        "tests/unit.rs",
        r#"
#[test]
fn test_other() { assert_eq!(myapp::other(), 0); }
"#,
    );
    temp.file(
        "tests/cli/run.bats",
        r#"
@test "runs main" {
    run ./target/debug/myapp
    [ "$output" = "42" ]
}
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Coverage should be merged from both suites
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    let rust_coverage = coverage.unwrap().get("rust").and_then(|v| v.as_f64());

    // Both helper() and other() should be covered (100%)
    // because unit tests cover other() and CLI tests cover helper() via main
    assert!(rust_coverage.unwrap() > 90.0);
}

/// Spec: docs/specs/11-test-runners.md#no-coverage
///
/// > For suites that only contribute timing: targets = []
#[test]
#[ignore = "TODO: Phase 940 - Requires runner integration"]
fn suite_with_empty_targets_skips_coverage() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/smoke/"
targets = []  # Explicit: timing only
"#,
    );
    temp.file(
        "tests/smoke/basic.bats",
        r#"
@test "smoke test" { true; }
"#,
    );

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should have timing but no coverage
    assert!(metrics.get("total_ms").is_some());
    assert!(metrics.get("coverage").is_none());
}

// =============================================================================
// JAVASCRIPT COVERAGE
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > Jest runner provides implicit JavaScript coverage via --coverage.
#[test]
#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn jest_runner_collects_javascript_coverage() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "jest"
"#,
    );
    temp.file(
        "package.json",
        r#"{
  "name": "test-project",
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#,
    );
    temp.file(
        "src/lib.js",
        r#"
export function covered() { return 42; }
export function uncovered() { return 0; }
"#,
    );
    temp.file(
        "tests/lib.test.js",
        r#"
const { covered } = require('../src/lib');
test('covered function', () => { expect(covered()).toBe(42); });
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report JavaScript coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let js_coverage = coverage.unwrap().get("javascript").and_then(|v| v.as_f64());
    assert!(js_coverage.is_some());
    // Coverage should be ~50% (one function covered, one not)
    let pct = js_coverage.unwrap();
    assert!(
        pct > 40.0 && pct < 60.0,
        "Expected ~50% coverage, got {}",
        pct
    );
}

/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > Vitest runner provides implicit JavaScript/TypeScript coverage.
#[test]
#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn vitest_runner_collects_javascript_coverage() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "vitest"
"#,
    );
    temp.file(
        "package.json",
        r#"{
  "name": "test-project",
  "devDependencies": {
    "vitest": "^2.0.0"
  }
}"#,
    );
    temp.file("vitest.config.ts", "export default {}");
    temp.file(
        "src/lib.ts",
        r#"
export function covered(): number { return 42; }
export function uncovered(): number { return 0; }
"#,
    );
    temp.file(
        "tests/lib.test.ts",
        r#"
import { covered } from '../src/lib';
import { test, expect } from 'vitest';

test('covered function', () => { expect(covered()).toBe(42); });
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report JavaScript coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let js_coverage = coverage.unwrap().get("javascript").and_then(|v| v.as_f64());
    assert!(js_coverage.is_some());
    let pct = js_coverage.unwrap();
    assert!(
        pct > 40.0 && pct < 60.0,
        "Expected ~50% coverage, got {}",
        pct
    );
}

/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > Bun runner provides implicit JavaScript/TypeScript coverage.
#[test]
#[ignore = "TODO: Phase 4981 - Requires bun install"]
fn bun_runner_collects_javascript_coverage() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bun"
"#,
    );
    temp.file(
        "package.json",
        r#"{
  "name": "test-project"
}"#,
    );
    temp.file(
        "src/lib.ts",
        r#"
export function covered(): number { return 42; }
export function uncovered(): number { return 0; }
"#,
    );
    temp.file(
        "tests/lib.test.ts",
        r#"
import { covered } from '../src/lib';
import { test, expect } from 'bun:test';

test('covered function', () => { expect(covered()).toBe(42); });
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Should report JavaScript coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let js_coverage = coverage.unwrap().get("javascript").and_then(|v| v.as_f64());
    assert!(js_coverage.is_some());
    let pct = js_coverage.unwrap();
    assert!(
        pct > 40.0 && pct < 60.0,
        "Expected ~50% coverage, got {}",
        pct
    );
}

/// Spec: docs/specs/11-test-runners.md#aggregation
///
/// > JavaScript coverage is merged across suites.
#[test]
#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn multiple_js_suite_coverages_merged() {
    let temp = Project::empty();
    temp.config(
        r#"
# Suite 1: Unit tests with Jest
[[check.tests.suite]]
runner = "jest"
path = "tests/unit/"

# Suite 2: Integration tests with Jest
[[check.tests.suite]]
runner = "jest"
path = "tests/integration/"
"#,
    );
    temp.file(
        "package.json",
        r#"{
  "name": "test-project",
  "devDependencies": {
    "jest": "^29.0.0"
  }
}"#,
    );
    temp.file(
        "src/math.js",
        r#"
export function add(a, b) { return a + b; }
export function subtract(a, b) { return a - b; }
"#,
    );
    temp.file(
        "tests/unit/add.test.js",
        r#"
const { add } = require('../../src/math');
test('add', () => { expect(add(1, 2)).toBe(3); });
"#,
    );
    temp.file(
        "tests/integration/subtract.test.js",
        r#"
const { subtract } = require('../../src/math');
test('subtract', () => { expect(subtract(3, 1)).toBe(2); });
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Coverage should be merged from both suites
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    let js_coverage = coverage.unwrap().get("javascript").and_then(|v| v.as_f64());

    // Both add() and subtract() should be covered (~100%)
    assert!(
        js_coverage.unwrap() > 90.0,
        "Expected ~100% merged coverage"
    );
}
