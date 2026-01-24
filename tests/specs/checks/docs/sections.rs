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
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
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
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
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
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn section_matching_is_case_insensitive() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[check.docs]
path = "docs/specs"
sections.required = ["purpose"]
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(dir.path().join("docs/specs/CLAUDE.md"), "# Specs\n").unwrap();
    std::fs::write(
        dir.path().join("docs/specs/feature.md"),
        "# Feature\n\n## PURPOSE\n\nThis is the purpose.\n",
    )
    .unwrap();

    // "PURPOSE" should match required "purpose"
    check("docs").pwd(dir.path()).passes();
}
