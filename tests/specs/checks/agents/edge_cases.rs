//! Edge case specs.
//!
//! These tests verify graceful handling of edge cases discovered during dogfooding.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Edge case: sync_source file doesn't exist
///
/// > When sync_source is configured but the file doesn't exist,
/// > the check should not panic and should skip syncing gracefully.
#[test]
fn agents_sync_source_missing_gracefully_handles() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
sections.required = []
required = [".cursorrules"]
"#,
    )
    .unwrap();

    // Only create .cursorrules, not CLAUDE.md (the sync source)
    std::fs::write(
        dir.path().join(".cursorrules"),
        "# Target\n\nSome content.\n",
    )
    .unwrap();

    // Should not panic - sync is skipped when source doesn't exist
    // The check passes because .cursorrules exists and no sync source means no sync
    let result = check("agents").pwd(dir.path()).json().passes();
    // Just verify we get a result without panic
    assert!(result.raw_json().contains("agents"));
}

/// Edge case: sync with identical files reports in_sync
///
/// > When multiple agent files have identical content,
/// > in_sync should be true in metrics.
#[test]
fn agents_identical_files_reports_in_sync() {
    let dir = tempfile::tempdir().unwrap();
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

    let content =
        "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), content).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), content).unwrap();

    let result = check("agents").pwd(dir.path()).json().passes();
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
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
required = ["CLAUDE.md"]
sections.required = ["Directory Structure"]
"#,
    )
    .unwrap();

    // Create an empty agent file
    std::fs::write(dir.path().join("CLAUDE.md"), "").unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section")),
        "empty file should fail section validation"
    );
}

/// Edge case: whitespace-only agent file
///
/// > A file with only whitespace should be treated similarly to an empty file.
#[test]
fn agents_whitespace_only_file_validates_sections() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
required = ["CLAUDE.md"]
sections.required = ["Directory Structure"]
"#,
    )
    .unwrap();

    // Create a whitespace-only agent file
    std::fs::write(dir.path().join("CLAUDE.md"), "   \n\n   \n").unwrap();

    let result = check("agents").pwd(dir.path()).json().fails();
    let violations = result.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section")),
        "whitespace-only file should fail section validation"
    );
}
