// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for git diff parsing.

use super::*;

#[test]
fn parse_numstat_basic() {
    let output = "10\t5\tsrc/parser.rs
3\t1\tsrc/lexer.rs";

    let result = parse_numstat(output);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, PathBuf::from("src/parser.rs"));
    assert_eq!(result[0].1, 10); // added
    assert_eq!(result[0].2, 5); // deleted
    assert_eq!(result[1].0, PathBuf::from("src/lexer.rs"));
    assert_eq!(result[1].1, 3);
    assert_eq!(result[1].2, 1);
}

#[test]
fn parse_numstat_binary_files() {
    let output = "-\t-\timage.png
10\t5\tsrc/parser.rs";

    let result = parse_numstat(output);
    assert_eq!(result.len(), 2);
    // Binary files have 0 for counts
    assert_eq!(result[0].0, PathBuf::from("image.png"));
    assert_eq!(result[0].1, 0);
    assert_eq!(result[0].2, 0);
}

#[test]
fn parse_numstat_empty() {
    let output = "";
    let result = parse_numstat(output);
    assert!(result.is_empty());
}

#[test]
fn parse_name_status_basic() {
    let output = "A\tsrc/new_file.rs
M\tsrc/parser.rs
D\tsrc/old_file.rs";

    let result = parse_name_status(output);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].0, PathBuf::from("src/new_file.rs"));
    assert_eq!(result[0].1, ChangeType::Added);
    assert_eq!(result[1].0, PathBuf::from("src/parser.rs"));
    assert_eq!(result[1].1, ChangeType::Modified);
    assert_eq!(result[2].0, PathBuf::from("src/old_file.rs"));
    assert_eq!(result[2].1, ChangeType::Deleted);
}

#[test]
fn parse_name_status_renamed() {
    let output = "R100\told_name.rs\tnew_name.rs";

    let result = parse_name_status(output);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, PathBuf::from("new_name.rs"));
    assert_eq!(result[0].1, ChangeType::Modified);
}

#[test]
fn parse_name_status_empty() {
    let output = "";
    let result = parse_name_status(output);
    assert!(result.is_empty());
}

#[test]
fn file_change_lines_changed() {
    let change = FileChange {
        path: PathBuf::from("src/test.rs"),
        change_type: ChangeType::Modified,
        lines_added: 10,
        lines_deleted: 5,
    };

    assert_eq!(change.lines_changed(), 15);
}

#[test]
fn merge_outputs_combines_data() {
    let numstat = "10\t5\tsrc/parser.rs";
    let name_status = "M\tsrc/parser.rs";
    let root = Path::new("/project");

    let result = merge_diff_outputs(numstat, name_status, root).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, PathBuf::from("/project/src/parser.rs"));
    assert_eq!(result[0].change_type, ChangeType::Modified);
    assert_eq!(result[0].lines_added, 10);
    assert_eq!(result[0].lines_deleted, 5);
}

#[test]
fn merge_outputs_handles_missing_counts() {
    // File appears in name-status but not numstat (e.g., binary)
    let numstat = "";
    let name_status = "A\tsrc/image.png";
    let root = Path::new("/project");

    let result = merge_diff_outputs(numstat, name_status, root).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].lines_added, 0);
    assert_eq!(result[0].lines_deleted, 0);
}

// =============================================================================
// COMMIT CHANGES TESTS
// =============================================================================

#[test]
fn commit_changes_struct_fields() {
    let changes = CommitChanges {
        hash: "abc123def456".to_string(),
        message: "feat: add parser".to_string(),
        changes: vec![FileChange {
            path: PathBuf::from("src/parser.rs"),
            change_type: ChangeType::Added,
            lines_added: 50,
            lines_deleted: 0,
        }],
    };

    assert_eq!(changes.hash, "abc123def456");
    assert_eq!(changes.message, "feat: add parser");
    assert_eq!(changes.changes.len(), 1);
    assert_eq!(changes.changes[0].lines_added, 50);
}

// =============================================================================
// INITIAL COMMIT HANDLING TESTS
// =============================================================================

#[test]
fn get_commit_changes_handles_initial_commit() {
    use std::fs;
    use tempfile::TempDir;

    // Create a temp repo with single initial commit
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Initialize repo
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(root)
        .output()
        .expect("git init");

    // Configure git for the test
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(root)
        .output()
        .expect("git config email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(root)
        .output()
        .expect("git config name");

    // Create and commit a file
    fs::write(root.join("test.txt"), "hello").unwrap();

    std::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(root)
        .output()
        .expect("git add");

    std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(root)
        .output()
        .expect("git commit");

    // Get the commit hash
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("git rev-parse");
    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Test that get_commit_changes works for initial commit
    let result = get_commit_changes(root, &hash);
    assert!(result.is_ok(), "Should handle initial commit: {:?}", result);

    let changes = result.unwrap();
    assert_eq!(changes.len(), 1);
    assert!(changes[0].path.to_string_lossy().contains("test.txt"));
    assert_eq!(changes[0].change_type, ChangeType::Added);
}
