// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for the Python language adapter.
//!
//! Tests that quench correctly:
//! - Detects Python projects via pyproject.toml, setup.py, setup.cfg, requirements.txt
//! - Applies default source/test/ignore patterns
//! - Handles src-layout and flat-layout project structures
//! - Detects package name from config files and directory structure
//! - Applies Python-specific escape patterns
//! - Applies Python-specific suppress patterns
//! - Enforces lint config standalone policy
//!
//! Reference: docs/specs/langs/python.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > pyproject.toml detection
#[test]
fn auto_detected_when_pyproject_toml_present() {
    let result = cli().on("python/auto-detect-pyproject").json().passes();
    let checks = result.checks();
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc")),
        "cloc check should be present for Python project"
    );
}

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > setup.py detection
#[test]
fn auto_detected_when_setup_py_present() {
    let result = cli().on("python/auto-detect-setup-py").json().passes();
    let checks = result.checks();
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc")),
        "cloc check should be present for Python project"
    );
}

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > requirements.txt detection (fallback)
#[test]
fn auto_detected_when_requirements_txt_present() {
    let result = cli().on("python/auto-detect-requirements").json().passes();
    let checks = result.checks();
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("cloc")),
        "cloc check should be present for Python project"
    );
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Python adapter activates when pyproject.toml exists (escapes check)
#[test]
fn python_adapter_auto_detected_when_pyproject_toml_present() {
    let result = cli().on("python/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have python-specific patterns active
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Python adapter activates when setup.py exists
#[test]
fn python_adapter_auto_detected_when_setup_py_present() {
    let result = cli().on("python/setup-py").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Python adapter activates when setup.cfg exists
#[test]
fn python_adapter_auto_detected_when_setup_cfg_present() {
    let result = cli().on("python/setup-cfg").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Python adapter activates when requirements.txt exists (fallback)
#[test]
fn python_adapter_auto_detected_when_requirements_txt_present() {
    let result = cli().on("python/requirements").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > Default source patterns (**/*.py)
#[test]
fn default_source_pattern_matches_py_files() {
    let cloc = check("cloc")
        .on("python/auto-detect-pyproject")
        .json()
        .passes();
    let metrics = cloc.require("metrics");
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .py files as source");
}

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > Default test patterns (tests/**/*.py, test_*.py, *_test.py, conftest.py)
#[test]
fn default_test_pattern_matches_test_files() {
    let cloc = check("cloc")
        .on("python/auto-detect-pyproject")
        .json()
        .passes();
    let metrics = cloc.require("metrics");
    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count test_*.py files as test");
}

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > Default ignores (.venv/, __pycache__/, etc.)
#[test]
fn default_ignores_venv_directory() {
    let cloc = check("cloc").on("python/venv-ignore").json().passes();
    let metrics = cloc.require("metrics");
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    // Only src/myproject/__init__.py should be counted, not .venv/lib/site.py
    assert!(
        source_lines < 20,
        ".venv/ should be ignored, got {} lines",
        source_lines
    );
}

/// Spec: docs/specs/langs/python.md#default-patterns
///
/// > source = ["**/*.py"]
#[test]
fn python_adapter_default_source_pattern_matches_py_files() {
    let cloc = check("cloc").on("python/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    // Should count .py files as source
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .py files as source");
}

/// Spec: docs/specs/langs/python.md#default-patterns
///
/// > tests = ["tests/**/*.py", "test_*.py", "*_test.py", "conftest.py"]
#[test]
fn python_adapter_default_test_pattern_matches_test_files() {
    let cloc = check("cloc").on("python/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // tests/test_app.py should be counted as test LOC
    assert!(
        test_lines > 0,
        "test files in tests/ should be counted as test LOC"
    );
}

/// Spec: docs/specs/langs/python.md#default-patterns
///
/// > ignore = [".venv/", ...]
#[test]
fn python_adapter_default_ignores_venv_directory() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/app.py", "def main(): pass");
    temp.file(".venv/lib/site.py", "# should be ignored");

    let cloc = check("cloc").pwd(temp.path()).json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| { f.as_str().map(|s| s.contains(".venv/")).unwrap_or(false) }),
            ".venv/ directory should be ignored"
        );
    }
}

/// Spec: docs/specs/langs/python.md#default-patterns
///
/// > ignore = ["__pycache__/", ...]
#[test]
fn python_adapter_default_ignores_pycache_directory() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/app.py", "def main(): pass");
    temp.file("src/__pycache__/app.cpython-311.pyc", "# compiled");

    let cloc = check("cloc").pwd(temp.path()).json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files.iter().any(|f| {
                f.as_str()
                    .map(|s| s.contains("__pycache__/"))
                    .unwrap_or(false)
            }),
            "__pycache__/ directory should be ignored"
        );
    }
}

