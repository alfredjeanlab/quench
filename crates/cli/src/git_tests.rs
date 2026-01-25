// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for git utilities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

use tempfile::TempDir;

use super::*;

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Initialize a git repository in the temp directory.
fn init_git_repo(temp: &TempDir) {
    Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to init git repo");

    // Configure user for commits
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to configure git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to configure git name");
}

/// Stage a file using git add.
fn git_add(temp: &TempDir, file: &str) {
    Command::new("git")
        .args(["add", file])
        .current_dir(temp.path())
        .output()
        .expect("Failed to git add");
}

/// Create a commit with the given message.
fn git_commit(temp: &TempDir, message: &str) {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(temp.path())
        .output()
        .expect("Failed to git commit");
}

/// Create and checkout a new branch.
fn git_checkout_b(temp: &TempDir, branch: &str) {
    Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(temp.path())
        .output()
        .expect("Failed to create branch");
}

/// Create an initial commit with a README file.
fn create_initial_commit(temp: &TempDir) {
    std::fs::write(temp.path().join("README.md"), "# Project\n").unwrap();
    git_add(temp, "README.md");
    git_commit(temp, "chore: initial commit");
}

// =============================================================================
// GET_STAGED_FILES TESTS
// =============================================================================

#[test]
fn get_staged_files_empty_staging() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    let files = get_staged_files(temp.path()).unwrap();
    assert!(files.is_empty(), "Expected no staged files");
}

#[test]
fn get_staged_files_with_staged_file() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create and stage a file
    std::fs::write(temp.path().join("test.txt"), "content").unwrap();
    git_add(&temp, "test.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.txt"));
}

#[test]
fn get_staged_files_multiple_staged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create and stage multiple files
    std::fs::write(temp.path().join("a.txt"), "a").unwrap();
    std::fs::write(temp.path().join("b.txt"), "b").unwrap();
    git_add(&temp, "a.txt");
    git_add(&temp, "b.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 2);
}

#[test]
fn get_staged_files_ignores_unstaged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create file but don't stage it
    std::fs::write(temp.path().join("unstaged.txt"), "content").unwrap();

    let files = get_staged_files(temp.path()).unwrap();
    assert!(files.is_empty(), "Unstaged files should not be included");
}

#[test]
fn get_staged_files_in_subdirectory() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create and stage a file in a subdirectory
    std::fs::create_dir(temp.path().join("subdir")).unwrap();
    std::fs::write(temp.path().join("subdir/nested.txt"), "content").unwrap();
    git_add(&temp, "subdir/nested.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("subdir/nested.txt"));
}

#[test]
fn get_staged_files_new_repo_no_commits() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    // Stage a file in a repo with no commits yet
    std::fs::write(temp.path().join("first.txt"), "content").unwrap();
    git_add(&temp, "first.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("first.txt"));
}

// =============================================================================
// GET_CHANGED_FILES TESTS
// =============================================================================

#[test]
fn get_changed_files_includes_committed() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create new branch with changes
    git_checkout_b(&temp, "feature");
    std::fs::write(temp.path().join("new.txt"), "content").unwrap();
    git_add(&temp, "new.txt");
    git_commit(&temp, "feat: add new file");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("new.txt"));
}

#[test]
fn get_changed_files_includes_staged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create new branch with staged changes
    git_checkout_b(&temp, "feature");
    std::fs::write(temp.path().join("staged.txt"), "content").unwrap();
    git_add(&temp, "staged.txt");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("staged.txt"));
}

#[test]
fn get_changed_files_includes_unstaged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create new branch with unstaged changes to tracked file
    git_checkout_b(&temp, "feature");

    // Modify the existing README.md
    std::fs::write(temp.path().join("README.md"), "# Modified\n").unwrap();

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("README.md"));
}

#[test]
fn get_changed_files_combines_all_changes() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create new branch
    git_checkout_b(&temp, "feature");

    // Add a committed file
    std::fs::write(temp.path().join("committed.txt"), "content").unwrap();
    git_add(&temp, "committed.txt");
    git_commit(&temp, "feat: add committed file");

    // Add a staged file
    std::fs::write(temp.path().join("staged.txt"), "content").unwrap();
    git_add(&temp, "staged.txt");

    // Modify an existing file (unstaged)
    std::fs::write(temp.path().join("README.md"), "# Modified\n").unwrap();

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 3);
}

#[test]
fn get_changed_files_no_changes() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create branch with no changes
    git_checkout_b(&temp, "feature");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert!(files.is_empty());
}

#[test]
fn get_changed_files_invalid_base_ref() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    let result = get_changed_files(temp.path(), "nonexistent");
    assert!(result.is_err());
}

// =============================================================================
// IS_GIT_REPO TESTS
// =============================================================================

#[test]
fn is_git_repo_returns_true_for_repo() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    assert!(is_git_repo(temp.path()));
}

#[test]
fn is_git_repo_returns_false_for_non_repo() {
    let temp = TempDir::new().unwrap();

    assert!(!is_git_repo(temp.path()));
}
