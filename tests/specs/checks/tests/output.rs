// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Output format specs for tests check.
//!
//! Reference: docs/specs/checks/tests.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use std::process::Command;

/// Initialize a git repo with user config and initial commit.
fn init_git_repo(path: &std::path::Path) {
    Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit", "--allow-empty"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Stage files without committing.
fn git_stage(path: &std::path::Path) {
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
}

/// Add and commit all changes.
fn git_commit(path: &std::path::Path, msg: &str) {
    Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
    Command::new("git").args(["commit", "-m", msg]).current_dir(path).output().unwrap();
}

/// Create a feature branch.
fn git_branch(path: &std::path::Path, name: &str) {
    Command::new("git").args(["checkout", "-b", name]).current_dir(path).output().unwrap();
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: Text output format for missing_tests violation in staged mode
#[test]
fn tests_text_output_missing_tests_staged() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}");
    git_stage(temp.path());

    check("tests").pwd(temp.path()).args(&["--staged"]).fails().stdout_eq(
        "tests: FAIL
  src/feature.rs: missing_tests
    Add tests in tests/feature.rs or a sibling feature_tests.rs file
FAIL: tests
",
    );
}

/// Spec: Text output format for branch mode with multiple violations
#[test]
fn tests_text_output_missing_tests_branch() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );
    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/test");

    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add parser and lexer");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("tests: FAIL")
        .stdout_has("src/parser.rs: missing_tests")
        .stdout_has("src/lexer.rs: missing_tests");
}

/// Spec: JSON output includes change_type and lines_changed
#[test]
fn tests_json_output_violation_structure() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}\npub fn more() {}");
    git_stage(temp.path());

    let result = check("tests").pwd(temp.path()).args(&["--staged"]).json().fails();

    let violations = result.violations();
    assert_eq!(violations.len(), 1);

    let v = &violations[0];
    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("missing_tests"));
    assert_eq!(v.get("change_type").and_then(|v| v.as_str()), Some("added"));
    assert_eq!(v.get("lines_changed").and_then(|v| v.as_i64()), Some(2));
}

/// Spec: Text output passes when tests exist
#[test]
fn tests_text_output_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}");
    temp.file("tests/feature_tests.rs", "#[test] fn t() {}");
    git_stage(temp.path());

    check("tests").pwd(temp.path()).args(&["--staged"]).passes();
}
