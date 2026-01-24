//! Behavioral specs for TOC (directory tree) validation in the docs check.
//!
//! Reference: docs/specs/checks/docs.md#fast-mode-toc-validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// TOC TREE VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Each file in the tree is checked for existence.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_tree_entries_validated_against_filesystem() {
    // Valid TOC with all files existing should pass
    check("docs").on("docs/toc-ok").passes();
}

/// Spec: docs/specs/checks/docs.md#output
///
/// > CLAUDE.md:72: toc path not found: checks/coverage.md
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_toc_path_generates_violation() {
    check("docs")
        .on("docs/toc-broken")
        .fails()
        .stdout_has("docs: FAIL")
        .stdout_has("toc path not found");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Both box-drawing format and indentation format are supported.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_box_drawing_format_supported() {
    let temp = default_project();
    temp.file(
        "docs/specs/overview.md",
        "# Overview\n\n## Purpose\n\nTest.\n",
    );
    temp.file(
        "docs/CLAUDE.md",
        r#"# Docs

## File Structure

```
docs/specs/
├── overview.md
└── config.md
```
"#,
    );
    // config.md doesn't exist - should fail
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("config.md");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Indentation format (spaces or tabs) is supported.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_indentation_format_supported() {
    let temp = default_project();
    temp.file(
        "docs/specs/overview.md",
        "# Overview\n\n## Purpose\n\nTest.\n",
    );
    temp.file(
        "docs/CLAUDE.md",
        r#"# Docs

## File Structure

```
docs/specs/
  overview.md
  missing.md
```
"#,
    );
    // missing.md doesn't exist - should fail
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("missing.md");
}

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Paths resolved in order: 1. Relative to markdown file's directory
/// > 2. Relative to docs/ directory 3. Relative to project root
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_path_resolution_order() {
    let temp = default_project();
    // Create file at project root
    temp.file("README.md", "# README\n");
    temp.file(
        "docs/CLAUDE.md",
        r#"# Docs

## File Structure

```
README.md
```
"#,
    );
    // Should resolve README.md from project root
    check("docs").pwd(temp.path()).passes();
}
