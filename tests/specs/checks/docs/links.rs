//! Behavioral specs for markdown link validation in the docs check.
//!
//! Reference: docs/specs/checks/docs.md#fast-mode-link-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// LINK VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > Valid markdown links to local files should pass.
#[test]
fn valid_markdown_link_passes() {
    check("docs").on("docs/link-ok").passes();
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > Markdown links to local files are validated.
#[test]
fn markdown_link_to_missing_file_generates_violation() {
    check("docs")
        .on("docs/link-broken")
        .fails()
        .stdout_has("docs: FAIL")
        .stdout_has("broken_link");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > External URLs (http/https) are not validated.
#[test]
fn external_urls_not_validated() {
    check("docs").on("docs/link-external").passes();
}

/// Spec: docs/specs/checks/docs.md#output-1
///
/// > README.md:45: broken link: docs/old-guide.md
#[test]
fn broken_link_includes_file_and_line() {
    let docs = check("docs").on("docs/link-broken").json().fails();
    let violations = docs.require("violations").as_array().unwrap();

    let link_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("broken_link"))
        .expect("should have broken_link violation");

    assert!(link_violation.get("file").is_some(), "should have file");
    assert!(link_violation.get("line").is_some(), "should have line");
    assert!(
        link_violation.get("pattern").is_some(),
        "should have pattern"
    );
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated-1
///
/// > Check [configuration](../02-config.md) for options.
#[test]
fn relative_path_links_validated() {
    let temp = default_project();
    temp.file(
        "docs/specs/overview.md",
        "See [config](../config.md) for details.\n",
    );
    // ../config.md doesn't exist relative to docs/specs/overview.md
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("config.md");
}
