//! Behavioral specs for CI mode.
//!
//! Tests that quench correctly handles:
//! - --ci enables slow checks (build, license)
//! - --ci disables violation limit
//! - --ci auto-detects base branch
//! - --save writes metrics to file
//! - --save-notes writes metrics to git notes
//!
//! Reference: docs/specs/01-cli.md#scope-flags

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SLOW CHECKS ENABLED
// =============================================================================

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > CI mode (`--ci`) enables slow checks (build, license).
#[test]
#[ignore = "TODO: Requires fixture with build artifacts; currently both produce stub"]
fn ci_mode_enables_build_check() {
    let temp = default_project();

    // Without --ci, build check should return a stub
    let result = cli().pwd(temp.path()).args(&["--build"]).json().passes();
    let build = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_eq!(
        build.get("stub").and_then(|s| s.as_bool()),
        Some(true),
        "build check should be a stub without --ci"
    );

    // With --ci, build check should run (no stub field or stub=false)
    let result = cli()
        .pwd(temp.path())
        .args(&["--ci", "--build"])
        .json()
        .passes();
    let build = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_ne!(
        build.get("stub").and_then(|s| s.as_bool()),
        Some(true),
        "build check should run with --ci"
    );
}

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > CI mode (`--ci`) enables slow checks (build, license).
#[test]
fn ci_mode_enables_license_check() {
    let temp = default_project();

    // Without --ci, license check passes silently (CI-only check)
    let result = cli().pwd(temp.path()).args(&["--license"]).json().passes();
    let license = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("license"))
        .expect("license check should exist");
    assert_eq!(
        license.get("passed").and_then(|s| s.as_bool()),
        Some(true),
        "license check should pass without --ci (CI-only check skipped)"
    );
    // Verify it's not marked as a stub (real implementation)
    assert_ne!(
        license.get("stub").and_then(|s| s.as_bool()),
        Some(true),
        "license check is no longer a stub"
    );

    // With --ci, license check runs (but passes because no license config)
    let result = cli()
        .pwd(temp.path())
        .args(&["--ci", "--license"])
        .json()
        .passes();
    let license = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("license"))
        .expect("license check should exist");
    assert_eq!(
        license.get("passed").and_then(|s| s.as_bool()),
        Some(true),
        "license check should pass with --ci (no license config = disabled)"
    );
}

// =============================================================================
// VIOLATION LIMIT DISABLED
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > CI mode implicitly disables the violation limit.
#[test]
fn ci_mode_shows_all_violations() {
    // Use fixture with >15 violations
    // Skip git check since it adds violations beyond what fixture intends
    let result = cli()
        .on("ci-mode")
        .args(&["--ci", "--no-git"])
        .json()
        .fails();

    // Get total violations across all checks
    let total_violations: usize = result
        .checks()
        .iter()
        .filter_map(|c| c.get("violations").and_then(|v| v.as_array()))
        .map(|v| v.len())
        .sum();

    assert!(
        total_violations > 15,
        "CI mode should show all violations (got {})",
        total_violations
    );
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Default limit is 15 violations (without --ci or --no-limit).
#[test]
fn default_mode_limits_violations() {
    // Use fixture with >15 violations
    // Skip git check since it adds violations without respecting the limit
    let result = cli().on("ci-mode").args(&["--no-git"]).json().fails();

    // Violations should be capped at 15
    let total_violations: usize = result
        .checks()
        .iter()
        .filter_map(|c| c.get("violations").and_then(|v| v.as_array()))
        .map(|v| v.len())
        .sum();

    assert!(
        total_violations <= 15,
        "default mode should limit to 15 violations (got {})",
        total_violations
    );
}

// =============================================================================
// BASE BRANCH DETECTION
// =============================================================================

/// Spec: docs/specs/01-cli.md#scope-flags
///
/// > --ci auto-detects base branch (main > master > develop)
#[test]
fn ci_mode_auto_detects_main_branch() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    // Create a feature branch with changes
    git_branch(&temp, "feature");
    temp.file("src/new_file.rs", "// new file\n");
    git_commit(&temp, "feat: add new file");

    // CI mode should detect main as base and compare
    // Use --no-git since default project CLAUDE.md doesn't have Commits section
    let result = cli()
        .pwd(temp.path())
        .env("QUENCH_DEBUG", "1")
        .args(&["--ci", "--no-git"])
        .passes();

    // Debug output should mention the detected base
    result.stderr_has("main");
}

