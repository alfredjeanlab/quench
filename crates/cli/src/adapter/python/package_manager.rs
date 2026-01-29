// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python package manager detection.
//!
//! Detects package manager from lock files and provides command generation.
//! Detection order (first match wins):
//! 1. `uv.lock` (uv)
//! 2. `poetry.lock` (Poetry)
//! 3. `Pipfile.lock` / `Pipfile` (pipenv)
//! 4. `requirements.txt` or `pyproject.toml` (pip)

use std::path::Path;

/// Python package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackageManager {
    #[default]
    Pip,
    Poetry,
    Uv,
    Pipenv,
}

impl PackageManager {
    /// Detect package manager from lock files in project root.
    pub fn detect(root: &Path) -> Self {
        // uv.lock indicates uv (modern, fast)
        if root.join("uv.lock").exists() {
            return Self::Uv;
        }
        // poetry.lock indicates Poetry
        if root.join("poetry.lock").exists() {
            return Self::Poetry;
        }
        // Pipfile.lock or Pipfile indicates pipenv
        if root.join("Pipfile.lock").exists() || root.join("Pipfile").exists() {
            return Self::Pipenv;
        }
        // Default to pip
        Self::Pip
    }

    /// Package manager executable name.
    pub fn executable(&self) -> &'static str {
        match self {
            PackageManager::Pip => "pip",
            PackageManager::Poetry => "poetry",
            PackageManager::Uv => "uv",
            PackageManager::Pipenv => "pipenv",
        }
    }

    /// Command prefix for running Python tools.
    ///
    /// Returns the command prefix that should be used before tool names.
    /// - pip: no prefix (tools installed globally or in venv)
    /// - poetry: `poetry run`
    /// - uv: `uv run`
    /// - pipenv: `pipenv run`
    pub fn run_prefix(&self) -> Option<Vec<String>> {
        match self {
            PackageManager::Pip => None, // Direct execution
            PackageManager::Poetry => Some(vec!["poetry".into(), "run".into()]),
            PackageManager::Uv => Some(vec!["uv".into(), "run".into()]),
            PackageManager::Pipenv => Some(vec!["pipenv".into(), "run".into()]),
        }
    }
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.executable())
    }
}

// =============================================================================
// TOOL DETECTION
// =============================================================================

/// Detected Python tooling in a project.
#[derive(Debug, Default)]
pub struct PythonTooling {
    /// Package manager detected
    pub package_manager: PackageManager,
    /// Ruff is configured
    pub has_ruff: bool,
    /// Black is configured
    pub has_black: bool,
    /// Mypy is configured
    pub has_mypy: bool,
    /// Flake8 is configured
    pub has_flake8: bool,
    /// Pylint is configured
    pub has_pylint: bool,
    /// Pytest is likely in use
    pub has_pytest: bool,
    /// Build system is configured (can run python -m build)
    pub has_build: bool,
}

impl PythonTooling {
    /// Detect Python tooling from project files.
    pub fn detect(root: &Path) -> Self {
        let package_manager = PackageManager::detect(root);
        let pyproject = root.join("pyproject.toml");
        let pyproject_content = std::fs::read_to_string(&pyproject).unwrap_or_default();

        Self {
            package_manager,
            // Ruff: ruff.toml, .ruff.toml, or [tool.ruff] in pyproject.toml
            has_ruff: root.join("ruff.toml").exists()
                || root.join(".ruff.toml").exists()
                || pyproject_content.contains("[tool.ruff]"),
            // Black: [tool.black] in pyproject.toml
            has_black: pyproject_content.contains("[tool.black]"),
            // Mypy: mypy.ini, .mypy.ini, or [tool.mypy] in pyproject.toml
            has_mypy: root.join("mypy.ini").exists()
                || root.join(".mypy.ini").exists()
                || pyproject_content.contains("[tool.mypy]"),
            // Flake8: .flake8 or setup.cfg with [flake8]
            has_flake8: root.join(".flake8").exists() || has_flake8_in_setup_cfg(root),
            // Pylint: .pylintrc, pylintrc, or [tool.pylint*] in pyproject.toml
            has_pylint: root.join(".pylintrc").exists()
                || root.join("pylintrc").exists()
                || pyproject_content.contains("[tool.pylint"),
            // Pytest: tests/, conftest.py, pytest.ini, or [tool.pytest*] in pyproject.toml
            has_pytest: root.join("tests").is_dir()
                || root.join("conftest.py").exists()
                || root.join("pytest.ini").exists()
                || pyproject_content.contains("[tool.pytest"),
            // Build: pyproject.toml with [build-system] or setup.py
            has_build: pyproject_content.contains("[build-system]")
                || root.join("setup.py").exists(),
        }
    }
}

/// Check if setup.cfg contains [flake8] section.
fn has_flake8_in_setup_cfg(root: &Path) -> bool {
    let setup_cfg = root.join("setup.cfg");
    if setup_cfg.exists()
        && let Ok(content) = std::fs::read_to_string(&setup_cfg)
    {
        return content.contains("[flake8]");
    }
    false
}

#[cfg(test)]
#[path = "package_manager_tests.rs"]
mod tests;
