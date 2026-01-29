// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::TempDir;

fn setup_dir() -> TempDir {
    TempDir::new().unwrap()
}

// =============================================================================
// Package Manager Detection
// =============================================================================

#[test]
fn detects_uv_from_lock_file() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("uv.lock"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Uv);
}

#[test]
fn detects_poetry_from_lock_file() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("poetry.lock"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Poetry);
}

#[test]
fn detects_pipenv_from_pipfile_lock() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("Pipfile.lock"), "{}").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Pipenv);
}

#[test]
fn detects_pipenv_from_pipfile() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("Pipfile"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Pipenv);
}

#[test]
fn defaults_to_pip() {
    let dir = setup_dir();
    // No lock files
    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Pip);
}

#[test]
fn uv_takes_precedence_over_poetry() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("uv.lock"), "").unwrap();
    std::fs::write(dir.path().join("poetry.lock"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Uv);
}

#[test]
fn poetry_takes_precedence_over_pipenv() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("poetry.lock"), "").unwrap();
    std::fs::write(dir.path().join("Pipfile"), "").unwrap();

    assert_eq!(PackageManager::detect(dir.path()), PackageManager::Poetry);
}

// =============================================================================
// Package Manager Methods
// =============================================================================

#[test]
fn executable_returns_correct_name() {
    assert_eq!(PackageManager::Pip.executable(), "pip");
    assert_eq!(PackageManager::Poetry.executable(), "poetry");
    assert_eq!(PackageManager::Uv.executable(), "uv");
    assert_eq!(PackageManager::Pipenv.executable(), "pipenv");
}

#[test]
fn run_prefix_pip_returns_none() {
    assert!(PackageManager::Pip.run_prefix().is_none());
}

#[test]
fn run_prefix_poetry_returns_poetry_run() {
    let prefix = PackageManager::Poetry.run_prefix().unwrap();
    assert_eq!(prefix, vec!["poetry", "run"]);
}

#[test]
fn run_prefix_uv_returns_uv_run() {
    let prefix = PackageManager::Uv.run_prefix().unwrap();
    assert_eq!(prefix, vec!["uv", "run"]);
}

#[test]
fn run_prefix_pipenv_returns_pipenv_run() {
    let prefix = PackageManager::Pipenv.run_prefix().unwrap();
    assert_eq!(prefix, vec!["pipenv", "run"]);
}

#[test]
fn display_returns_executable() {
    assert_eq!(format!("{}", PackageManager::Pip), "pip");
    assert_eq!(format!("{}", PackageManager::Poetry), "poetry");
    assert_eq!(format!("{}", PackageManager::Uv), "uv");
    assert_eq!(format!("{}", PackageManager::Pipenv), "pipenv");
}

// =============================================================================
// Tooling Detection
// =============================================================================

#[test]
fn detects_ruff_from_ruff_toml() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("ruff.toml"), "[lint]\nselect = [\"E\"]").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_ruff);
}

#[test]
fn detects_ruff_from_dot_ruff_toml() {
    let dir = setup_dir();
    std::fs::write(dir.path().join(".ruff.toml"), "[lint]\nselect = [\"E\"]").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_ruff);
}

#[test]
fn detects_ruff_from_pyproject_toml() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.ruff]\nline-length = 88\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_ruff);
}

#[test]
fn detects_black_from_pyproject_toml() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.black]\nline-length = 88\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_black);
}

#[test]
fn detects_mypy_from_mypy_ini() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("mypy.ini"), "[mypy]\nstrict = true").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_mypy);
}

#[test]
fn detects_mypy_from_dot_mypy_ini() {
    let dir = setup_dir();
    std::fs::write(dir.path().join(".mypy.ini"), "[mypy]\nstrict = true").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_mypy);
}

#[test]
fn detects_mypy_from_pyproject_toml() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.mypy]\nstrict = true\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_mypy);
}

#[test]
fn detects_pytest_from_tests_dir() {
    let dir = setup_dir();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pytest);
}

#[test]
fn detects_pytest_from_conftest() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("conftest.py"), "# pytest config").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pytest);
}

#[test]
fn detects_pytest_from_pytest_ini() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("pytest.ini"), "[pytest]").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pytest);
}

#[test]
fn detects_pytest_from_pyproject_toml() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\nminversion = \"6.0\"\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pytest);
}

#[test]
fn detects_build_from_pyproject_build_system() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[build-system]\nrequires = [\"setuptools\"]\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_build);
}

#[test]
fn detects_build_from_setup_py() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("setup.py"), "from setuptools import setup").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_build);
}

#[test]
fn detects_flake8_from_dotfile() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join(".flake8"),
        "[flake8]\nmax-line-length = 100",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_flake8);
}

#[test]
fn detects_flake8_from_setup_cfg() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("setup.cfg"),
        "[flake8]\nmax-line-length = 100",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_flake8);
}

#[test]
fn detects_pylint_from_pylintrc() {
    let dir = setup_dir();
    std::fs::write(dir.path().join(".pylintrc"), "[MESSAGES CONTROL]").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pylint);
}

#[test]
fn detects_pylint_from_pylintrc_without_dot() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("pylintrc"), "[MESSAGES CONTROL]").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pylint);
}

#[test]
fn detects_pylint_from_pyproject_toml() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.pylint.messages_control]\ndisable = \"C0114\"\n",
    )
    .unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_pylint);
}

// =============================================================================
// Tooling Detection with Package Manager
// =============================================================================

#[test]
fn tooling_detects_package_manager() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("poetry.lock"), "").unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert_eq!(tooling.package_manager, PackageManager::Poetry);
}

#[test]
fn tooling_detects_multiple_tools() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::write(dir.path().join("mypy.ini"), "[mypy]").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let tooling = PythonTooling::detect(dir.path());
    assert!(tooling.has_ruff);
    assert!(tooling.has_mypy);
    assert!(tooling.has_pytest);
}