/// Spec: docs/specs/01-cli.md#scope-flags
///
/// > --ci falls back to master if main doesn't exist
#[test]
fn ci_mode_falls_back_to_master() {
    let temp = default_project();
    git_init(&temp);

    // Rename main to master
    std::process::Command::new("git")
        .args(["branch", "-m", "master"])
        .current_dir(temp.path())
        .output()
        .expect("git branch rename should succeed");

    git_initial_commit(&temp);
    git_branch(&temp, "feature");
    temp.file("src/new_file.rs", "// new file\n");
    git_commit(&temp, "feat: add new file");

    // CI mode should detect master as base
    // Use --no-git since default project CLAUDE.md doesn't have Commits section
    let result = cli()
        .pwd(temp.path())
        .env("QUENCH_DEBUG", "1")
        .args(&["--ci", "--no-git"])
        .passes();

    result.stderr_has("master");
}

// =============================================================================
// METRICS PERSISTENCE - FILE
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save <FILE> saves metrics to file
#[test]
fn save_writes_metrics_to_file() {
    let temp = default_project();
    let save_path = temp.path().join(".quench/metrics.json");

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save", save_path.to_str().unwrap()])
        .passes();

    // File should exist and contain valid JSON
    assert!(save_path.exists(), "metrics file should be created");

    let content = std::fs::read_to_string(&save_path).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("metrics file should be valid JSON");

    // Should have metrics structure
    assert!(json.get("checks").is_some(), "should have checks field");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save creates parent directories if needed
#[test]
fn save_creates_parent_directories() {
    let temp = default_project();
    let save_path = temp.path().join("deep/nested/path/metrics.json");

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save", save_path.to_str().unwrap()])
        .passes();

    assert!(
        save_path.exists(),
        "metrics file should be created with parents"
    );
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save requires --ci mode (or warn?)
#[test]
fn save_works_only_with_ci_mode() {
    let temp = default_project();
    let save_path = temp.path().join("metrics.json");

    // Without --ci, --save should still work but may warn
    cli()
        .pwd(temp.path())
        .args(&["--save", save_path.to_str().unwrap()])
        .passes();

    // Metrics should still be saved
    assert!(save_path.exists(), "--save should work without --ci");
}

// =============================================================================
// METRICS PERSISTENCE - GIT NOTES
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --fix saves to git notes by default (was --save-notes)
#[test]
fn fix_saves_to_git_notes_by_default() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    // Use --no-git since default project CLAUDE.md doesn't have Commits section
    cli()
        .pwd(temp.path())
        .args(&["--ci", "--fix", "--no-git"])
        .passes();

    // Git notes should be created for HEAD
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "show", "HEAD"])
        .current_dir(temp.path())
        .output()
        .expect("git notes show should succeed");

    assert!(output.status.success(), "git notes should exist for HEAD");

    let content = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("git note should be valid JSON");

    assert!(json.get("checks").is_some(), "should have checks field");
}

/// Spec: docs/specs/01-cli.md#output-flags (legacy)
///
/// > --save-notes stores metrics in git notes (legacy flag, still supported)
#[test]
fn save_notes_writes_to_git() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    // Use --no-git since default project CLAUDE.md doesn't have Commits section
    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes", "--no-git"])
        .passes();

    // Git notes should be created for HEAD
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "show", "HEAD"])
        .current_dir(temp.path())
        .output()
        .expect("git notes show should succeed");

    assert!(output.status.success(), "git notes should exist for HEAD");

    let content = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("git note should be valid JSON");

    assert!(json.get("checks").is_some(), "should have checks field");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save-notes requires git repository
#[test]
fn save_notes_fails_without_git() {
    let temp = default_project();
    // No git init

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes"])
        .exits(2)
        .stderr_has("not a git repository");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save-notes uses refs/notes/quench namespace
#[test]
fn save_notes_uses_quench_namespace() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    // Use --no-git since default project CLAUDE.md doesn't have Commits section
    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes", "--no-git"])
        .passes();

    // Check that refs/notes/quench exists
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "list"])
        .current_dir(temp.path())
        .output()
        .expect("git notes list should succeed");

    assert!(output.status.success(), "quench notes ref should exist");
    assert!(!output.stdout.is_empty(), "should have at least one note");
}

// =============================================================================
// LOCAL CACHE
// =============================================================================

/// Spec: docs/specs/04-ratcheting.md#local-cache
///
/// > --fix also writes .quench/latest.json for local caching
#[test]
#[ignore = "TODO: Phase 3 - Local cache implementation"]
fn fix_writes_latest_json_cache() {
    // Setup: git project
    // Run: quench check --fix
    // Assert: .quench/latest.json exists with current metrics
}
