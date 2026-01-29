// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

use crate::prelude::*;

/// Valid source content with required sections.
const SOURCE: &str =
    "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";

/// Different target content (will need syncing).
const TARGET: &str = "# Target\n\n## Different\n\nContent B\n";

// =============================================================================
// ERROR HANDLING SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run: Show what --fix would change without changing it
/// > Using --dry-run without --fix is an error.
#[test]
fn dry_run_without_fix_is_error() {
    let temp = default_project();
    cli()
        .pwd(temp.path())
        .args(&["--dry-run"])
        .exits(2) // Configuration error
        .stderr_has("--fix")
        .stderr_has("preview");
}

// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows files that would be modified without modifying them.
#[test]
fn dry_run_shows_files_that_would_be_modified() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    );
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", TARGET);

    cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has(".cursorrules");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows diff of proposed changes.
#[test]
fn dry_run_shows_diff_of_changes() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
sections.required = []
"#,
    );
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", TARGET);

    // Diff output should show both old and new content
    cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has("Content B") // Old content (being removed)
        .stdout_has("Landing the Plane"); // New content (being added)
}

// =============================================================================
// EXIT CODE SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run exits 0 even when fixes are needed.
#[test]
fn dry_run_exits_0_when_fixes_needed() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    );
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", TARGET);

    // Files are out of sync, fixes are needed, but --dry-run exits 0
    cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .passes(); // passes() expects exit code 0
}

// =============================================================================
// FILE INTEGRITY SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run does not modify any files.
#[test]
fn dry_run_does_not_modify_files() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    );
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", TARGET);

    // Run with --dry-run
    cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .passes();

    // Verify .cursorrules was NOT modified
    let content = std::fs::read_to_string(temp.path().join(".cursorrules")).unwrap();
    assert_eq!(content, TARGET, "file should not be modified");
}

// =============================================================================
// EDGE CASE SPECS (Phase 6C)
// =============================================================================

/// Edge case: dry-run with no changes needed
///
/// > When files are already in sync, dry-run should show PASS with no preview.
#[test]
fn dry_run_no_changes_shows_clean() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    );
    // Both files have the same content
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", SOURCE);

    // Dry-run should pass with no preview needed
    cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_lacks("Would sync"); // No preview shown
}

/// Edge case: dry-run with JSON output includes previews
///
/// > JSON output in dry-run mode should include previews in fix_summary.
#[test]
fn dry_run_json_output_includes_previews() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
sections.required = []
"#,
    );
    temp.file("CLAUDE.md", SOURCE);
    temp.file(".cursorrules", TARGET);

    let result = cli()
        .pwd(temp.path())
        .args(&["--fix", "--dry-run"])
        .json()
        .passes();

    // Find the agents check result
    let agents = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("agents"))
        .expect("should have agents check");

    let fix_summary = agents.get("fix_summary").expect("should have fix_summary");
    let previews = fix_summary
        .get("previews")
        .and_then(|p| p.as_array())
        .expect("should have previews array");

    assert!(!previews.is_empty(), "previews should not be empty");
    assert!(
        previews[0].get("file").is_some(),
        "preview should have file"
    );
    assert!(
        previews[0].get("source").is_some(),
        "preview should have source"
    );
}
