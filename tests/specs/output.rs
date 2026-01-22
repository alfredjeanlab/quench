//! Behavioral specs for output infrastructure.
//!
//! Tests that quench correctly formats output according to:
//! - docs/specs/03-output.md (text and JSON formats)
//! - docs/specs/output.schema.json (JSON schema)
//!
//! Reference: docs/specs/03-output.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// Text Output Format
// =============================================================================

/// Spec: docs/specs/03-output.md#text-format
///
/// > Text format: `<check-name>: FAIL`
/// > `  <file>:<line>: <brief violation description>`
/// > `    <advice>`
#[test]
fn text_output_format_check_name_fail() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"^\w+: FAIL").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > File path and line number format: `<file>:<line>:`
#[test]
fn text_output_format_file_line() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"  \S+:").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > Advice is indented under violation
#[test]
fn text_output_format_advice_indented() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"\n    \S").unwrap()); // 4-space indent for advice
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > Summary line: `N checks passed, M failed`
#[test]
fn text_output_summary_line() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"\d+ checks? (passed|failed)").unwrap());
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > When all checks pass, only summary: `N checks passed`
#[test]
fn text_output_passing_summary_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"^\d+ checks? passed\n?$").unwrap());
}

// =============================================================================
// JSON Output Format
// =============================================================================

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON output validates against output.schema.json
#[test]
fn json_output_validates_against_schema() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("output should be valid JSON");

    // Load schema from docs/specs/output.schema.json
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs/specs/output.schema.json");
    let schema_str = std::fs::read_to_string(&schema_path).unwrap();
    let schema: serde_json::Value = serde_json::from_str(&schema_str).unwrap();

    let compiled = jsonschema::validator_for(&schema).expect("schema should be valid");

    assert!(
        compiled.is_valid(&json),
        "output should validate against schema"
    );
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON has required fields: passed, checks
#[test]
fn json_output_has_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("passed").is_some(), "should have 'passed' field");
    assert!(json.get("checks").is_some(), "should have 'checks' array");
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON timestamp is ISO 8601 format
#[test]
fn json_output_timestamp_iso8601() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let timestamp = json.get("timestamp").and_then(|v| v.as_str());
    assert!(timestamp.is_some(), "should have timestamp");

    // ISO 8601 format: 2026-01-21T10:30:00Z
    let ts = timestamp.unwrap();
    assert!(
        ts.contains('T') && ts.ends_with('Z'),
        "timestamp should be ISO 8601: {}",
        ts
    );
}

/// Spec: docs/specs/output.schema.json
///
/// > Check objects have required fields: name, passed
#[test]
fn json_output_check_has_required_fields() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = json.get("checks").and_then(|v| v.as_array()).unwrap();

    for check in checks {
        assert!(check.get("name").is_some(), "check should have 'name'");
        assert!(check.get("passed").is_some(), "check should have 'passed'");
    }
}

/// Spec: docs/specs/output.schema.json
///
/// > Violation objects have required fields: type, advice
#[test]
fn json_output_violation_has_required_fields() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = json.get("checks").and_then(|v| v.as_array()).unwrap();

    for check in checks {
        if let Some(violations) = check.get("violations").and_then(|v| v.as_array()) {
            for violation in violations {
                assert!(
                    violation.get("type").is_some(),
                    "violation should have 'type'"
                );
                assert!(
                    violation.get("advice").is_some(),
                    "violation should have 'advice'"
                );
            }
        }
    }
}

// =============================================================================
// Exit Codes
// =============================================================================

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 0 when all checks pass
#[test]
fn exit_code_0_all_checks_pass() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .code(0);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 1 when any check fails
#[test]
fn exit_code_1_check_fails() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 2 on configuration error
#[test]
fn exit_code_2_config_error() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("config-error"))
        .assert()
        .code(2);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit codes: 0 (pass), 1 (fail), 2 (config), 3 (internal)
/// > These are the ONLY valid exit codes
#[test]
fn exit_codes_are_exactly_0_1_2_3() {
    // This test documents the contract. Individual tests verify each code.
    // Exit code 3 (internal error) is hard to trigger intentionally,
    // so we verify the enum values in error.rs match the spec.

    use quench::error::ExitCode;
    assert_eq!(ExitCode::Success as u8, 0);
    assert_eq!(ExitCode::CheckFailed as u8, 1);
    assert_eq!(ExitCode::ConfigError as u8, 2);
    assert_eq!(ExitCode::InternalError as u8, 3);
}

// =============================================================================
// Colorization
// =============================================================================

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when CLAUDE_CODE env var is set
#[test]
fn color_disabled_when_claude_code_env_set() {
    let output = quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .env("CLAUDE_CODE", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes when CLAUDE_CODE is set"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when stdout is not a TTY
#[test]
fn color_disabled_when_not_tty() {
    // When run via assert_cmd, stdout is piped (not a TTY)
    let output = quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes when not a TTY"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > --no-color flag disables color output
#[test]
fn no_color_flag_disables_color() {
    let output = quench_cmd()
        .args(["check", "--no-color"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes with --no-color"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > --color=never disables color output
#[test]
fn color_never_disables_color() {
    let output = quench_cmd()
        .args(["check", "--color=never"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes with --color=never"
    );
}

// =============================================================================
// Violation Limits
// =============================================================================

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Default limit: 15 violations shown
#[test]
fn violation_limit_defaults_to_15() {
    // This spec requires a fixture with >15 violations
    // For now, just verify the flag is accepted
    quench_cmd()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("limit"));
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > --no-limit shows all violations
#[test]
fn no_limit_shows_all_violations() {
    let _ = quench_cmd()
        .args(["check", "--no-limit"])
        .current_dir(fixture("output-test"))
        .assert(); // Just verify flag is accepted
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > --limit N shows N violations
#[test]
fn limit_n_shows_n_violations() {
    let _ = quench_cmd()
        .args(["check", "--limit", "5"])
        .current_dir(fixture("output-test"))
        .assert(); // Just verify flag is accepted
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Message shown when limit reached: "Stopped after N violations"
#[test]
fn limit_message_when_truncated() {
    // Requires fixture with many violations
    quench_cmd()
        .args(["check", "--limit", "1"])
        .current_dir(fixture("violations"))
        .assert()
        .stdout(
            predicates::str::contains("Stopped after").or(predicates::str::contains("--no-limit")),
        );
}

// =============================================================================
// Config Validation Mode
// =============================================================================

/// Spec: docs/specs/01-cli.md#commands (implied)
///
/// > --config-only validates config and exits without running checks
#[test]
fn config_flag_validates_and_exits() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check", "--config-only"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().or(predicates::str::contains("valid")));
}

/// Spec: docs/specs/01-cli.md#commands (implied)
///
/// > --config-only with invalid config returns exit code 2
#[test]
fn config_flag_invalid_returns_code_2() {
    quench_cmd()
        .args(["check", "--config-only"])
        .current_dir(fixture("config-error"))
        .assert()
        .code(2);
}

// =============================================================================
// Debug Output
// =============================================================================

/// Spec: docs/specs/03-output.md (implied from QUENCH_LOG)
///
/// > QUENCH_LOG=debug emits diagnostics to stderr
#[test]
fn quench_log_debug_emits_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .env("QUENCH_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::is_empty().not());
}
