//! Behavioral specifications for quench CLI.
//!
//! These tests are black-box: they invoke the CLI binary and verify
//! stdout, stderr, and exit codes. See CLAUDE.md for conventions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "specs/prelude.rs"]
mod prelude;

use prelude::*;

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
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_command_exists() {
    quench_cmd().arg("check").assert().success();
}

/// Spec: docs/specs/03-output.md#text-output
///
/// > Text output format snapshot
#[test]
#[ignore = "TODO: Phase 030 - Output infrastructure"]
fn check_output_format_snapshot() {
    let output = quench_cmd()
        .args(["check", "--cloc"])
        .current_dir(prelude::fixture("violations"))
        .output()
        .expect("command should run");

    insta::assert_snapshot!(
        String::from_utf8_lossy(&output.stdout),
        @"" // Inline snapshot, will be filled on first run
    );
}
