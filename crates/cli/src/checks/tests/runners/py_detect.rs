// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python test runner auto-detection.
//!
//! Detection priority (first match wins):
//! 1. Config files (pytest.ini, setup.cfg with [tool:pytest])
//! 2. pyproject.toml [tool.pytest] section
//! 3. conftest.py presence
//! 4. Test file patterns (test_*.py suggests pytest style)
//! 5. Fallback to unittest (always available with Python)

use std::path::Path;
use std::process::{Command, Stdio};

/// Detected Python test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyRunner {
    Pytest,
    Unittest,
}

impl PyRunner {
    /// Convert to runner name string used in TestSuiteConfig.
    pub fn name(&self) -> &'static str {
        match self {
            PyRunner::Pytest => "pytest",
            PyRunner::Unittest => "unittest",
        }
    }
}

/// Detection result with confidence signal.
#[derive(Debug)]
pub struct PyDetectionResult {
    pub runner: PyRunner,
    pub source: PyDetectionSource,
}

/// How the runner was detected.
#[derive(Debug)]
pub enum PyDetectionSource {
    /// Detected from a config file (e.g., "pytest.ini").
    ConfigFile(String),
    /// Detected from pyproject.toml section (e.g., "[tool.pytest.ini_options]").
    PyprojectSection(String),
    /// Detected from conftest.py presence.
    Conftest,
    /// Detected from test file patterns (pytest style test_*.py).
    TestFilePattern,
    /// Fallback to unittest (default Python test framework).
    Fallback,
}

impl PyDetectionSource {
    /// Convert to a string for metrics.
    pub fn to_metric_string(&self) -> String {
        match self {
            PyDetectionSource::ConfigFile(name) => format!("config_file:{}", name),
            PyDetectionSource::PyprojectSection(section) => {
                format!("pyproject_section:{}", section)
            }
            PyDetectionSource::Conftest => "conftest".to_string(),
            PyDetectionSource::TestFilePattern => "test_file_pattern".to_string(),
            PyDetectionSource::Fallback => "fallback".to_string(),
        }
    }
}

/// Detect Python test runner for a project.
///
/// Returns None if no Python project markers are found.
pub fn detect_py_runner(root: &Path) -> Option<PyDetectionResult> {
    // First check if this is a Python project
    if !is_python_project(root) {
        return None;
    }

    // 1. Check config files (highest priority)
    if let Some(result) = detect_from_config_files(root) {
        return Some(result);
    }

    // 2. Check pyproject.toml for pytest configuration
    if let Some(result) = detect_from_pyproject(root) {
        return Some(result);
    }

    // 3. Check for conftest.py (pytest-specific)
    if let Some(result) = detect_from_conftest(root) {
        return Some(result);
    }

    // 4. Check for pytest-style test files
    if let Some(result) = detect_from_test_patterns(root) {
        return Some(result);
    }

    // 5. Fallback to unittest if pytest is not available
    Some(PyDetectionResult {
        runner: if is_pytest_available() {
            PyRunner::Pytest
        } else {
            PyRunner::Unittest
        },
        source: PyDetectionSource::Fallback,
    })
}

/// Check if this is a Python project.
fn is_python_project(root: &Path) -> bool {
    // Check for common Python project markers
    root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("setup.cfg").exists()
        || root.join("requirements.txt").exists()
        || root.join("Pipfile").exists()
        || root.join("poetry.lock").exists()
        || root.join("uv.lock").exists()
}

/// Detect test runner from pytest-specific config files.
fn detect_from_config_files(root: &Path) -> Option<PyDetectionResult> {
    // pytest.ini is definitive
    if root.join("pytest.ini").exists() {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::ConfigFile("pytest.ini".to_string()),
        });
    }

    // Check setup.cfg for [tool:pytest] section
    let setup_cfg = root.join("setup.cfg");
    if setup_cfg.exists()
        && let Ok(content) = std::fs::read_to_string(&setup_cfg)
        && (content.contains("[tool:pytest]") || content.contains("[pytest]"))
    {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::ConfigFile("setup.cfg".to_string()),
        });
    }

    // tox.ini can indicate pytest usage
    let tox_ini = root.join("tox.ini");
    if tox_ini.exists()
        && let Ok(content) = std::fs::read_to_string(&tox_ini)
        && (content.contains("[pytest]") || content.contains("pytest"))
    {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::ConfigFile("tox.ini".to_string()),
        });
    }

    None
}

/// Detect test runner from pyproject.toml.
fn detect_from_pyproject(root: &Path) -> Option<PyDetectionResult> {
    let pyproject = root.join("pyproject.toml");
    if !pyproject.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&pyproject).ok()?;

    // Check for pytest configuration sections
    // [tool.pytest.ini_options] is the standard location
    if content.contains("[tool.pytest") {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::PyprojectSection("[tool.pytest]".to_string()),
        });
    }

    // Check for pytest in dependencies
    if content.contains("pytest") {
        // Could be in [project.dependencies] or [tool.poetry.dependencies]
        // This is a weaker signal but still useful
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::PyprojectSection("dependencies".to_string()),
        });
    }

    None
}

/// Detect test runner from conftest.py presence.
fn detect_from_conftest(root: &Path) -> Option<PyDetectionResult> {
    // conftest.py at root is a strong pytest indicator
    if root.join("conftest.py").exists() {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::Conftest,
        });
    }

    // Also check tests/ directory
    if root.join("tests").join("conftest.py").exists() {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::Conftest,
        });
    }

    // Check test/ directory (singular)
    if root.join("test").join("conftest.py").exists() {
        return Some(PyDetectionResult {
            runner: PyRunner::Pytest,
            source: PyDetectionSource::Conftest,
        });
    }

    None
}

/// Detect test runner from test file patterns.
fn detect_from_test_patterns(root: &Path) -> Option<PyDetectionResult> {
    // Check for pytest-style test files in common locations
    let test_dirs = ["tests", "test", "."];

    for dir in test_dirs {
        let test_path = if dir == "." {
            root.to_path_buf()
        } else {
            root.join(dir)
        };

        if !test_path.is_dir() {
            continue;
        }

        // Look for test_*.py files (pytest convention)
        if let Ok(entries) = std::fs::read_dir(&test_path) {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                if name.starts_with("test_") && name.ends_with(".py") {
                    return Some(PyDetectionResult {
                        runner: PyRunner::Pytest,
                        source: PyDetectionSource::TestFilePattern,
                    });
                }
            }
        }
    }

    None
}

/// Check if pytest is installed and available.
fn is_pytest_available() -> bool {
    Command::new("pytest")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(test)]
#[path = "py_detect_tests.rs"]
mod tests;
