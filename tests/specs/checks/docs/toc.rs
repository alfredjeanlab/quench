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
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/overview.md"),
        "# Overview\n\n## Purpose\n\nTest.\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
docs/specs/
├── overview.md
└── config.md
```
"#,
    )
    .unwrap();
    // config.md doesn't exist - should fail
    check("docs")
        .pwd(dir.path())
        .fails()
        .stdout_has("config.md");
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Indentation format (spaces or tabs) is supported.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn toc_indentation_format_supported() {
    let dir = temp_project();
    std::fs::create_dir_all(dir.path().join("docs/specs")).unwrap();
    std::fs::write(
        dir.path().join("docs/specs/overview.md"),
        "# Overview\n\n## Purpose\n\nTest.\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
docs/specs/
  overview.md
  missing.md
```
"#,
    )
    .unwrap();
    // missing.md doesn't exist - should fail
    check("docs")
        .pwd(dir.path())
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
    let dir = temp_project();
    // Create file at project root
    std::fs::write(dir.path().join("README.md"), "# README\n").unwrap();
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(
        dir.path().join("docs/CLAUDE.md"),
        r#"# Docs

## File Structure

```
README.md
```
"#,
    )
    .unwrap();
    // Should resolve README.md from project root
    check("docs").pwd(dir.path()).passes();
}
