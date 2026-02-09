// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use super::*;
use tempfile::tempdir;

// === Index File Detection ===

#[test]
fn detects_claude_md_first() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/CLAUDE.md"), "# Index").unwrap();
    std::fs::write(temp.path().join("docs/specs/overview.md"), "# Overview").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/specs/CLAUDE.md")),
        "CLAUDE.md should have highest priority"
    );
}

#[test]
fn detects_numbered_overview() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/00-overview.md"), "# Overview").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/specs/00-overview.md")),
        "00-overview.md should be detected"
    );
}

#[test]
fn detects_overview_md() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/overview.md"), "# Overview").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/specs/overview.md")),
        "overview.md should be detected"
    );
}

#[test]
fn detects_index_md() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/index.md"), "# Index").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(result, Some(PathBuf::from("docs/specs/index.md")), "index.md should be detected");
}

#[test]
fn detects_fixed_docs_claude_md() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::create_dir_all(temp.path().join("docs")).unwrap();
    std::fs::write(temp.path().join("docs/CLAUDE.md"), "# Docs Index").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/CLAUDE.md")),
        "docs/CLAUDE.md should be detected as fallback"
    );
}

#[test]
fn detects_specifications_md() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/SPECIFICATIONS.md"), "# Specs").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/SPECIFICATIONS.md")),
        "docs/SPECIFICATIONS.md should be detected"
    );
}

#[test]
fn returns_none_when_no_index() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/orphan.md"), "# Orphan").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(result, None, "Should return None when no index file exists");
}

#[test]
fn priority_order_claude_over_overview() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs")).unwrap();
    std::fs::write(temp.path().join("docs/specs/CLAUDE.md"), "# Claude").unwrap();
    std::fs::write(temp.path().join("docs/specs/overview.md"), "# Overview").unwrap();
    std::fs::write(temp.path().join("docs/specs/index.md"), "# Index").unwrap();

    let result = detect_index_file(temp.path(), "docs/specs");
    assert_eq!(
        result,
        Some(PathBuf::from("docs/specs/CLAUDE.md")),
        "CLAUDE.md should take priority over overview.md and index.md"
    );
}

// === Extension Matching ===

#[test]
fn matches_md_extension_with_dot() {
    assert!(matches_extension(Path::new("foo.md"), ".md"));
    assert!(matches_extension(Path::new("path/to/bar.md"), ".md"));
}

#[test]
fn matches_md_extension_without_dot() {
    assert!(matches_extension(Path::new("foo.md"), "md"));
    assert!(matches_extension(Path::new("path/to/bar.md"), "md"));
}

#[test]
fn case_insensitive_extension() {
    assert!(matches_extension(Path::new("foo.MD"), ".md"));
    assert!(matches_extension(Path::new("foo.Md"), "md"));
}

#[test]
fn rejects_wrong_extension() {
    assert!(!matches_extension(Path::new("foo.txt"), ".md"));
    assert!(!matches_extension(Path::new("foo.rs"), "md"));
}

#[test]
fn handles_no_extension() {
    assert!(!matches_extension(Path::new("README"), ".md"));
    assert!(!matches_extension(Path::new("Makefile"), "md"));
}

// === Spec File Counting ===

#[test]
fn counts_spec_files() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("docs/specs/sub")).unwrap();
    std::fs::write(temp.path().join("docs/specs/CLAUDE.md"), "# Index").unwrap();
    std::fs::write(temp.path().join("docs/specs/feature.md"), "# Feature").unwrap();
    std::fs::write(temp.path().join("docs/specs/sub/nested.md"), "# Nested").unwrap();
    std::fs::write(temp.path().join("docs/specs/readme.txt"), "Not a spec").unwrap();

    let count = count_spec_files(temp.path(), "docs/specs", ".md");
    assert_eq!(count, 3, "Should count only .md files");
}

#[test]
fn returns_zero_for_missing_dir() {
    let temp = tempdir().unwrap();
    let count = count_spec_files(temp.path(), "docs/specs", ".md");
    assert_eq!(count, 0, "Should return 0 for non-existent directory");
}
