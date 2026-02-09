// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for spec content validation.
//!
//! Tests content rules (tables, diagrams, mermaid), size limits, and
//! section validation for spec files in docs/specs/.
//!
//! Reference: docs/specs/checks/docs.md#content-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SECTION VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.required = ["Purpose"]
#[test]
fn spec_missing_required_section() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
sections.required = ["Purpose"]
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\nSome content.\n");

    check("docs").pwd(temp.path()).fails().stdout_has("missing").stdout_has("section");
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > Present required sections pass validation.
#[test]
fn spec_has_required_section() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
sections.required = ["Purpose"]
"#,
    );
    // Both spec files need the required section
    temp.file("docs/specs/CLAUDE.md", "# Overview\n\n## Purpose\n\nIndex purpose.\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n## Purpose\n\nExplains the feature.\n");

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.forbid = ["TODO", "Draft*"]
#[test]
fn spec_has_forbidden_section() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
sections.forbid = ["TODO", "Draft*"]
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n## Draft Notes\n\nWork in progress.\n");

    check("docs").pwd(temp.path()).fails().stdout_has("forbidden section");
}

// =============================================================================
// CONTENT RULE SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > tables = "allow" (default for specs)
#[test]
fn spec_tables_allowed_by_default() {
    let temp = default_project();
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n| A | B |\n|---|---|\n| 1 | 2 |\n");

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > tables = "forbid" generates forbidden_table violation
#[test]
fn spec_tables_forbidden_when_configured() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
tables = "forbid"
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n| A | B |\n|---|---|\n| 1 | 2 |\n");

    check("docs").pwd(temp.path()).fails().stdout_has("forbidden table");
}

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > box_diagrams = "allow" (default for specs)
#[test]
fn spec_box_diagrams_allowed_by_default() {
    let temp = default_project();
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n```\n┌───┐\n│ A │\n└───┘\n```\n");

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > box_diagrams = "forbid" generates forbidden_diagram violation
#[test]
fn spec_box_diagrams_forbidden_when_configured() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
box_diagrams = "forbid"
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n┌───┐\n│ A │\n└───┘\n");

    check("docs").pwd(temp.path()).fails().stdout_has("forbidden box diagram");
}

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > mermaid = "allow" (default for specs)
#[test]
fn spec_mermaid_allowed_by_default() {
    let temp = default_project();
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n```mermaid\ngraph TD;\nA-->B;\n```\n");

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#content-rules
///
/// > mermaid = "forbid" generates forbidden_mermaid violation
#[test]
fn spec_mermaid_forbidden_when_configured() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
mermaid = "forbid"
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", "# Feature\n\n```mermaid\ngraph TD;\nA-->B;\n```\n");

    check("docs").pwd(temp.path()).fails().stdout_has("forbidden mermaid");
}

// =============================================================================
// SIZE LIMIT SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#size-limits
///
/// > max_lines = 1000 (default for specs)
#[test]
fn spec_within_default_line_limit() {
    let temp = default_project();
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    // 500 lines is well under the 1000 line default
    temp.file("docs/specs/feature.md", &"line\n".repeat(500));

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#size-limits
///
/// > max_lines = N generates spec_too_large when exceeded
#[test]
fn spec_exceeds_line_limit() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
max_lines = 10
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    temp.file("docs/specs/feature.md", &"line\n".repeat(20));

    check("docs").pwd(temp.path()).fails().stdout_has("spec_too_large");
}

/// Spec: docs/specs/checks/docs.md#size-limits
///
/// > max_lines = false disables the line limit
#[test]
fn spec_line_limit_disabled() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
max_lines = false
max_tokens = false
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    // 2000 lines would exceed default, but limit is disabled
    temp.file("docs/specs/feature.md", &"line\n".repeat(2000));

    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#size-limits
///
/// > max_tokens = 20000 (default for specs)
#[test]
fn spec_exceeds_token_limit() {
    let temp = default_project();
    temp.config(
        r#"[check.docs.specs]
max_tokens = 100
max_lines = false
"#,
    );
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    // ~500 chars / 4 = ~125 tokens, exceeds 100
    temp.file("docs/specs/feature.md", &"a".repeat(500));

    check("docs").pwd(temp.path()).fails().stdout_has("spec_too_large").stdout_has("tokens");
}

// =============================================================================
// DEFAULT BEHAVIOR SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#content-validation
///
/// > By default, specs have no required sections (unlike agents)
#[test]
fn spec_default_no_required_sections() {
    let temp = default_project();
    temp.file("docs/specs/CLAUDE.md", "# Overview\n");
    // No "Directory Structure" or "Landing the Plane" required for specs
    temp.file("docs/specs/feature.md", "# Feature\n\nJust content.\n");

    check("docs").pwd(temp.path()).passes();
}
