// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for section validation in spec files.
//!
//! Reference: docs/specs/checks/docs.md#section-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SECTION VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.required = ["Purpose", "Configuration"]
#[test]
fn missing_required_section_in_spec_generates_violation() {
    check("docs")
        .on("docs/section-required")
        .fails()
        .stdout_has("missing required section")
        .stdout_has("Purpose");
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > sections.forbid = ["TODO", "Draft*"]
#[test]
fn forbidden_section_in_spec_generates_violation() {
    check("docs")
        .on("docs/section-forbidden")
        .fails()
        .stdout_has("forbidden section")
        .stdout_has("TODO");
}

/// Spec: docs/specs/checks/docs.md#section-validation
///
/// > Case-insensitive matching for section names.
#[test]
fn section_matching_is_case_insensitive() {
    let temp = Project::empty();
    temp.config(
        r#"[check.docs.specs]
path = "docs/specs"
sections.required = ["purpose"]
"#,
    );
    temp.file(
        "docs/specs/CLAUDE.md",
        "# Specs\n\n## Purpose\n\nSpecs index.\n",
    );
    temp.file(
        "docs/specs/feature.md",
        "# Feature\n\n## PURPOSE\n\nThis is the purpose.\n",
    );

    // "PURPOSE" should match required "purpose"
    check("docs").pwd(temp.path()).passes();
}
