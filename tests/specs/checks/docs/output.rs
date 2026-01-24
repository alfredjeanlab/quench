//! Behavioral specs for docs check JSON output format.
//!
//! Reference: docs/specs/checks/docs.md#json-output

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// JSON OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > Violation types: missing_section, forbidden_section, broken_toc, broken_link, unreachable_spec
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn docs_violation_type_is_one_of_expected_values() {
    let docs = check("docs").on("docs/toc-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_section",
        "forbidden_section",
        "broken_toc",
        "broken_link",
        "unreachable_spec",
        "missing_docs",
    ];

    for violation in violations {
        let vtype = violation.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > broken_toc violation includes file, line, path, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_toc_violation_structure() {
    let docs = check("docs").on("docs/toc-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let toc_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_toc"))
        .expect("should have broken_toc violation");

    assert!(toc_violation.get("file").is_some(), "missing file");
    assert!(toc_violation.get("line").is_some(), "missing line");
    assert!(toc_violation.get("path").is_some(), "missing path");
    assert!(toc_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > broken_link violation includes file, line, target, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_link_violation_structure() {
    let docs = check("docs").on("docs/link-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let link_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_link"))
        .expect("should have broken_link violation");

    assert!(link_violation.get("file").is_some(), "missing file");
    assert!(link_violation.get("line").is_some(), "missing line");
    assert!(link_violation.get("target").is_some(), "missing target");
    assert!(link_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > missing_section violation includes file, section, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn missing_section_violation_structure() {
    let docs = check("docs").on("docs/section-required").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let section_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_section"))
        .expect("should have missing_section violation");

    assert!(section_violation.get("file").is_some(), "missing file");
    assert!(
        section_violation.get("section").is_some(),
        "missing section"
    );
    assert!(section_violation.get("advice").is_some(), "missing advice");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > metrics: { index_file, spec_files }
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn docs_json_metrics_structure() {
    let docs = check("docs").on("docs/index-auto").json().passes();
    let metrics = docs.require("metrics");

    assert!(metrics.get("index_file").is_some(), "missing index_file");
    assert!(metrics.get("spec_files").is_some(), "missing spec_files");
}

/// Spec: docs/specs/checks/docs.md#json-output
///
/// > forbidden_section violation includes file, section, advice
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn forbidden_section_violation_structure() {
    let docs = check("docs").on("docs/section-forbidden").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let section_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section"))
        .expect("should have forbidden_section violation");

    assert!(section_violation.get("file").is_some(), "missing file");
    assert!(
        section_violation.get("section").is_some(),
        "missing section"
    );
    assert!(section_violation.get("advice").is_some(), "missing advice");
}
