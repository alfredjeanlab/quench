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
/// > broken_toc: File does not exist.
#[test]
#[ignore = "TODO: Phase 602 - Docs Check Implementation"]
fn broken_toc_path_generates_violation() {
    check("docs")
        .on("docs/toc-broken")
        .fails()
        .stdout_has("docs: FAIL")
        .stdout_has("broken_toc");
}

/// Spec: docs/specs/checks/docs.md#output
///
/// > Advice message is multiline for readability.
#[test]
fn broken_toc_advice_is_multiline() {
    let temp = default_project();
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
src/
├── missing.rs
```
"#,
    );
    check("docs").pwd(temp.path()).fails().stdout_eq(
        "docs: FAIL
  CLAUDE.md:5: broken_toc: src/missing.rs
    File does not exist.
    This check ensures directory trees in documentation stay up-to-date.
    Update the table of contents or directory tree to match actual files.
    If this is illustrative, add a ```text language tag.

    Tried: relative to markdown file, relative to project root, stripping parent directory prefix

FAIL: docs
",
    );
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

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Ellipsis entries (`.`, `..`, `...`) are ignored as placeholders.
#[test]
fn toc_ellipsis_entries_ignored() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file("src/main.rs", "// main\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
src/
├── lib.rs
├── main.rs
└── ...
```
"#,
    );
    // The `...` should be ignored, not treated as a file to validate
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Dot entries (`.` and `..`) are ignored as directory references.
#[test]
fn toc_dot_entries_ignored() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
.
├── src/
│   └── lib.rs
```
"#,
    );
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Glob patterns (containing `*`) match any file.
#[test]
fn toc_glob_pattern_matches_files() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file("src/main.rs", "// main\n");
    temp.file("src/utils.rs", "// utils\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
src/
├── *.rs
```
"#,
    );
    // The `*.rs` glob should match the existing .rs files
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Recursive glob patterns (`**/*.ext`) match nested files.
#[test]
fn toc_recursive_glob_pattern_matches_nested() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file("src/utils/helpers.rs", "// helpers\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
src/
├── **/*.rs
```
"#,
    );
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#what-gets-validated
///
/// > Glob patterns fail if no files match.
#[test]
fn toc_glob_pattern_fails_when_no_match() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```
src/
├── *.ts
```
"#,
    );
    // No .ts files exist, so glob should fail
    check("docs").pwd(temp.path()).fails().stdout_has("*.ts");
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

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Strip markdown file's parent directory name prefix resolves relative to parent dir
///
/// Regression test: When a README in `checks/quality/` has a TOC entry like
/// `quality/evaluate.sh`, the StripParentDirName strategy should look for
/// `checks/quality/evaluate.sh`, not `evaluate.sh` at the project root.
#[test]
fn toc_parent_dir_prefix_stripped_resolves_relative_to_parent() {
    let temp = default_project();
    // Create subdirectory structure with files
    temp.file("checks/quality/evaluate.sh", "#!/bin/bash\n");
    temp.file("checks/quality/metrics/loc.sh", "#!/bin/bash\n");
    // README in subdirectory with TOC using parent dir name as prefix
    temp.file(
        "checks/quality/README.md",
        r#"# Quality Checks

```
quality/
├── evaluate.sh
└── metrics/
    └── loc.sh
```
"#,
    );
    // Both files exist at checks/quality/*, should pass
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > TOC with mixed path styles only reports truly missing files
///
/// When a TOC block has entries that resolve with different strategies,
/// only report entries that fail ALL strategies.
#[test]
fn toc_mixed_strategies_only_reports_truly_missing() {
    let temp = default_project();
    // Create files that resolve with different strategies
    temp.file("checks/benchmarks/run.sh", "#!/bin/bash\n");
    temp.file("checks/benchmarks/lib/common.sh", "#!/bin/bash\n");
    // README with TOC using full path from root
    temp.file(
        "checks/benchmarks/README.md",
        r#"# Benchmarks

```
checks/benchmarks/
├── run.sh
├── lib/
│   └── common.sh
└── results/
    └── missing.txt
```
"#,
    );
    // run.sh and common.sh exist (resolve via RelativeToRoot)
    // missing.txt does NOT exist - should be the only violation
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("missing.txt")
        .stdout_lacks("run.sh")
        .stdout_lacks("common.sh");
}

/// Spec: docs/specs/checks/docs.md#resolution
///
/// > Full relative path from root resolves correctly
#[test]
fn toc_full_path_from_root_resolves() {
    let temp = default_project();
    temp.file("checks/benchmarks/run.sh", "#!/bin/bash\n");
    temp.file("checks/benchmarks/README.md", "# Benchmarks\n");
    temp.file(
        "checks/benchmarks/CLAUDE.md",
        r#"# Benchmarks

```
checks/benchmarks/
├── README.md
└── run.sh
```
"#,
    );
    // Both entries use full path from root, should pass
    check("docs").pwd(temp.path()).passes();
}
