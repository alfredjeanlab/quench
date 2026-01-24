//! Output format specs (JSON, text, fix behavior).
//!
//! Reference: docs/specs/checks/agents.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// JSON OUTPUT SPECS (Phase 4)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON output includes files_found and in_sync metrics.
#[test]
fn agents_json_includes_files_found_and_in_sync_metrics() {
    let agents = check("agents").on("agents/metrics").json().passes();
    let metrics = agents.require("metrics");

    assert!(
        metrics.get("files_found").is_some(),
        "should have files_found metric"
    );
    assert!(
        metrics.get("in_sync").is_some(),
        "should have in_sync metric"
    );
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > Violation types are one of the expected values.
#[test]
fn agents_violation_type_is_valid() {
    let agents = check("agents").on("agents/missing-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_file",
        "forbidden_file",
        "out_of_sync",
        "missing_section",
        "forbidden_section",
        "forbidden_table",
        "forbidden_diagram",
        "forbidden_mermaid",
        "file_too_large",
    ];

    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(valid_types.contains(&vtype), "unexpected type: {}", vtype);
    }
}

// =============================================================================
// FIX BEHAVIOR SPECS (Phase 4)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#sync-behavior
///
/// > Running with --fix syncs files from sync_source.
#[test]
fn agents_fix_syncs_files_from_sync_source() {
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
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    // Run with --fix
    check("agents").pwd(dir.path()).args(&["--fix"]).passes();

    // Verify files are now synced
    let cursorrules = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(cursorrules, source);
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing file shows human-readable description.
#[test]
fn agents_missing_file_text_output() {
    check("agents")
        .on("agents/missing-file")
        .fails()
        .stdout_has("missing required file");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync shows other file name.
#[test]
fn agents_out_of_sync_text_output() {
    check("agents")
        .on("agents/out-of-sync")
        .fails()
        .stdout_has("out of sync with");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync with differing preamble shows "(preamble)" not empty string.
#[test]
fn agents_out_of_sync_preamble_text_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create two files with different preambles (content before ## headings)
    std::fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project A\n\nFirst preamble.\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".cursorrules"),
        "# Project B\n\nDifferent preamble.\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n",
    )
    .unwrap();

    check("agents")
        .pwd(dir.path())
        .fails()
        .stdout_has("(preamble)")
        .stdout_lacks("Section \"\"");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing section includes section name and advice.
#[test]
fn agents_missing_section_text_output() {
    check("agents")
        .on("agents/missing-section")
        .fails()
        .stdout_has("Landing the Plane")
        .stdout_has("Checklist");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Forbidden table shows line number.
#[test]
fn agents_forbidden_table_text_output() {
    let output = check("agents").on("agents/with-table").fails();
    // Verify line number is present (format: CLAUDE.md:N: forbidden table)
    let stdout = output.stdout();
    assert!(
        stdout.contains(":") && stdout.contains("forbidden table"),
        "should show file:line: forbidden table, got: {}",
        stdout
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > File too large shows value vs threshold.
#[test]
fn agents_file_too_large_text_output() {
    check("agents")
        .on("agents/oversized-lines")
        .fails()
        .stdout_has("vs");
}

// =============================================================================
// FIXED STATUS SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#fixed
///
/// > Running with --fix shows FIXED status when files are synced.
#[test]
fn agents_fix_shows_fixed_status() {
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
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .passes()
        .stdout_has("FIXED")
        .stdout_has("Synced");
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON includes fixed:true when --fix applies changes.
#[test]
fn agents_fix_json_includes_fixed_field() {
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
    let source =
        "# Source\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
    std::fs::write(dir.path().join("CLAUDE.md"), source).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Different content").unwrap();

    let result = check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .json()
        .passes();

    assert_eq!(
        result.require("fixed").as_bool(),
        Some(true),
        "should have fixed: true"
    );
}

// =============================================================================
// EXACT OUTPUT FORMAT SPECS
// =============================================================================
// These tests verify exact output format using direct comparison.
// Any output change requires explicit test update (no auto-accept).

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing file shows human-readable description with exact format.
#[test]
fn exact_missing_file_text() {
    check("agents").on("agents/missing-file").fails().stdout_eq(
        r###"agents: FAIL
  CLAUDE.md: missing required file
    Required agent file 'CLAUDE.md' not found at project root
FAIL: agents
"###,
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync shows other file name with exact format.
#[test]
fn exact_out_of_sync_text() {
    check("agents").on("agents/out-of-sync").fails().stdout_eq(
        r###"agents: FAIL
  .cursorrules: out of sync with CLAUDE.md
    Code Style differs. Use --fix to sync from CLAUDE.md, or reconcile manually.
  CLAUDE.md: missing required section
    Add a "## Directory Structure" section: Overview of project layout and key directories
  CLAUDE.md: missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before completing work
  .cursorrules: missing required section
    Add a "## Directory Structure" section: Overview of project layout and key directories
  .cursorrules: missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before completing work
FAIL: agents
"###,
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Forbidden table shows line number with exact format.
#[test]
fn exact_forbidden_table_text() {
    check("agents").on("agents/with-table").fails().stdout_eq(
        r###"agents: FAIL
  CLAUDE.md: missing required section
    Add a "## Directory Structure" section: Overview of project layout and key directories
  CLAUDE.md: missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before completing work
  CLAUDE.md:7: forbidden table
    Tables are not token-efficient. Convert to a list or prose.
FAIL: agents
"###,
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing section includes section name and advice with exact format.
#[test]
fn exact_missing_section_text() {
    check("agents")
        .on("agents/missing-section")
        .fails()
        .stdout_eq(
            r###"agents: FAIL
  CLAUDE.md: missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before finishing work
FAIL: agents
"###,
        );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > File too large shows value vs threshold with exact format.
#[test]
fn exact_oversized_lines_text() {
    check("agents")
        .on("agents/oversized-lines")
        .fails()
        .stdout_eq(
            r###"agents: FAIL
  CLAUDE.md: missing required section
    Add a "## Directory Structure" section: Overview of project layout and key directories
  CLAUDE.md: missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before completing work
  CLAUDE.md: file too large (59 vs 50)
    File has 59 lines (max: 50). Split into smaller files or reduce content.
FAIL: agents
"###,
        );
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON output for multi-scope project includes expected structure.
#[test]
fn exact_agents_project_json() {
    let result = check("agents").on("agents-project").json().passes();
    let metrics = result.require("metrics");

    // Verify structure without timestamp dependency
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert_eq!(files_found.len(), 3);
    assert!(files_found.iter().any(|f| f.as_str() == Some("CLAUDE.md")));
    assert!(
        files_found
            .iter()
            .any(|f| f.as_str() == Some(".cursorrules"))
    );
    assert!(
        files_found
            .iter()
            .any(|f| f.as_str() == Some("crates/api/CLAUDE.md"))
    );

    let files_missing = metrics.get("files_missing").unwrap().as_array().unwrap();
    assert!(files_missing.is_empty());

    assert_eq!(metrics.get("in_sync").unwrap().as_bool(), Some(true));
}
