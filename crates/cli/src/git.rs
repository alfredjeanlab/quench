// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git utilities for change detection.
//!
//! Uses git2 (libgit2) for performance-critical operations to avoid subprocess overhead.
//! Subprocess calls are retained for diff operations which are less frequently called.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use git2::Repository;

/// A commit with its hash and message.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Short commit hash (7 characters).
    pub hash: String,
    /// Full commit message (subject line only).
    pub message: String,
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

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        // Short hash (7 chars)
        let hash = oid.to_string()[..7].to_string();

        // Subject line only (first line of message)
        let message = commit.summary().unwrap_or("").to_string();

        commits.push(Commit { hash, message });
    }

    Ok(commits)
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

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            let hash = oid.to_string()[..7].to_string();
            let message = commit.summary().unwrap_or("").to_string();
            commits.push(Commit { hash, message });
        }

        Ok(commits)
    }
}

// =============================================================================
// Subprocess-based functions (retained for diff operations)
// =============================================================================

/// Get list of changed files compared to a git base ref.
pub fn get_changed_files(root: &Path, base: &str) -> anyhow::Result<Vec<PathBuf>> {
    // Get staged/unstaged changes (diffstat against base)
    let output = Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr.trim());
    }

    // Also get staged changes
    let staged_output = Command::new("git")
        .args(["diff", "--name-only", "--cached", base])
        .current_dir(root)
        .output()?;

    let mut files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.is_empty() {
            files.insert(root.join(line));
        }
    }

    if staged_output.status.success() {
        for line in String::from_utf8_lossy(&staged_output.stdout).lines() {
            if !line.is_empty() {
                files.insert(root.join(line));
            }
        }
    }

    Ok(files.into_iter().collect())
}

/// Get list of staged files (for --staged flag).
pub fn get_staged_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    // Get staged changes
    let output = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff --cached failed: {}", stderr.trim());
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.is_empty() {
            files.push(root.join(line));
        }
    }

    Ok(files)
}

// =============================================================================
// Subprocess fallback functions (kept for debugging/comparison)
// =============================================================================

/// Get commits since base (subprocess fallback).
// KEEP UNTIL: git2 integration is proven stable; useful for debugging/comparison
#[allow(dead_code)]
fn get_commits_since_subprocess(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    let output = Command::new("git")
        .args([
            "log",
            "--format=%h%n%s", // Short hash, newline, subject
            &format!("{}..HEAD", base),
        ])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr.trim());
    }

    parse_git_log_output(&String::from_utf8_lossy(&output.stdout))
}

/// Parse git log output with format "%h%n%s".
// KEEP UNTIL: git2 integration is proven stable; used by subprocess fallback
#[allow(dead_code)]
fn parse_git_log_output(output: &str) -> anyhow::Result<Vec<Commit>> {
    let lines: Vec<&str> = output.lines().collect();
    let mut commits = Vec::new();

    // Process pairs of lines (hash, message)
    for chunk in lines.chunks(2) {
        if chunk.len() == 2 && !chunk[0].is_empty() {
            commits.push(Commit {
                hash: chunk[0].to_string(),
                message: chunk[1].to_string(),
            });
        }
    }

    Ok(commits)
}