// =============================================================================
// LAYOUT DETECTION SPECS
// =============================================================================

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > Src-layout detection (src/package_name/)
#[test]
fn detects_src_layout_structure() {
    let cloc = check("cloc").on("python/src-layout").json().passes();
    let metrics = cloc.require("metrics");
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should detect src-layout");
}

/// Spec: plans/.5-roadmap-python.md#phase-443
///
/// > Flat-layout detection (package_name/)
#[test]
fn detects_flat_layout_structure() {
    let cloc = check("cloc").on("python/flat-layout").json().passes();
    let metrics = cloc.require("metrics");
    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should detect flat-layout");
}

// =============================================================================
// PACKAGE DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/langs/python.md#package-detection
///
/// > Package name detected from pyproject.toml [project].name
#[test]
fn python_adapter_detects_package_name_from_pyproject_toml() {
    let cloc = check("cloc").on("python/auto-detect").json().passes();

    // Package detection should identify "test-project" from pyproject.toml
    let by_package = cloc.get("by_package");
    assert!(
        by_package.is_some(),
        "should detect package from pyproject.toml"
    );
}

/// Spec: docs/specs/langs/python.md#package-detection
///
/// > Package name detected from setup.py setup(name=...)
#[test]
fn python_adapter_detects_package_name_from_setup_py() {
    let cloc = check("cloc").on("python/setup-py").json().passes();

    // Package detection should identify "test-project" from setup.py
    let by_package = cloc.get("by_package");
    assert!(by_package.is_some(), "should detect package from setup.py");
}

/// Spec: docs/specs/langs/python.md#package-detection
///
/// > src-layout: src/package_name/__init__.py
#[test]
fn python_adapter_detects_src_layout_package() {
    let cloc = check("cloc").on("python/src-layout").json().passes();

    // Should detect "mypackage" from src/mypackage/__init__.py
    let by_package = cloc.get("by_package");
    assert!(
        by_package.is_some(),
        "should detect package from src-layout structure"
    );
}

/// Spec: docs/specs/langs/python.md#package-detection
///
/// > flat-layout: package_name/__init__.py
#[test]
fn python_adapter_detects_flat_layout_package() {
    let cloc = check("cloc").on("python/flat-layout").json().passes();

    // Should detect "mypackage" from mypackage/__init__.py
    let by_package = cloc.get("by_package");
    assert!(
        by_package.is_some(),
        "should detect package from flat-layout structure"
    );
}

