// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn finds_config_in_current_dir() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("quench.toml");
    fs::write(&config_path, "version = 1\n").unwrap();

    let found = find_config(dir.path());
    assert_eq!(found, Some(config_path));
}

#[test]
fn finds_config_in_parent_dir() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("quench.toml");
    fs::write(&config_path, "version = 1\n").unwrap();

    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let found = find_config(&subdir);
    assert_eq!(found, Some(config_path));
}

#[test]
fn stops_at_git_root() {
    let dir = tempdir().unwrap();

    // Create .git directory (git root marker)
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Create subdir without config
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    // No config anywhere - should return None at git root
    let found = find_config(&subdir);
    assert_eq!(found, None);
}

#[test]
fn finds_config_before_git_root() {
    let dir = tempdir().unwrap();

    // Create .git directory
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Create config at git root
    let config_path = dir.path().join("quench.toml");
    fs::write(&config_path, "version = 1\n").unwrap();

    // Create subdir
    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    let found = find_config(&subdir);
    assert_eq!(found, Some(config_path));
}

#[test]
fn returns_none_when_no_config() {
    let dir = tempdir().unwrap();

    // Create .git directory to stop at
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    let found = find_config(dir.path());
    assert_eq!(found, None);
}
