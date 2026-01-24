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
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn exists_mode_only_checks_index_exists() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs]
path = "docs/specs"
index = "exists"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(dir.path().join("docs/specs/CLAUDE.md"), "# Specs Index\n").unwrap();
    std::fs::write(
        dir.path().join("docs/specs/orphan.md"),
        "# Orphan (not linked)\n",
    )
    .unwrap();

    // In exists mode, orphan.md is not flagged as unreachable
    check("docs").pwd(dir.path()).passes();
}
