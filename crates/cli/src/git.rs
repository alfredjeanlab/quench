// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git utilities for change detection.
//!
//! Uses git2 (libgit2) for all git operations to avoid subprocess overhead.
//!
//! ## File Detection
//!
//! When detecting changed files:
//! - Added files: path from `new_file()`
//! - Modified files: path from `new_file()` (same as old)
//! - Renamed files: path from `new_file()` (the new location)
//! - Deleted files: path from `old_file()` (since `new_file()` is empty)

use std::path::{Path, PathBuf};

use anyhow::Context;
use git2::Repository;

/// Extract file path from a diff delta.
///
/// For deleted files, `new_file().path()` is `None`, so fall back to `old_file()`.
/// Order matters: try `new_file` first (works for add, modify, rename, copy),
/// then fall back to `old_file` (needed for delete).
fn extract_path<'a>(delta: &'a git2::DiffDelta<'a>) -> Option<&'a Path> {
    delta.new_file().path().or_else(|| delta.old_file().path())
}

/// A commit with its hash and message.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Short commit hash (7 characters).
    pub hash: String,
    /// Full commit message (subject line only).
    pub message: String,
}

/// Collect commits from a revwalk iterator into a Vec.
fn collect_commits(repo: &Repository, revwalk: git2::Revwalk) -> anyhow::Result<Vec<Commit>> {
    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        commits.push(Commit {
            hash: oid.to_string()[..7].to_string(),
            message: commit.summary().unwrap_or("").to_string(),
        });
    }
    Ok(commits)
}

/// Check if a path is in a git repository.
pub fn is_git_repo(root: &Path) -> bool {
    Repository::discover(root).is_ok()
}

/// Detect base branch for CI mode (main or master).
pub fn detect_base_branch(root: &Path) -> Option<String> {
    let repo = Repository::discover(root).ok()?;

    // Check if main branch exists locally
    if repo.find_branch("main", git2::BranchType::Local).is_ok() {
        return Some("main".to_string());
    }

    // Fall back to master locally
    if repo.find_branch("master", git2::BranchType::Local).is_ok() {
        return Some("master".to_string());
    }

    // Check for remote branches if local don't exist
    for name in ["origin/main", "origin/master"] {
        if repo.revparse_single(name).is_ok() {
            return Some(name.to_string());
        }
    }

    None
}

/// Get commits since a base ref.
///
/// Returns commits from newest to oldest.
pub fn get_commits_since(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    // Resolve base and HEAD
    let base_oid = repo
        .revparse_single(base)
        .with_context(|| format!("Failed to resolve base ref: {}", base))?
        .id();
    let head_oid = repo
        .head()
        .context("Failed to get HEAD")?
        .target()
        .ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;

    // Walk commits from HEAD, stopping at base
    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_oid)?;
    revwalk.hide(base_oid)?;

    collect_commits(&repo, revwalk)
}

/// Get all commits on current branch (for CI mode).
pub fn get_all_branch_commits(root: &Path) -> anyhow::Result<Vec<Commit>> {
    if let Some(base) = detect_base_branch(root) {
        get_commits_since(root, &base)
    } else {
        // No base branch found, get all commits
        let repo = Repository::discover(root).context("Failed to open repository")?;
        let head_oid = repo
            .head()
            .context("Failed to get HEAD")?
            .target()
            .ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;

        collect_commits(&repo, revwalk)
    }
}

