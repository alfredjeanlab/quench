//! File detection and violation detection specs.
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

    // Verify advice includes section name and configured advice
    let advice_text = advice.unwrap();
    assert!(
        advice_text.contains("Landing the Plane"),
        "advice should include section name"
    );
    assert!(
        advice_text.contains("Checklist"),
        "advice should include configured advice text"
    );
}

/// Spec: docs/specs/checks/agents.md#forbidden-sections
///
/// > Having a forbidden section generates a violation.
#[test]
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

/// Spec: docs/specs/checks/agents.md#glob-patterns
///
/// > Glob patterns match multiple section names.
#[test]
fn agents_forbidden_section_glob_matches() {
    let agents = check("agents")
        .on("agents/forbidden-section")
        .json()
        .fails();
    let violations = agents.require("violations").as_array().unwrap();

    let matches_test = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Test*"))
                .unwrap_or(false)
    });

    assert!(matches_test, "should match Test* glob pattern");
}
