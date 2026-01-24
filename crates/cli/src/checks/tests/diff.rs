// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git diff parsing for change detection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Type of change detected in git diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

/// A file change detected from git diff.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub lines_added: usize,
    pub lines_deleted: usize,
}

impl FileChange {
    /// Total lines changed (added + deleted).
    pub fn lines_changed(&self) -> usize {
        self.lines_added + self.lines_deleted
    }
}

/// Get changed files from staged area (--staged flag).
pub fn get_staged_changes(root: &Path) -> Result<Vec<FileChange>, String> {
    let numstat = run_git_diff(root, &["--cached", "--numstat"])?;
    let name_status = run_git_diff(root, &["--cached", "--name-status"])?;

    merge_diff_outputs(&numstat, &name_status, root)
}

/// Get changed files comparing to base ref (--base flag).
pub fn get_base_changes(root: &Path, base: &str) -> Result<Vec<FileChange>, String> {
    let range = format!("{}..HEAD", base);
    let numstat = run_git_diff(root, &["--numstat", &range])?;
    let name_status = run_git_diff(root, &["--name-status", &range])?;

    merge_diff_outputs(&numstat, &name_status, root)
}

/// Run a git diff command with the given arguments.
pub fn run_git_diff(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("diff").args(args).current_dir(root);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Merge numstat and name-status outputs into FileChange structs.
fn merge_diff_outputs(
    numstat: &str,
    name_status: &str,
    root: &Path,
) -> Result<Vec<FileChange>, String> {
    let line_counts = parse_numstat(numstat);
    let change_types = parse_name_status(name_status);

    let mut result = Vec::new();

    // Create a map of path -> (lines_added, lines_deleted)
    let counts_map: HashMap<PathBuf, (usize, usize)> = line_counts
        .into_iter()
        .map(|(path, added, deleted)| (path, (added, deleted)))
        .collect();

    for (path, change_type) in change_types {
        let (lines_added, lines_deleted) = counts_map.get(&path).copied().unwrap_or((0, 0));
        result.push(FileChange {
            path: root.join(&path),
            change_type,
            lines_added,
            lines_deleted,
        });
    }

    Ok(result)
}

/// Parse git diff --numstat output into line counts.
///
/// Format: `<added>\t<deleted>\t<path>`
/// Binary files show `-` for counts.
fn parse_numstat(output: &str) -> Vec<(PathBuf, usize, usize)> {
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                return None;
            }

            // Handle binary files (shown as -)
            let added = parts[0].parse().unwrap_or(0);
            let deleted = parts[1].parse().unwrap_or(0);

            // Handle renamed files: path may contain arrow
            let path = if parts.len() > 3 {
                // Renamed file: old -> new
                parts[3].trim()
            } else {
                parts[2].trim()
            };

            Some((PathBuf::from(path), added, deleted))
        })
        .collect()
}

/// Parse git diff --name-status output into change types.
///
/// Format: `<status>\t<path>` or `<status>\t<old>\t<new>` for renames
fn parse_name_status(output: &str) -> Vec<(PathBuf, ChangeType)> {
    output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() {
                return None;
            }

            let status = parts[0].chars().next()?;
            let change_type = match status {
                'A' => ChangeType::Added,
                'M' => ChangeType::Modified,
                'D' => ChangeType::Deleted,
                'R' | 'C' => ChangeType::Modified, // Renamed/Copied treated as modified
                'T' => ChangeType::Modified,       // Type change
                _ => return None,                  // Skip unknown statuses
            };

            // For renames (R100), use the new path (second path)
            let path = if parts.len() >= 3 && (status == 'R' || status == 'C') {
                parts[2].trim()
            } else if parts.len() >= 2 {
                parts[1].trim()
            } else {
                return None;
            };

            Some((PathBuf::from(path), change_type))
        })
        .collect()
}

/// A single commit's changes.
#[derive(Debug, Clone)]
pub struct CommitChanges {
    /// Commit hash (full SHA).
    pub hash: String,
    /// Commit message (first line).
    pub message: String,
    /// Files changed in this commit.
    pub changes: Vec<FileChange>,
}

/// Get changes per commit from base..HEAD.
///
/// Returns commits in chronological order (oldest first).
pub fn get_commits_since(root: &Path, base: &str) -> Result<Vec<CommitChanges>, String> {
    // Get list of commits in the range
    let output = Command::new("git")
        .args(["log", "--format=%H|%s", &format!("{}..HEAD", base)])
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed: {}", stderr.trim()));
    }

    // Parse commit lines (newest first from git log)
    let commits: Vec<(String, String)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    // Get changes for each commit (reverse to get oldest first)
    let mut result = Vec::new();
    for (hash, message) in commits.into_iter().rev() {
        let changes = get_commit_changes(root, &hash)?;
        result.push(CommitChanges {
            hash,
            message,
            changes,
        });
    }

    Ok(result)
}

/// Get file changes for a specific commit.
fn get_commit_changes(root: &Path, commit_hash: &str) -> Result<Vec<FileChange>, String> {
    // Use hash^..hash to get changes in that specific commit
    // Note: This won't work for the initial commit, but for feature branches
    // comparing to main, we don't need to handle the initial commit.
    let range = format!("{}^..{}", commit_hash, commit_hash);

    let numstat = run_git_diff(root, &["--numstat", &range])?;
    let name_status = run_git_diff(root, &["--name-status", &range])?;

    merge_diff_outputs(&numstat, &name_status, root)
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
