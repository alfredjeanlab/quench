//! Behavioral specs for consolidated --[no-] help formatting.
//!
//! Tests that negatable flag pairs are displayed as --[no-]flag
//! instead of separate --flag and --no-flag lines.
//!
//! Reference: docs/specs/01-cli.md#help

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CONSOLIDATED HELP FORMAT SPECS
// =============================================================================

/// Spec: Help shows consolidated --[no-]flag format
///
/// > Negatable flags should be displayed as --[no-]flag
#[test]
fn check_help_shows_consolidated_color_flag() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show consolidated format
    assert!(
        stdout.contains("--[no-]color"),
        "Expected --[no-]color in help, got:\n{stdout}"
    );

    // Should NOT show separate --no-color line
    // (The combined --[no-]color is okay, but standalone --no-color is not)
    let lines: Vec<&str> = stdout.lines().collect();
    let no_color_lines: Vec<_> = lines
        .iter()
        .filter(|l| {
            let trimmed = l.trim_start();
            trimmed.starts_with("--no-color")
        })
        .collect();
    assert!(
        no_color_lines.is_empty(),
        "Found separate --no-color line(s): {:?}",
        no_color_lines
    );
}

/// Spec: Help shows consolidated --[no-]limit format with optional value
///
/// > --limit <N> and --no-limit should become --[no-]limit [N]
#[test]
fn check_help_shows_consolidated_limit_flag() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show consolidated format with optional value
    assert!(
        stdout.contains("--[no-]limit"),
        "Expected --[no-]limit in help, got:\n{stdout}"
    );
}

/// Spec: Help preserves standalone --no-cache (no --cache counterpart)
///
/// > Flags without a positive counterpart should remain as-is
#[test]
fn check_help_preserves_standalone_no_cache() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have --no-cache as standalone
    assert!(
        stdout.contains("--no-cache"),
        "Expected --no-cache in help, got:\n{stdout}"
    );

    // Should NOT be consolidated (no --cache counterpart exists)
    assert!(
        !stdout.contains("--[no-]cache"),
        "Should not consolidate --no-cache (no --cache exists)"
    );
}

/// Spec: Check toggle flags are consolidated
///
/// > All 9 check toggles should show as --[no-]<check>
#[test]
fn check_help_shows_consolidated_check_toggles() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    for check in [
        "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
    ] {
        let consolidated = format!("--[no-]{check}");
        assert!(
            stdout.contains(&consolidated),
            "Expected {consolidated} in help, got:\n{stdout}"
        );
    }
}

/// Spec: Report help shows consolidated check toggles
///
/// > Report command should also consolidate filter flags
#[test]
fn report_help_shows_consolidated_check_toggles() {
    let output = quench_cmd()
        .args(["report", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    for check in [
        "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
    ] {
        let consolidated = format!("--[no-]{check}");
        assert!(
            stdout.contains(&consolidated),
            "Expected {consolidated} in report help, got:\n{stdout}"
        );
    }
}

// =============================================================================
// HELP COMPLETENESS SPECS
// =============================================================================

/// Spec: Help contains all required elements
///
/// > Help should show usage, subcommands, and options sections
#[test]
fn main_help_contains_all_sections() {
    let output = quench_cmd()
        .arg("--help")
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Usage:"), "Missing Usage section");
    assert!(stdout.contains("Commands:"), "Missing Commands section");
    assert!(stdout.contains("Options:"), "Missing Options section");
    assert!(stdout.contains("check"), "Missing check command");
    assert!(stdout.contains("report"), "Missing report command");
    assert!(stdout.contains("init"), "Missing init command");
}

/// Spec: Check help contains all options
///
/// > Check --help should list all available options
#[test]
fn check_help_contains_all_options() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Core options
    assert!(stdout.contains("--config"), "Missing --config");
    assert!(stdout.contains("--output"), "Missing --output");
    assert!(stdout.contains("--verbose"), "Missing --verbose");
    assert!(stdout.contains("--fix"), "Missing --fix");
    assert!(stdout.contains("--dry-run"), "Missing --dry-run");
    assert!(stdout.contains("--ci"), "Missing --ci");
    assert!(stdout.contains("--timing"), "Missing --timing");
    assert!(stdout.contains("--base"), "Missing --base");
    assert!(stdout.contains("--staged"), "Missing --staged");
    assert!(stdout.contains("--max-depth"), "Missing --max-depth");
    assert!(stdout.contains("--config-only"), "Missing --config-only");
}

/// Spec: No duplicate flag entries after consolidation
///
/// > Each flag should appear exactly once in help output
#[test]
fn check_help_has_no_duplicate_entries() {
    let output = quench_cmd()
        .args(["check", "--help"])
        .output()
        .expect("command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count occurrences of each check toggle
    for check in [
        "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
    ] {
        // After consolidation, we should see exactly one line containing --[no-]<check>
        // and zero lines with standalone --<check> or --no-<check>
        let consolidated = format!("--[no-]{check}");
        let positive = format!("--{check}");
        let negative = format!("--no-{check}");

        let lines: Vec<&str> = stdout.lines().collect();

        // Count consolidated occurrences
        let consolidated_count = lines.iter().filter(|l| l.contains(&consolidated)).count();

        // Count standalone positive (excluding consolidated)
        let positive_count = lines
            .iter()
            .filter(|l| l.contains(&positive) && !l.contains(&consolidated))
            .count();

        // Count standalone negative (excluding consolidated)
        let negative_count = lines
            .iter()
            .filter(|l| l.contains(&negative) && !l.contains(&consolidated))
            .count();

        assert_eq!(
            consolidated_count, 1,
            "Expected exactly one {consolidated}, found {consolidated_count}"
        );
        assert_eq!(
            positive_count, 0,
            "Found standalone {positive} (should be consolidated)"
        );
        assert_eq!(
            negative_count, 0,
            "Found standalone {negative} (should be consolidated)"
        );
    }
}
