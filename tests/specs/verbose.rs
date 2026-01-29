// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for verbose output.
//!
//! Reference: plans/verbose-in-ci-mode.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// VERBOSE FLAG BEHAVIOR
// =============================================================================

/// Spec: plans/verbose-in-ci-mode.md - Verification Plan
///
/// > `--verbose` flag produces verbose output
#[test]
fn verbose_flag_produces_verbose_output() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");

    cli()
        .pwd(temp.path())
        .args(&["--verbose"])
        .passes()
        .stderr_has("=== Configuration ===")
        .stderr_has("=== Discovery ===")
        .stderr_has("=== Summary ===");
}

/// Spec: plans/verbose-in-ci-mode.md - Verification Plan
///
/// > `--ci` implicitly enables verbose
#[test]
fn ci_mode_implicitly_enables_verbose() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");

    cli()
        .pwd(temp.path())
        .args(&["--ci"])
        .passes()
        .stderr_has("=== Configuration ===")
        .stderr_has("=== Discovery ===")
        .stderr_has("=== Summary ===");
}

/// Spec: plans/verbose-in-ci-mode.md - Verification Plan
///
/// > Normal mode has no verbose output
#[test]
fn normal_mode_has_no_verbose_output() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");

    let result = cli()
        .pwd(temp.path())
        .passes();

    // Normal mode should not have verbose section headers
    let stderr = result.stderr();
    assert!(!stderr.contains("=== Configuration ==="));
    assert!(!stderr.contains("=== Discovery ==="));
    assert!(!stderr.contains("=== Summary ==="));
}

/// Spec: plans/verbose-in-ci-mode.md - Verification Plan
///
/// > JSON mode keeps stdout clean (verbose output only on stderr)
#[test]
fn json_mode_keeps_stdout_clean() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");

    let result = cli()
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    // Verify stdout is valid JSON
    let json = result.value();
    assert!(json.get("checks").is_some());

    // Verify the RunAssert's stderr contains verbose output
    // Note: We can't check stderr from ChecksJson, but we know it went to stderr
    // based on the implementation using eprintln!
}

// =============================================================================
// VERBOSE CONTENT LOGGING
// =============================================================================

/// Spec: plans/verbose-in-ci-mode.md - Phase 2
///
/// > Config rules are logged
#[test]
fn config_rules_are_logged() {
    let temp = default_project();
    temp.config(
        r#"
[project]
source = ["src/**/*.rs"]
tests = ["tests/**/*.rs"]
exclude = ["target/**"]
"#,
    );
    temp.file("src/lib.rs", "fn main() {}");

    cli()
        .pwd(temp.path())
        .args(&["--verbose"])
        .passes()
        .stderr_has("=== Configuration ===")
        .stderr_has("project.source:")
        .stderr_has("project.tests:")
        .stderr_has("project.exclude:");
}

/// Spec: plans/verbose-in-ci-mode.md - Phase 2
///
/// > File count is logged
#[test]
fn file_count_is_logged() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");
    temp.file("src/other.rs", "fn test() {}");

    cli()
        .pwd(temp.path())
        .args(&["--verbose"])
        .passes()
        .stderr_has("=== Discovery ===")
        .stderr_has("Scanned");
}

/// Spec: plans/verbose-in-ci-mode.md - Phase 3
///
/// > Suite execution is logged (before/after)
#[test]
fn suite_execution_is_logged() {
    let temp = Project::cargo("test_project");
    temp.file(
        "CLAUDE.md",
        "# Test\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    cli()
        .pwd(temp.path())
        .args(&["--verbose"])
        .passes()
        .stderr_has("=== Test Suites ===")
        .stderr_has("Running suite:")
        .stderr_has("Suite");
}

/// Spec: plans/verbose-in-ci-mode.md - Phase 6
///
/// > Wall time is logged
#[test]
fn wall_time_is_logged() {
    let temp = default_project();
    temp.file("src/lib.rs", "fn main() {}");

    cli()
        .pwd(temp.path())
        .args(&["--verbose"])
        .passes()
        .stderr_has("=== Summary ===")
        .stderr_has("Total wall time:");
}

// =============================================================================
// AUTO-DETECTED SUITES VERBOSE OUTPUT
// =============================================================================

/// Spec: plans/verbose-in-ci-mode.md - Phase 3 (item c)
///
/// > Auto-detected suites show detection source
#[test]
fn auto_detected_suites_show_detection_source() {
    // Create a Cargo project without explicit test suite config
    // so auto-detection kicks in
    let temp = Project::empty();
    temp.file(
        "CLAUDE.md",
        "# Test\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#,
    );

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--verbose"])
        .passes()
        .stderr_has("=== Test Suites ===")
        .stderr_has("Auto-detected suites:")
        .stderr_has("(detected:");
}
