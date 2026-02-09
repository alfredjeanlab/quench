// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Edge case specs.
//!
//! These tests verify graceful handling of edge cases discovered during dogfooding.

use crate::prelude::*;

/// Edge case: sync_from file doesn't exist
///
/// > When sync_from is configured but the file doesn't exist,
/// > the check should not panic and should skip syncing gracefully.
#[test]
fn agents_sync_source_missing_gracefully_handles() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_from = "CLAUDE.md"
sections.required = []
required = [".cursorrules"]
"#,
    );
    // Only create .cursorrules, not CLAUDE.md (the sync source)
    temp.file(".cursorrules", "# Target\n\nSome content.\n");

    // Should not panic - sync is skipped when source doesn't exist
    // The check passes because .cursorrules exists and no sync source means no sync
    let result = check("agents").pwd(temp.path()).json().passes();
    // Just verify we get a result without panic
    assert!(result.raw_json().contains("agents"));
}

/// Edge case: sync with identical files reports in_sync
///
/// > When multiple agent files have identical content,
/// > in_sync should be true in metrics.
#[test]
fn agents_identical_files_reports_in_sync() {
    let content =
        "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_from = "CLAUDE.md"
"#,
    );
    temp.file("CLAUDE.md", content);
    temp.file(".cursorrules", content);

    let result = check("agents").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    assert_eq!(
        metrics.get("in_sync").and_then(|v| v.as_bool()),
        Some(true),
        "identical files should report in_sync: true"
    );
}

/// Edge case: empty agent file should still validate sections
///
/// > An empty agent file should fail section validation if
/// > required sections are configured.
#[test]
fn agents_empty_file_validates_sections() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = ["CLAUDE.md"]
sections.required = ["Directory Structure"]
"#,
    );
    temp.file("CLAUDE.md", "");

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(result.has_violation("missing_section"), "empty file should fail section validation");
}

/// Edge case: whitespace-only agent file
///
/// > A file with only whitespace should be treated similarly to an empty file.
#[test]
fn agents_whitespace_only_file_validates_sections() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = ["CLAUDE.md"]
sections.required = ["Directory Structure"]
"#,
    );
    temp.file("CLAUDE.md", "   \n\n   \n");

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(
        result.has_violation("missing_section"),
        "whitespace-only file should fail section validation"
    );
}