/// Get list of changed files compared to a git base ref.
///
/// Combines committed, staged, and unstaged changes using git2 diff operations.
pub fn get_changed_files(root: &Path, base: &str) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    // Resolve base to a tree
    let base_tree = repo
        .revparse_single(base)
        .with_context(|| format!("Failed to resolve base ref: {}", base))?
        .peel_to_tree()
        .context("Failed to get tree for base ref")?;

    // Get HEAD tree
    let head_tree = repo.head()?.peel_to_tree()?;

    // Get index for staged changes
    let index = repo.index()?;

    let mut files = std::collections::HashSet::new();

    // Compare HEAD to base (committed changes on branch)
    let head_diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    for delta in head_diff.deltas() {
        if let Some(path) = extract_path(&delta) {
            files.insert(root.join(path));
        }
    }

    // Compare index to base (staged changes)
    let index_diff = repo.diff_tree_to_index(Some(&base_tree), Some(&index), None)?;
    for delta in index_diff.deltas() {
        if let Some(path) = extract_path(&delta) {
            files.insert(root.join(path));
        }
    }

    // Compare workdir to index (unstaged changes)
    let workdir_diff = repo.diff_index_to_workdir(Some(&index), None)?;
    for delta in workdir_diff.deltas() {
        if let Some(path) = extract_path(&delta) {
            files.insert(root.join(path));
        }
    }

    Ok(files.into_iter().collect())
}

/// Get list of staged files (for --staged flag).
///
/// Uses git2 to compare the index against HEAD to find staged changes.
pub fn get_staged_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    // Get HEAD tree (handle case of empty repo with no commits)
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree().context("Failed to get HEAD tree")?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e).context("Failed to get HEAD"),
    };

    let index = repo.index().context("Failed to get repository index")?;

    // Compare HEAD tree to index to find staged changes
    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), Some(&index), None)
        .context("Failed to compute diff")?;

    let mut files = Vec::new();
    for delta in diff.deltas() {
        if let Some(path) = extract_path(&delta) {
            files.push(root.join(path));
        }
    }

    Ok(files)
}

/// Save content to git notes for HEAD commit.
///
/// Uses `refs/notes/quench` namespace to avoid conflicts with other tools.
pub fn save_to_git_notes(root: &Path, content: &str) -> anyhow::Result<()> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    let head_commit = repo
        .head()
        .context("Failed to get HEAD")?
        .peel_to_commit()
        .context("HEAD is not a commit")?;

    let sig = repo.signature().or_else(|_| git2::Signature::now("quench", "quench@local"))?;

    // Get or create notes ref
    let notes_ref = "refs/notes/quench";

    // Add note (this handles creating the ref if needed)
    repo.note(
        &sig,            // author
        &sig,            // committer
        Some(notes_ref), // ref name
        head_commit.id(),
        content,
        true, // overwrite existing notes
    )?;

    Ok(())
}

/// Read git note for a specific commit.
pub fn read_git_note(root: &Path, commit_ref: &str) -> anyhow::Result<Option<String>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    let commit = repo
        .revparse_single(commit_ref)
        .context("Failed to resolve commit ref")?
        .peel_to_commit()
        .context("Ref is not a commit")?;

    let notes_ref = "refs/notes/quench";

    match repo.find_note(Some(notes_ref), commit.id()) {
        Ok(note) => Ok(note.message().map(|s| s.to_string())),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e).context("Failed to read git note"),
    }
}

/// Find the merge-base commit for ratchet comparison.
///
/// If base_ref is provided, uses that ref.
/// Otherwise, finds merge-base with main/master.
/// Falls back to HEAD~1 if no merge-base found.
pub fn find_ratchet_base(root: &Path, base_ref: Option<&str>) -> anyhow::Result<String> {
    let repo = Repository::discover(root)?;

    if let Some(base_ref) = base_ref {
        // Explicit --base REF provided
        let commit = repo.revparse_single(base_ref)?.peel_to_commit()?;
        return Ok(commit.id().to_string());
    }

    // Try to find merge-base with main branch
    let head = repo.head()?.peel_to_commit()?;

    for main_branch in &["origin/main", "origin/master", "main", "master"] {
        if let Ok(main_ref) = repo.revparse_single(main_branch)
            && let Ok(main_commit) = main_ref.peel_to_commit()
            && let Ok(base) = repo.merge_base(head.id(), main_commit.id())
        {
            return Ok(base.to_string());
        }
    }

    // Fallback: use HEAD~1 (parent commit)
    if let Ok(parent) = head.parent(0) {
        return Ok(parent.id().to_string());
    }

    // Last resort: HEAD itself (initial commit)
    Ok(head.id().to_string())
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;
