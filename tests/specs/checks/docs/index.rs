//! Behavioral specs for specs index file detection.
//!
//! Reference: docs/specs/checks/docs.md#index-file

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// INDEX FILE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#index-file
///
/// > Detection order: 1. {path}/CLAUDE.md 2. docs/CLAUDE.md
/// > 3. {path}/[00-]{overview,summary,index}.md 4. docs/SPECIFICATIONS.md
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn specs_directory_index_file_detected() {
    let docs = check("docs").on("docs/index-auto").json().passes();
    let metrics = docs.require("metrics");

    assert!(
        metrics.get("index_file").is_some(),
        "should have index_file in metrics"
    );
}

/// Spec: docs/specs/checks/docs.md#toc-format
///
/// > `linked` mode: All spec files must be reachable via markdown links.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn unreachable_spec_file_generates_violation_linked_mode() {
    check("docs")
        .on("docs/unreachable-spec")
        .fails()
        .stdout_has("unreachable from index");
}

/// Spec: docs/specs/checks/docs.md#index-file
///
/// > `exists` mode: Index file must exist, no reachability check.
#[test]
fn exists_mode_only_checks_index_exists() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.specs]
path = "docs/specs"
index = "exists"
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Specs Index\n");
    temp.file("docs/specs/orphan.md", "# Orphan (not linked)\n");

    // In exists mode, orphan.md is not flagged as unreachable
    check("docs").pwd(temp.path()).passes();
}
