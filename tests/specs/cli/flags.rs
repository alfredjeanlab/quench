// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for CLI flags.
//!
//! Tests that quench correctly handles:
//! - Global flags (-h, -V, -C)
//! - Check command flags (-o, --output)
//! - Unknown flags (exit code 2)
//!
//! Reference: docs/specs/01-cli.md#global-flags

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// GLOBAL FLAG SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > -h shows help (short for --help)
#[test]
fn short_help_flag_works() {
    quench_cmd()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > -V shows version (short for --version)
#[test]
fn short_version_flag_works() {
    quench_cmd()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > Unknown global flags produce error, not silently ignored
#[test]
fn unknown_global_flag_fails() {
    quench_cmd()
        .arg("-x")
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > Unknown long flags produce error
#[test]
fn unknown_long_flag_fails() {
    quench_cmd()
        .arg("--unknown-flag")
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}

// =============================================================================
// CHECK COMMAND FLAG SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > -o <FMT> sets output format (short for --output)
#[test]
fn check_short_output_flag_works() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{"));
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > -o json produces JSON output
#[test]
fn check_output_json_format() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
    assert!(
        json.get("passed").is_some(),
        "JSON should have 'passed' field"
    );
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Unknown flags to check command produce error
#[test]
fn check_unknown_flag_fails() {
    quench_cmd()
        .args(["check", "-x"])
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Unknown long flags to check command produce error
#[test]
fn check_unknown_long_flag_fails() {
    quench_cmd()
        .args(["check", "--unknown-option"])
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}
