// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for CLI commands.
//!
//! Tests that quench correctly handles:
//! - Bare invocation (shows help)
//! - help, version, check, report, init commands
//! - Unknown commands (exit code 2)
//!
//! Reference: docs/specs/01-cli.md#commands

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// COMMAND SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench (bare invocation) shows help
#[test]
fn bare_invocation_shows_help() {
    quench_cmd()
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 0 when invoked with --help
#[test]
fn help_exits_successfully() {
    quench_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("quench"));
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench help shows help
#[test]
fn help_command_shows_help() {
    quench_cmd()
        .arg("help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 0 when invoked with --version
#[test]
fn version_exits_successfully() {
    quench_cmd().arg("--version").assert().success();
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench check runs quality checks
#[test]
fn check_command_exists() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench report generates reports
#[test]
fn report_command_exists() {
    quench_cmd().arg("report").assert().success();
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench init initializes configuration
#[test]
fn init_command_exists() {
    let temp = Project::empty();
    quench_cmd()
        .arg("init")
        .current_dir(temp.path())
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 2 for unknown commands
#[test]
fn unknown_command_fails() {
    quench_cmd()
        .arg("unknown")
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unrecognized|unknown)").unwrap());
}
