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
/// > 2. Relative to project root
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

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Relative to the markdown file's directory (`.`/`./` treated as current directory)
#[test]
fn toc_dot_prefix_resolves_relative_to_markdown_file() {
    let temp = default_project();
    temp.file("README.md", "# README\n");
    temp.file("crates/cli/lib.rs", "// lib\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
.
├── README.md
├── crates/
│   └── cli/
│       └── lib.rs
```
"#,
    );
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Strip markdown file's parent directory name prefix
#[test]
fn toc_parent_dir_prefix_stripped() {
    let temp = default_project();
    temp.file("README.md", "# README\n");
    temp.file("crates/cli/lib.rs", "// lib\n");
    // The temp directory name is used as the tree root
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
TEMPNAME/
├── README.md
├── crates/
│   └── cli/
│       └── lib.rs
```
"#,
    );
    // Replace TEMPNAME with actual directory name
    let dir_name = temp.path().file_name().unwrap().to_str().unwrap();
    let content = std::fs::read_to_string(temp.path().join("CLAUDE.md")).unwrap();
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        content.replace("TEMPNAME", dir_name),
    )
    .unwrap();
    check("docs").pwd(temp.path()).passes();
}
