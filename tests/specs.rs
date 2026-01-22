//! Behavioral specifications for quench CLI.
//!
//! These tests are black-box: they invoke the CLI binary and verify
//! stdout, stderr, and exit codes. See CLAUDE.md for conventions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "specs/prelude.rs"]
mod prelude;

#[path = "specs/file_walking.rs"]
mod file_walking;

use prelude::*;

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
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
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
    let dir = tempfile::tempdir().unwrap();
    quench_cmd()
        .arg("init")
        .current_dir(dir.path())
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
/// > -C <FILE> specifies config file (short for --config)
#[test]
fn short_config_flag_works() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "version = 1\n").unwrap();

    quench_cmd()
        .args(["-C", config_path.to_str().unwrap(), "check"])
        .current_dir(dir.path())
        .assert()
        .success();
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
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{"));
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > -o json produces JSON output
#[test]
fn check_output_json_format() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
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

// =============================================================================
// CONFIG WARNING SPECS
// =============================================================================

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown keys are warnings (forward compatibility)
#[test]
fn unknown_config_key_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        "version = 1\nunknown_key = true\n",
    )
    .unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success() // Should not fail
        .stderr(predicates::str::contains("unknown").or(predicates::str::contains("unrecognized")));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown nested keys are warnings
#[test]
fn unknown_nested_config_key_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.unknown]
field = "value"
"#,
    )
    .unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("unknown").or(predicates::str::contains("unrecognized")));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Valid config produces no warnings
#[test]
fn valid_config_no_warnings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::is_empty().or(predicates::str::contains("warning").not()));
}

// =============================================================================
// ENVIRONMENT VARIABLE SPECS
// =============================================================================

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_NO_COLOR=1 disables color output
#[test]
fn env_no_color_disables_color() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create a file that would trigger a violation with colored output
    std::fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    let output = quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI escape codes start with \x1b[
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI codes"
    );
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_CONFIG sets config file location
#[test]
fn env_config_sets_path() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom-config.toml");
    std::fs::write(&config_path, "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_CONFIG", config_path.to_str().unwrap())
        .assert()
        .success();
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG enables debug logging to stderr
#[test]
fn env_log_enables_debug() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::contains("DEBUG").or(predicates::str::contains("debug")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG=trace enables trace logging
#[test]
fn env_log_trace_level() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_LOG", "trace")
        .assert()
        .success()
        .stderr(predicates::str::contains("TRACE").or(predicates::str::contains("trace")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > Unknown QUENCH_* environment variables are silently ignored
#[test]
fn env_unknown_vars_ignored() {
    quench_cmd()
        .arg("--help")
        .env("QUENCH_UNKNOWN_VAR", "some_value")
        .assert()
        .success(); // Should not error on unknown env vars
}

// =============================================================================
// OUTPUT SNAPSHOT SPECS
// =============================================================================

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
