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
    // "<check-name>: FAIL" at start of line
    cli()
        .on("output-test")
        .exits(1)
        .stdout_has(predicates::str::is_match(r"(?m)^[a-z][a-z0-9_-]*: FAIL$").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > File path and line number format: `  <file>:<line>: <description>` or `  <file>: <description>`
#[test]
fn text_output_format_file_line() {
    // 2-space indent, file path, colon, optional line number, colon, description
    cli()
        .on("output-test")
        .exits(1)
        .stdout_has(predicates::str::is_match(r"(?m)^  [a-zA-Z0-9_./-]+(:\d+)?: .+$").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > Advice is indented under violation (4-space indent)
#[test]
fn text_output_format_advice_indented() {
    // 4-space indent followed by advice
    cli()
        .on("output-test")
        .exits(1)
        .stdout_has(predicates::str::is_match(r#"(?m)^    [A-Z"].+$"#).unwrap());
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > Summary lists checks by status: `PASS: check1, check2` and `FAIL: check3`
/// > Stub checks (not yet implemented) are omitted from the summary entirely.
#[test]
fn text_output_summary_lists_checks_by_status() {
    // When cloc fails and all other checks are stubs, only FAIL line appears
    cli()
        .on("output-test")
        .exits(1)
        .stdout_has(predicates::str::is_match(r"(?m)^FAIL: [a-z, ]+$").unwrap());
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > When all checks pass, only PASS line: `PASS: check1, check2, ...`
/// > Stub checks (not yet implemented) are omitted from the summary entirely.
#[test]
fn text_output_passing_summary_only() {
    let temp = default_project();
    // Only non-stub checks appear; currently cloc, escapes, agents, docs, tests, license are implemented
    cli()
        .pwd(temp.path())
        .args(&["--no-git"])
        .passes()
        .stdout_has("PASS: cloc, escapes, agents, docs, tests, license\n");
}

// =============================================================================
// JSON Output Format
// =============================================================================

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON output validates against output.schema.json
#[test]
fn json_output_validates_against_schema() {
    let result = cli().on("output-test").json().fails();
    let json = result.value();

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
        compiled.is_valid(json),
        "output should validate against schema"
    );
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON has required fields: passed, checks
#[test]
fn json_output_has_required_fields() {
    let temp = default_project();
    let result = cli().pwd(temp.path()).args(&["--no-git"]).json().passes();
    let json = result.value();
    assert!(json.get("passed").is_some(), "should have 'passed' field");
    assert!(json.get("checks").is_some(), "should have 'checks' array");
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON timestamp is ISO 8601 format
#[test]
fn json_output_timestamp_iso8601() {
    let temp = default_project();
    let result = cli().pwd(temp.path()).args(&["--no-git"]).json().passes();
    let json = result.value();

    let ts = json
        .get("timestamp")
        .and_then(|v| v.as_str())
        .expect("should have timestamp");

    // ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ (e.g., 2026-01-21T10:30:00Z)
    assert_eq!(ts.len(), 20, "timestamp should be exactly 20 chars: {}", ts);
    assert!(
        predicates::str::is_match(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
            .unwrap()
            .eval(ts),
        "timestamp should match ISO 8601: {}",
        ts
    );
}

/// Spec: docs/specs/output.schema.json
///
/// > Check objects have required fields: name, passed
#[test]
fn json_output_check_has_required_fields() {
    let result = cli().on("output-test").json().fails();
    for check in result.checks() {
        assert!(check.get("name").is_some(), "check should have 'name'");
        assert!(check.get("passed").is_some(), "check should have 'passed'");
    }
}

/// Spec: docs/specs/output.schema.json
///
/// > Violation objects have required fields: type, advice
#[test]
fn json_output_violation_has_required_fields() {
    let result = cli().on("output-test").json().fails();
    for check in result.checks() {
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
    let temp = default_project();
    // Use --no-git to avoid skip in non-git directory
    cli().pwd(temp.path()).args(&["--no-git"]).passes();
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 1 when any check fails
#[test]
fn exit_code_1_check_fails() {
    cli().on("output-test").exits(1);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 2 on configuration error
#[test]
fn exit_code_2_config_error() {
    cli().on("config-error").exits(2);
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
    // No ANSI escape codes in output
    cli()
        .on("output-test")
        .env("CLAUDE_CODE", "1")
        .exits(1)
        .stdout_lacks("\x1b[");
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when stdout is not a TTY
#[test]
fn color_disabled_when_not_tty() {
    // When run via assert_cmd, stdout is piped (not a TTY)
    // No ANSI escape codes in output
    cli().on("output-test").exits(1).stdout_lacks("\x1b[");
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when NO_COLOR env var is set
#[test]
fn no_color_env_disables_color() {
    // No ANSI escape codes in output
    cli()
        .on("output-test")
        .env("NO_COLOR", "1")
        .exits(1)
        .stdout_lacks("\x1b[");
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color enabled when COLOR env var is set (even without TTY)
#[test]
fn color_env_forces_color() {
    // Note: This test runs in a non-TTY environment (piped stdout)
    // COLOR should force color output regardless
    cli()
        .on("output-test")
        .env("COLOR", "1")
        .exits(1)
        .stdout_has("\x1b[");
}

// =============================================================================
// Violation Limits
// =============================================================================

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Default limit: 15 violations shown
#[test]
fn violation_limit_defaults_to_15() {
    // TODO: requires fixture with >15 violations
    // For now, just verify --help mentions limit
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
    // Verify flag is accepted
    cli().on("output-test").args(&["--no-limit"]).exits(1);
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > --limit N shows N violations
#[test]
fn limit_n_shows_n_violations() {
    // Verify flag is accepted
    cli().on("output-test").args(&["--limit", "5"]).exits(1);
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Message shown when limit reached: "Stopped after N violations. Use --no-limit to see all."
#[test]
fn limit_message_when_truncated() {
    cli()
        .on("violations")
        .args(&["--limit", "1"])
        .exits(1)
        .stdout_has(
            predicates::str::is_match(
                r"Stopped after \d+ violations?\. Use --no-limit to see all\.",
            )
            .unwrap(),
        );
}

// =============================================================================
// =============================================================================
// Debug Output
// =============================================================================

/// Spec: docs/specs/03-output.md (implied from QUENCH_LOG)
///
/// > QUENCH_LOG=debug emits diagnostics to stderr
#[test]
fn quench_log_debug_emits_diagnostics() {
    let temp = default_project();
    // stderr should not be empty when debug logging is enabled
    cli()
        .pwd(temp.path())
        .args(&["--no-git"])
        .env("QUENCH_LOG", "debug")
        .passes()
        .stderr_has(predicates::str::is_empty().not());
}

// =============================================================================
// Output Snapshot Specs
// =============================================================================

/// Spec: docs/specs/03-output.md#text-output
///
/// > Text output format exact match
#[test]
fn check_output_format_exact() {
    let output = quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .output()
        .expect("command should run");

    // Stub checks are omitted from summary per docs/specs/03-output.md#verbosity
    // Multi-line advice has trailing newline for readability
    let expected = "\
cloc: FAIL
  src/oversized.rs: file_too_large (lines: 15 vs 10)
    Can the code be made more concise?

    Look for repetitive patterns that could be extracted into helper functions
    or consider refactoring to be more unit testable.

    If not, split large source files into sibling modules or submodules in a folder,

    Avoid picking and removing individual lines to satisfy the linter,
    prefer properly refactoring out testable code blocks.

PASS: escapes, agents, docs, tests, git, license
FAIL: cloc
";

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "output format must match exactly"
    );
}

// =============================================================================
// Advice Deduplication
// =============================================================================

/// Spec: docs/specs/03-output.md#advice-deduplication
///
/// > Consecutive violations with identical advice only show advice once
#[test]
fn text_output_deduplicates_consecutive_identical_advice() {
    cli().on("dedup-advice").exits(1).stdout_eq(
        "cloc: FAIL
  src/file_c.rs: file_too_large (lines: 7 vs 5)
    Can the code be made more concise?

    Look for repetitive patterns that could be extracted into helper functions
    or consider refactoring to be more unit testable.

    If not, split large source files into sibling modules or submodules in a folder,

    Avoid picking and removing individual lines to satisfy the linter,
    prefer properly refactoring out testable code blocks.

  src/file_b.rs: file_too_large (lines: 7 vs 5)
  src/file_a.rs: file_too_large (lines: 7 vs 5)
PASS: escapes, agents, docs, tests, git, license
FAIL: cloc
",
    );
}

/// Spec: docs/specs/03-output.md#advice-deduplication
///
/// > JSON output is never deduplicated (preserves full machine-readable data)
#[test]
fn json_output_never_deduplicates_advice() {
    let result = cli().on("dedup-advice").json().fails();

    // Get cloc check
    let cloc_check = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc"))
        .expect("should have cloc check");

    let violations = cloc_check
        .get("violations")
        .and_then(|v| v.as_array())
        .expect("should have violations array");

    // Should have exactly 3 violations (one per oversized file)
    assert_eq!(
        violations.len(),
        3,
        "should have 3 file_too_large violations"
    );

    // Every violation must have non-empty advice in JSON output
    for violation in violations {
        let advice = violation
            .get("advice")
            .and_then(|a| a.as_str())
            .expect("all violations in JSON should have advice field");

        assert!(
            !advice.is_empty(),
            "advice should not be empty in JSON output"
        );

        // Verify it's the expected multi-line advice
        assert!(
            advice.contains("Can the code be made more concise?"),
            "should have full advice text"
        );
    }
}
