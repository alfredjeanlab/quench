// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::TempDir;

fn setup_dir() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn detects_bun_from_bun_lock() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("bun.lock"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Bun);
}

#[test]
fn detects_bun_from_bun_lockb() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("bun.lockb"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Bun);
}

#[test]
fn detects_pnpm_from_lock_file() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Pnpm);
}

#[test]
fn detects_yarn_from_lock_file() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Yarn);
}

#[test]
fn detects_npm_from_package_lock() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Npm);
}

#[test]
fn defaults_to_npm_when_no_lock_file() {
    let dir = setup_dir();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Npm);
}

#[test]
fn bun_takes_priority_over_other_lock_files() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("bun.lock"), "").unwrap();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Bun);
}

#[test]
fn pnpm_takes_priority_over_yarn_and_npm() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Pnpm);
}

#[test]
fn yarn_takes_priority_over_npm() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Yarn);
}

#[test]
fn executable_returns_correct_names() {
    assert_eq!(PackageManager::Npm.executable(), "npm");
    assert_eq!(PackageManager::Pnpm.executable(), "pnpm");
    assert_eq!(PackageManager::Yarn.executable(), "yarn");
    assert_eq!(PackageManager::Bun.executable(), "bun");
}

#[test]
fn run_command_generates_correct_args() {
    assert_eq!(PackageManager::Npm.run_command("build"), vec!["npm", "run", "build"]);
    assert_eq!(PackageManager::Pnpm.run_command("build"), vec!["pnpm", "run", "build"]);
    // Yarn doesn't need "run"
    assert_eq!(PackageManager::Yarn.run_command("build"), vec!["yarn", "build"]);
    assert_eq!(PackageManager::Bun.run_command("build"), vec!["bun", "run", "build"]);
}

#[test]
fn test_command_generates_correct_args() {
    assert_eq!(PackageManager::Npm.test_command(), vec!["npm", "test"]);
    assert_eq!(PackageManager::Pnpm.test_command(), vec!["pnpm", "test"]);
    assert_eq!(PackageManager::Yarn.test_command(), vec!["yarn", "test"]);
    assert_eq!(PackageManager::Bun.test_command(), vec!["bun", "test"]);
}

#[test]
fn exec_command_generates_correct_args() {
    assert_eq!(PackageManager::Npm.exec_command(), vec!["npx"]);
    assert_eq!(PackageManager::Pnpm.exec_command(), vec!["pnpm", "exec"]);
    assert_eq!(PackageManager::Yarn.exec_command(), vec!["yarn"]);
    assert_eq!(PackageManager::Bun.exec_command(), vec!["bunx"]);
}

#[test]
fn display_shows_executable() {
    assert_eq!(format!("{}", PackageManager::Npm), "npm");
    assert_eq!(format!("{}", PackageManager::Bun), "bun");
}

#[test]
fn default_is_npm() {
    assert_eq!(PackageManager::default(), PackageManager::Npm);
}
