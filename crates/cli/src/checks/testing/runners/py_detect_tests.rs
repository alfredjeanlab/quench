// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::fs;
use tempfile::TempDir;

fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn detects_pytest_from_pytest_ini() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::write(temp.path().join("pytest.ini"), "[pytest]\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::ConfigFile(ref name) if name == "pytest.ini"
    ));
}

#[test]
fn detects_pytest_from_setup_cfg_tool_pytest() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("setup.py"), "").unwrap();
    fs::write(temp.path().join("setup.cfg"), "[tool:pytest]\naddopts = -v\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::ConfigFile(ref name) if name == "setup.cfg"
    ));
}

#[test]
fn detects_pytest_from_setup_cfg_pytest_section() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("setup.py"), "").unwrap();
    fs::write(temp.path().join("setup.cfg"), "[pytest]\naddopts = -v\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::ConfigFile(ref name) if name == "setup.cfg"
    ));
}

#[test]
fn detects_pytest_from_pyproject_tool_section() {
    let temp = create_temp_dir();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname = \"test\"\n\n[tool.pytest.ini_options]\naddopts = \"-v\"\n",
    )
    .unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::PyprojectSection(ref section) if section == "[tool.pytest]"
    ));
}

#[test]
fn detects_pytest_from_pyproject_dependencies() {
    let temp = create_temp_dir();
    fs::write(
        temp.path().join("pyproject.toml"),
        r#"[project]
name = "test"
dependencies = ["pytest>=7.0"]
"#,
    )
    .unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::PyprojectSection(ref section) if section == "dependencies"
    ));
}

#[test]
fn detects_pytest_from_conftest_at_root() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::write(temp.path().join("conftest.py"), "# pytest fixtures\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(result.source, PyDetectionSource::Conftest));
}

#[test]
fn detects_pytest_from_conftest_in_tests() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::create_dir(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests").join("conftest.py"), "# pytest fixtures\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(result.source, PyDetectionSource::Conftest));
}

#[test]
fn detects_pytest_from_test_file_pattern() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::create_dir(temp.path().join("tests")).unwrap();
    fs::write(temp.path().join("tests").join("test_example.py"), "def test_one(): pass\n").unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(result.source, PyDetectionSource::TestFilePattern));
}

#[test]
fn returns_none_for_non_python_project() {
    let temp = create_temp_dir();
    // No Python markers

    let result = detect_py_runner(temp.path());
    assert!(result.is_none());
}

#[test]
fn is_python_project_detects_pyproject() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn is_python_project_detects_setup_py() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("setup.py"), "from setuptools import setup\nsetup()\n").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn is_python_project_detects_requirements() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("requirements.txt"), "requests>=2.0\n").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn is_python_project_detects_pipfile() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("Pipfile"), "[packages]\n").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn is_python_project_detects_poetry_lock() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("poetry.lock"), "").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn is_python_project_detects_uv_lock() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("uv.lock"), "").unwrap();

    assert!(is_python_project(temp.path()));
}

#[test]
fn py_runner_name() {
    assert_eq!(PyRunner::Pytest.name(), "pytest");
    assert_eq!(PyRunner::Unittest.name(), "unittest");
}

#[test]
fn detection_source_to_metric_string() {
    assert_eq!(
        PyDetectionSource::ConfigFile("pytest.ini".to_string()).to_metric_string(),
        "config_file:pytest.ini"
    );
    assert_eq!(
        PyDetectionSource::PyprojectSection("[tool.pytest]".to_string()).to_metric_string(),
        "pyproject_section:[tool.pytest]"
    );
    assert_eq!(PyDetectionSource::Conftest.to_metric_string(), "conftest");
    assert_eq!(PyDetectionSource::TestFilePattern.to_metric_string(), "test_file_pattern");
    assert_eq!(PyDetectionSource::Fallback.to_metric_string(), "fallback");
}

#[test]
fn detects_pytest_from_tox_ini() {
    let temp = create_temp_dir();
    fs::write(temp.path().join("pyproject.toml"), "[project]\nname = \"test\"\n").unwrap();
    fs::write(
        temp.path().join("tox.ini"),
        "[tox]\nenvlist = py39\n\n[testenv]\ncommands = pytest\n",
    )
    .unwrap();

    let result = detect_py_runner(temp.path()).unwrap();
    assert_eq!(result.runner, PyRunner::Pytest);
    assert!(matches!(
        result.source,
        PyDetectionSource::ConfigFile(ref name) if name == "tox.ini"
    ));
}
