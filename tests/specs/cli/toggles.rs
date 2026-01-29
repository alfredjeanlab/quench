// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the check framework.
//!
//! Tests that quench correctly handles:
//! - Check toggle flags (--cloc, --no-cloc, etc.)
//! - Check name validation
//! - Multiple check flag combinations
//! - Error isolation between checks
//! - Skipped check reporting
//!
//! Reference: docs/specs/01-cli.md#check-toggles

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use yare::parameterized;

// =============================================================================
// Check Names
// =============================================================================

/// Spec: docs/specs/00-overview.md#built-in-checks
///
/// > Built-in checks: cloc, escapes, agents, docs, tests, git, build, license
#[test]
fn check_names_are_exactly_8_known_checks() {
    let temp = default_project();
    let result = cli().pwd(temp.path()).json().passes();
    let checks = result.checks();

    let names: Vec<&str> = checks
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();

    // All 8 checks should be present
    assert!(names.contains(&"cloc"), "should have cloc check");
    assert!(names.contains(&"escapes"), "should have escapes check");
    assert!(names.contains(&"agents"), "should have agents check");
    assert!(names.contains(&"docs"), "should have docs check");
    assert!(names.contains(&"tests"), "should have tests check");
    assert!(names.contains(&"git"), "should have git check");
    assert!(names.contains(&"build"), "should have build check");
    assert!(names.contains(&"license"), "should have license check");

    // No other checks should be present
    assert_eq!(names.len(), 8, "should have exactly 8 checks");
}

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > Check toggles appear in help: --[no-]cloc, --[no-]escapes, etc.
#[test]
fn check_toggles_shown_in_help() {
    // Uses quench_cmd() directly since this tests --help, not check execution
    quench_cmd()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--[no-]cloc"))
        .stdout(predicates::str::contains("--[no-]escapes"))
        .stdout(predicates::str::contains("--[no-]agents"))
        .stdout(predicates::str::contains("--[no-]docs"))
        .stdout(predicates::str::contains("--[no-]tests"))
        .stdout(predicates::str::contains("--[no-]git"))
        .stdout(predicates::str::contains("--[no-]build"))
        .stdout(predicates::str::contains("--[no-]license"));
}

// =============================================================================
// Enable Flags (--<check>)
// =============================================================================

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > --<check>: Only run that check (implies --no-* for others)
#[parameterized(
    cloc = { "cloc" },
    escapes = { "escapes" },
    agents = { "agents" },
    docs = { "docs" },
    tests = { "tests" },
    git = { "git" },
    build = { "build" },
    license = { "license" },
)]
fn enable_flag_runs_only_that_check(check_name: &str) {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&[&format!("--{}", check_name)])
        .json()
        .passes();
    let names = check_names(result.value());

    assert_eq!(names.len(), 1, "only one check should run");
    assert_eq!(names[0], check_name);
}

// =============================================================================
// Disable Flags (--no-<check>)
// =============================================================================

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > --no-<check>: Skip that check, run all others
#[parameterized(
    cloc = { "cloc" },
    escapes = { "escapes" },
    agents = { "agents" },
    docs = { "docs" },
    tests = { "tests" },
    git = { "git" },
    build = { "build" },
    license = { "license" },
)]
fn disable_flag_skips_that_check(check_name: &str) {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&[&format!("--no-{}", check_name)])
        .json()
        .passes();
    let names = check_names(result.value());

    assert!(
        !names.contains(&check_name),
        "{} should not be present",
        check_name
    );
    assert_eq!(
        names.len(),
        7,
        "7 checks should run (all except {})",
        check_name
    );
}

// =============================================================================
// Flag Combinations
// =============================================================================

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > Multiple enable flags combine: --cloc --escapes runs both checks
#[test]
fn multiple_enable_flags_run_multiple_checks() {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&["--cloc", "--escapes"])
        .json()
        .passes();
    let names = check_names(result.value());

    assert_eq!(names.len(), 2, "two checks should run");
    assert!(names.contains(&"cloc"), "cloc should be present");
    assert!(names.contains(&"escapes"), "escapes should be present");
}

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > Multiple disable flags combine: --no-docs --no-tests skips both
#[test]
fn multiple_disable_flags_skip_multiple_checks() {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&["--no-docs", "--no-tests"])
        .json()
        .passes();
    let names = check_names(result.value());

    assert!(!names.contains(&"docs"), "docs should not be present");
    assert!(!names.contains(&"tests"), "tests should not be present");
    assert_eq!(names.len(), 6, "6 checks should run");
}

