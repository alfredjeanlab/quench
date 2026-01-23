//! Behavioral specs for the agents check.
//!
//! Tests that quench correctly:
//! - Detects agent context files (CLAUDE.md, .cursorrules)
//! - Validates file synchronization
//! - Checks required/forbidden sections
//! - Enforces content rules (tables, max_lines, max_tokens)
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/agents.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// FILE DETECTION SPECS (Phase 1)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#agent-files
///
/// > The agents check detects CLAUDE.md at the project root.
#[test]
fn agents_detects_claude_md_at_project_root() {
    let agents = check("agents").on("agents/basic").json().passes();
    let metrics = agents.require("metrics");
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert!(
        files_found.iter().any(|f| f.as_str() == Some("CLAUDE.md")),
        "should detect CLAUDE.md"
    );
}

/// Spec: docs/specs/checks/agents.md#agent-files
///
/// > The agents check detects .cursorrules at the project root.
#[test]
fn agents_detects_cursorrules_at_project_root() {
    let agents = check("agents").on("agents/basic").json().passes();
    let metrics = agents.require("metrics");
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert!(
        files_found
            .iter()
            .any(|f| f.as_str() == Some(".cursorrules")),
        "should detect .cursorrules"
    );
}

/// Spec: docs/specs/checks/agents.md#passing-check
///
/// > Check passes when all configured files exist and are valid.
#[test]
fn agents_passes_on_valid_project() {
    check("agents").on("agents/basic").passes();
}

// =============================================================================
// VIOLATION DETECTION SPECS (Phase 2)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#required-files
///
/// > Missing a required file generates a violation.
#[test]
fn agents_missing_required_file_generates_violation() {
    let agents = check("agents").on("agents/missing-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("missing_file") }),
        "should have missing_file violation"
    );
}

/// Spec: docs/specs/checks/agents.md#forbidden-files
///
/// > Having a forbidden file generates a violation.
#[test]
fn agents_forbidden_file_generates_violation() {
    let agents = check("agents").on("agents/forbidden-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_file") }),
        "should have forbidden_file violation"
    );
}

/// Spec: docs/specs/checks/agents.md#sync-behavior
///
/// > Files out of sync with sync_source generate a violation.
#[test]
fn agents_out_of_sync_generates_violation() {
    let agents = check("agents").on("agents/out-of-sync").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("out_of_sync") }),
        "should have out_of_sync violation"
    );
}

/// Spec: docs/specs/checks/agents.md#required-sections
///
/// > Missing a required section generates a violation with advice.
#[test]
#[ignore = "TODO: Phase 515 - Agents Section Validation"]
fn agents_missing_section_generates_violation_with_advice() {
    let agents = check("agents").on("agents/missing-section").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let missing_section = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section"));

    assert!(
        missing_section.is_some(),
        "should have missing_section violation"
    );

    let advice = missing_section
        .unwrap()
        .get("advice")
        .and_then(|a| a.as_str());
    assert!(
        advice.is_some() && !advice.unwrap().is_empty(),
        "missing_section violation should have advice"
    );
}

/// Spec: docs/specs/checks/agents.md#forbidden-sections
///
/// > Having a forbidden section generates a violation.
#[test]
#[ignore = "TODO: Phase 515 - Agents Section Validation"]
fn agents_forbidden_section_generates_violation() {
    let agents = check("agents")
        .on("agents/forbidden-section")
        .json()
        .fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section") }),
        "should have forbidden_section violation"
    );
}

// =============================================================================
// CONTENT RULES SPECS (Phase 3)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#tables
///
/// > Markdown tables generate a violation when tables = "forbid".
#[test]
#[ignore = "TODO: Phase 520 - Agents Content Rules"]
fn agents_markdown_table_generates_violation() {
    let agents = check("agents").on("agents/with-table").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("forbidden_table") }),
        "should have forbidden_table violation"
    );
}

/// Spec: docs/specs/checks/agents.md#max-lines
///
/// > File exceeding max_lines generates a violation.
#[test]
#[ignore = "TODO: Phase 520 - Agents Content Rules"]
fn agents_file_over_max_lines_generates_violation() {
    let agents = check("agents").on("agents/oversized-lines").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("file_too_large") }),
        "should have file_too_large violation"
    );
}

/// Spec: docs/specs/checks/agents.md#max-tokens
///
/// > File exceeding max_tokens generates a violation.
#[test]
#[ignore = "TODO: Phase 520 - Agents Content Rules"]
fn agents_file_over_max_tokens_generates_violation() {
    let agents = check("agents").on("agents/oversized-tokens").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("file_too_large") }),
        "should have file_too_large violation"
    );
}

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
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Source\nContent B").unwrap();

    // Run with --fix
    check("agents").pwd(dir.path()).args(&["--fix"]).passes();

    // Verify files are now synced
    let cursorrules = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(cursorrules, "# Source\nContent A");
}