// =============================================================================
// ESCAPE PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > eval( | comment | # EVAL:
#[test]
fn python_adapter_eval_without_comment_fails() {
    check("escapes")
        .on("python/escape-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# EVAL:");
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > eval( | comment | # EVAL:
#[test]
fn python_adapter_eval_with_comment_passes() {
    check("escapes").on("python/escape-ok").passes();
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > exec( | comment | # EXEC:
#[test]
fn python_adapter_exec_without_comment_fails() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/runner.py", "exec(code_string)");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("# EXEC:");
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > __import__( | comment | # DYNAMIC:
#[test]
fn python_adapter_dynamic_import_without_comment_fails() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/loader.py", "mod = __import__(module_name)");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("# DYNAMIC:");
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > breakpoint() | forbid
#[test]
fn python_adapter_breakpoint_always_fails() {
    check("escapes")
        .on("python/debugger-fail")
        .fails()
        .stdout_has("breakpoint()");
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > pdb.set_trace() | forbid
#[test]
fn python_adapter_pdb_set_trace_always_fails() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/debug.py", "import pdb; pdb.set_trace()");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("pdb.set_trace()");
}

/// Spec: docs/specs/langs/python.md#default-escape-patterns
///
/// > import pdb | forbid
#[test]
fn python_adapter_import_pdb_always_fails() {
    let temp = Project::empty();
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/debug.py", "import pdb\n\ndef debug(): pass");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("import pdb");
}

// =============================================================================
// SUPPRESS PATTERN SPECS
// =============================================================================

/// Spec: docs/specs/langs/python.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
fn python_adapter_noqa_without_comment_fails_when_configured() {
    check("escapes")
        .on("python/suppress-fail")
        .fails()
        .stdout_has("# noqa");
}

/// Spec: docs/specs/langs/python.md#suppress
///
/// > "comment" - Requires justification comment
#[test]
fn python_adapter_noqa_with_comment_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[python.suppress]
check = "comment"
"#,
    );
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    // Noqa with justification
    temp.file(
        "src/app.py",
        "# Legacy API compatibility\nx = 1  # noqa: E501",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/python.md#suppress
///
/// > type: ignore without code fails when configured
#[test]
fn python_adapter_type_ignore_without_comment_fails_when_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[python.suppress]
check = "comment"
"#,
    );
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("src/app.py", "x: int = \"not int\"  # type: ignore");

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("# type: ignore");
}

/// Spec: docs/specs/langs/python.md#suppress
///
/// > [python.suppress.test] check = "allow" - tests can suppress freely
#[test]
fn python_adapter_noqa_in_test_code_always_passes() {
    let temp = Project::empty();
    temp.config(
        r#"[python.suppress]
check = "comment"
[python.suppress.test]
check = "allow"
"#,
    );
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file("tests/test_app.py", "x = 1  # noqa\n\ndef test_x(): pass");

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: docs/specs/langs/python.md#suppress
///
/// > pylint: disable requires justification
#[test]
fn python_adapter_pylint_disable_without_comment_fails_when_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[python.suppress]
check = "comment"
"#,
    );
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );
    temp.file(
        "src/app.py",
        "# pylint: disable=missing-docstring\ndef f(): pass",
    );

    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("# pylint: disable");
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

/// Spec: docs/specs/langs/python.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
fn python_adapter_lint_config_changes_with_source_fails_standalone_policy() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[python.policy]
lint_changes = "standalone"
lint_config = ["pyproject.toml"]
"#,
    );

    // Setup pyproject.toml
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create initial commit with source
    temp.file("src/app.py", "def f(): pass");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n\n[tool.ruff]\nline-length = 100\n",
    );
    temp.file("src/app.py", "def f(): pass\ndef g(): pass");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Check with --base HEAD should detect mixed changes
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint config")
        .stdout_has("separate PR");
}

/// Spec: docs/specs/langs/python.md#policy
///
/// > lint_config files alone pass standalone policy
#[test]
fn python_adapter_lint_config_standalone_passes() {
    let temp = Project::empty();

    // Setup quench.toml with standalone policy
    temp.config(
        r#"[python.policy]
lint_changes = "standalone"
lint_config = ["ruff.toml"]
"#,
    );

    // Setup pyproject.toml
    temp.file(
        "pyproject.toml",
        "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
    );

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create initial commit
    temp.file("src/app.py", "def f(): pass");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add ONLY lint config change (no source changes)
    temp.file("ruff.toml", "line-length = 100\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should pass - only lint config changed
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}
