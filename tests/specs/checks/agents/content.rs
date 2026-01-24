//! Content rules specs.
//!
//! Reference: docs/specs/checks/agents.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Spec: docs/specs/checks/agents.md#tables
///
/// > Markdown tables generate a violation when tables = "forbid".
#[test]
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

/// Spec: docs/specs/checks/agents.md#box-diagrams
///
/// > Box diagrams generate a violation when box_diagrams = "forbid".
#[test]
fn agents_box_diagram_generates_violation() {
    let agents = check("agents").on("agents/with-box-diagram").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_diagram")),
        "should have forbidden_diagram violation"
    );
}

/// Spec: docs/specs/checks/agents.md#mermaid
///
/// > Mermaid blocks generate a violation when mermaid = "forbid".
#[test]
fn agents_mermaid_block_generates_violation() {
    let agents = check("agents").on("agents/with-mermaid").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_mermaid")),
        "should have forbidden_mermaid violation"
    );
}

/// Spec: docs/specs/checks/agents.md#size-limits
///
/// > Violations include value and threshold in JSON output.
#[test]
fn agents_size_violation_includes_threshold() {
    let agents = check("agents").on("agents/oversized-lines").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let size_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("file_too_large"));

    assert!(
        size_violation.is_some(),
        "should have file_too_large violation"
    );

    let v = size_violation.unwrap();
    assert!(v.get("value").is_some(), "should have value field");
    assert!(v.get("threshold").is_some(), "should have threshold field");
}