/// Spec: docs/specs/01-cli.md#examples
///
/// > quench check --no-cloc --no-escapes: Skip multiple checks
#[test]
fn no_cloc_no_escapes_skips_both() {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&["--no-cloc", "--no-escapes"])
        .json()
        .passes();
    let names = check_names(result.value());

    assert!(!names.contains(&"cloc"), "cloc should not be present");
    assert!(!names.contains(&"escapes"), "escapes should not be present");
}

/// Spec: docs/specs/01-cli.md#check-toggles (edge case)
///
/// > All checks can be disabled except one
#[test]
fn all_checks_disabled_except_one() {
    let temp = default_project();
    let result = cli()
        .pwd(temp.path())
        .args(&[
            "--no-cloc",
            "--no-escapes",
            "--no-agents",
            "--no-docs",
            "--no-tests",
            "--no-git",
            "--no-build",
            // license is the only one NOT disabled
        ])
        .json()
        .passes();

    assert_eq!(result.checks().len(), 1, "only one check should run");
    assert_eq!(
        result.checks()[0].get("name").and_then(|n| n.as_str()),
        Some("license"),
        "only license check should run"
    );
}

// =============================================================================
// Error Isolation
// =============================================================================

/// Spec: docs/specs/00-overview.md (implied)
///
/// > Check failure doesn't prevent other checks from running
#[test]
fn check_failure_doesnt_block_other_checks() {
    // Use fixture that triggers cloc failure (oversized file)
    let result = cli().on("check-framework").json().fails();
    let checks = result.checks();

    // All 8 checks should have run, even though cloc failed
    assert_eq!(checks.len(), 8, "all checks should have run");

    // Find cloc check - it should have failed
    let cloc = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc"))
        .expect("cloc check should exist");
    assert_eq!(
        cloc.get("passed").and_then(|p| p.as_bool()),
        Some(false),
        "cloc should have failed"
    );

    // Other checks should have completed (may pass or fail, but not skipped)
    let other_checks: Vec<_> = checks
        .iter()
        .filter(|c| c.get("name").and_then(|n| n.as_str()) != Some("cloc"))
        .collect();

    for check in other_checks {
        assert!(
            check.get("skipped").and_then(|s| s.as_bool()) != Some(true),
            "check {} should not be skipped due to cloc failure",
            check
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
        );
    }
}

// =============================================================================
// Skipped Checks
// =============================================================================

/// Spec: docs/specs/03-output.md (implied)
///
/// > Skipped check shows error message but continues with other checks
#[test]
fn skipped_check_shows_error_but_continues() {
    // Don't initialize git - git check should skip
    let temp = default_project();
    let result = cli().pwd(temp.path()).json().passes();
    let checks = result.checks();

    // Find git check
    let git = checks
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("git"));

    if let Some(git_check) = git {
        // Git check should be skipped with error message
        assert_eq!(
            git_check.get("skipped").and_then(|s| s.as_bool()),
            Some(true),
            "git check should be skipped"
        );
        assert!(
            git_check.get("error").is_some(),
            "skipped check should have error message"
        );
    }

    // Other checks should still have run
    let non_git_checks = checks
        .iter()
        .filter(|c| c.get("name").and_then(|n| n.as_str()) != Some("git"));
    assert!(non_git_checks.count() >= 7, "other checks should have run");
}

/// Spec: docs/specs/03-output.md#text-format (implied)
///
/// > Skipped check shows in text output with reason
/// > Format: `<check>: SKIP` followed by reason on next line
#[test]
fn skipped_check_text_output_shows_reason() {
    // Don't initialize git - git check should skip
    let temp = default_project();
    // Format: "git: SKIP" on its own line (matching FAIL format)
    cli()
        .pwd(temp.path())
        .passes()
        .stdout_has(predicates::str::is_match(r"(?m)^git: SKIP$").unwrap());
}

/// Spec: docs/specs/output.schema.json
///
/// > Skipped check has `skipped: true` and `error` field in JSON
#[test]
fn skipped_check_json_has_required_fields() {
    let temp = default_project();
    let result = cli().pwd(temp.path()).json().passes();

    // Find any skipped check
    let skipped: Vec<_> = result
        .checks()
        .iter()
        .filter(|c| c.get("skipped").and_then(|s| s.as_bool()) == Some(true))
        .collect();

    for check in skipped {
        assert!(
            check.get("error").is_some(),
            "skipped check should have 'error' field"
        );
        assert_eq!(
            check.get("passed").and_then(|p| p.as_bool()),
            Some(false),
            "skipped check should have passed=false"
        );
    }
}
