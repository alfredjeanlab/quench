//! Behavioral specs for the --dry-run flag.
//!
//! Tests that quench correctly handles dry-run mode:
//! - Requires --fix flag
//! - Shows files that would be modified
//! - Shows diff of proposed changes
//! - Exits 0 even when fixes needed
//! - Does not modify any files
//!
//! Reference: docs/specs/01-cli.md#output-flags

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// ERROR HANDLING SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run: Show what --fix would change without changing it
/// > Using --dry-run without --fix is an error.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_without_fix_is_error() {
    let dir = temp_project();
    cli()
        .pwd(dir.path())
        .args(&["--dry-run"])
        .exits(2) // Configuration error
        .stderr_has("--dry-run requires --fix");
}

// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows files that would be modified without modifying them.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_shows_files_that_would_be_modified() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has(".cursorrules");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows diff of proposed changes.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_shows_diff_of_changes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Diff output should show both old and new content
    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has("Content B") // Old content (being removed)
        .stdout_has("Content A"); // New content (being added)
}

// =============================================================================
// EXIT CODE SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run exits 0 even when fixes are needed.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_exits_0_when_fixes_needed() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Files are out of sync, fixes are needed, but --dry-run exits 0
    cli().pwd(dir.path()).args(&["--fix", "--dry-run"]).passes(); // passes() expects exit code 0
}

// =============================================================================
// FILE INTEGRITY SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run does not modify any files.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_does_not_modify_files() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Run with --dry-run
    cli().pwd(dir.path()).args(&["--fix", "--dry-run"]).passes();

    // Verify .cursorrules was NOT modified
    let content = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(
        content, "# Target\nContent B",
        "file should not be modified"
    );
}
