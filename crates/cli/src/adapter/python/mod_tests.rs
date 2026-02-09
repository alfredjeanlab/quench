// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the Python adapter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::{Adapter, FileKind};

use super::*;

// =============================================================================
// FILE CLASSIFICATION TESTS
// =============================================================================

#[test]
fn classifies_source_files() {
    let adapter = PythonAdapter::new();

    assert_eq!(adapter.classify(Path::new("app.py")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("src/app.py")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("lib/utils.py")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("main.py")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("src/mypackage/main.py")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("package/module.py")), FileKind::Source);
}

#[test]
fn classifies_test_files() {
    let adapter = PythonAdapter::new();

    // tests/ directory
    assert_eq!(adapter.classify(Path::new("tests/test_app.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/test_main.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/unit/test_models.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/unit/test_utils.py")), FileKind::Test);

    // test_*.py pattern
    assert_eq!(adapter.classify(Path::new("test_main.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("test_app.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("src/test_utils.py")), FileKind::Test);

    // *_test.py pattern
    assert_eq!(adapter.classify(Path::new("app_test.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("utils_test.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("src/utils_test.py")), FileKind::Test);

    // conftest.py
    assert_eq!(adapter.classify(Path::new("conftest.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/conftest.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/unit/conftest.py")), FileKind::Test);
}

// =============================================================================
// EXCLUDE PATTERN TESTS
// =============================================================================

#[test]
fn ignores_venv_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(".venv/lib/python3.11/site-packages/foo.py")),
        FileKind::Other
    );
    assert_eq!(
        adapter.classify(Path::new("venv/lib/python3.11/site-packages/foo.py")),
        FileKind::Other
    );
}

#[test]
fn ignores_pycache_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new("__pycache__/module.cpython-311.pyc")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("src/__pycache__/module.py")), FileKind::Other);
}

#[test]
fn ignores_mypy_cache() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new(".mypy_cache/3.11/module.py")), FileKind::Other);
}

#[test]
fn ignores_pytest_cache() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new(".pytest_cache/v/cache/foo.py")), FileKind::Other);
}

#[test]
fn ignores_ruff_cache() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new(".ruff_cache/foo.py")), FileKind::Other);
}

#[test]
fn ignores_dist_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(
        adapter.classify(Path::new("dist/mypackage-1.0.0/mypackage/module.py")),
        FileKind::Other
    );
}

#[test]
fn ignores_build_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new("build/lib/mypackage/module.py")), FileKind::Other);
}

#[test]
fn ignores_egg_info_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.classify(Path::new("mypackage.egg-info/PKG-INFO")), FileKind::Other);
}

#[test]
fn ignores_tox_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(".tox/py311/lib/python3.11/site-packages/foo.py")),
        FileKind::Other
    );
}

#[test]
fn ignores_nox_directory() {
    let adapter = PythonAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(".nox/tests/lib/python3.11/site-packages/foo.py")),
        FileKind::Other
    );
}

#[test]
fn classifies_non_python_files() {
    let adapter = PythonAdapter::new();

    assert_eq!(adapter.classify(Path::new("README.md")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("pyproject.toml")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("setup.py")), FileKind::Source); // This is Python!
    assert_eq!(adapter.classify(Path::new("requirements.txt")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("Makefile")), FileKind::Other);
}

#[test]
fn test_patterns_take_precedence_over_source() {
    let adapter = PythonAdapter::new();

    // A file matching both source and test patterns should be classified as test
    assert_eq!(adapter.classify(Path::new("tests/helpers.py")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("tests/test_lib.py")), FileKind::Test);
}

#[test]
fn should_exclude_common_directories() {
    let adapter = PythonAdapter::new();

    // Virtual environments
    assert!(adapter.should_exclude(Path::new(".venv/lib/site-packages/foo.py")));
    assert!(adapter.should_exclude(Path::new("venv/bin/python")));
    assert!(adapter.should_exclude(Path::new(".env/lib/site-packages/foo.py")));
    assert!(adapter.should_exclude(Path::new("env/bin/python")));

    // Cache directories
    assert!(adapter.should_exclude(Path::new("__pycache__/module.cpython-311.pyc")));
    assert!(adapter.should_exclude(Path::new(".mypy_cache/3.11/module.py")));
    assert!(adapter.should_exclude(Path::new(".pytest_cache/v/cache/lastfailed")));
    assert!(adapter.should_exclude(Path::new(".ruff_cache/0.1.0/foo")));

    // Build directories
    assert!(adapter.should_exclude(Path::new("dist/mypackage-1.0.0.tar.gz")));
    assert!(adapter.should_exclude(Path::new("build/lib/mypackage/module.py")));

    // Tox and nox
    assert!(adapter.should_exclude(Path::new(".tox/py311/lib/python3.11/site.py")));
    assert!(adapter.should_exclude(Path::new(".nox/tests/lib/python3.11/site.py")));

    // Normal source should not be excluded
    assert!(!adapter.should_exclude(Path::new("src/app.py")));
    assert!(!adapter.should_exclude(Path::new("mypackage/module.py")));
}

// =============================================================================
// ADAPTER TRAIT TESTS
// =============================================================================

#[test]
fn adapter_name() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.name(), "python");
}

#[test]
fn adapter_extensions() {
    let adapter = PythonAdapter::new();
    assert_eq!(adapter.extensions(), &["py"]);
}

// =============================================================================
// ESCAPE PATTERN TESTS
// =============================================================================

#[test]
fn default_escapes_include_debuggers() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    assert!(escapes.iter().any(|e| e.name == "breakpoint"));
    assert!(escapes.iter().any(|e| e.name == "pdb_set_trace"));
    assert!(escapes.iter().any(|e| e.name == "import_pdb"));
    assert!(escapes.iter().any(|e| e.name == "from_pdb"));
}

#[test]
fn default_escapes_include_dynamic_execution() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    assert!(escapes.iter().any(|e| e.name == "eval"));
    assert!(escapes.iter().any(|e| e.name == "exec"));
    assert!(escapes.iter().any(|e| e.name == "__import__"));
    assert!(escapes.iter().any(|e| e.name == "compile"));
}

#[test]
fn debugger_escapes_are_forbid() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let debugger_names = ["breakpoint", "pdb_set_trace", "import_pdb", "from_pdb"];
    for escape in escapes.iter().filter(|e| debugger_names.contains(&e.name)) {
        assert_eq!(
            escape.action,
            EscapeAction::Forbid,
            "debugger {} should be Forbid",
            escape.name
        );
    }
}

#[test]
fn debugger_escapes_forbid_in_tests() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let debugger_names = ["breakpoint", "pdb_set_trace", "import_pdb", "from_pdb"];
    for escape in escapes.iter().filter(|e| debugger_names.contains(&e.name)) {
        assert_eq!(
            escape.in_tests,
            Some("forbid"),
            "debugger {} should be forbidden in tests",
            escape.name
        );
    }
}

#[test]
fn dynamic_execution_escapes_require_comment() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let dynamic_names = ["eval", "exec", "__import__", "compile"];
    for escape in escapes.iter().filter(|e| dynamic_names.contains(&e.name)) {
        assert_eq!(
            escape.action,
            EscapeAction::Comment,
            "dynamic {} should be Comment",
            escape.name
        );
        assert!(escape.comment.is_some(), "dynamic {} should have comment pattern", escape.name);
    }
}

#[test]
fn dynamic_execution_escapes_allowed_in_tests() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let dynamic_names = ["eval", "exec", "__import__", "compile"];
    for escape in escapes.iter().filter(|e| dynamic_names.contains(&e.name)) {
        assert_eq!(
            escape.in_tests, None,
            "dynamic {} should be allowed in tests by default",
            escape.name
        );
    }
}

#[test]
fn eval_requires_eval_comment() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let eval_escape = escapes.iter().find(|e| e.name == "eval").unwrap();
    assert_eq!(eval_escape.comment, Some("# EVAL:"));
}

#[test]
fn exec_requires_exec_comment() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let exec_escape = escapes.iter().find(|e| e.name == "exec").unwrap();
    assert_eq!(exec_escape.comment, Some("# EXEC:"));
}

#[test]
fn dunder_import_requires_dynamic_comment() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let import_escape = escapes.iter().find(|e| e.name == "__import__").unwrap();
    assert_eq!(import_escape.comment, Some("# DYNAMIC:"));
}

#[test]
fn compile_requires_dynamic_comment() {
    let adapter = PythonAdapter::new();
    let escapes = adapter.default_escapes();

    let compile_escape = escapes.iter().find(|e| e.name == "compile").unwrap();
    assert_eq!(compile_escape.comment, Some("# DYNAMIC:"));
}

#[test]
fn with_patterns_uses_custom_patterns() {
    let patterns = super::super::ResolvedPatterns {
        source: vec!["src/**/*.py".to_string()],
        test: vec!["test/**/*.py".to_string()],
        exclude: vec!["vendor/".to_string()],
    };

    let adapter = PythonAdapter::with_patterns(patterns);

    // Custom source pattern
    assert_eq!(adapter.classify(Path::new("src/app.py")), FileKind::Source);

    // Custom test pattern
    assert_eq!(adapter.classify(Path::new("test/test_app.py")), FileKind::Test);

    // File outside custom patterns
    assert_eq!(adapter.classify(Path::new("lib/utils.py")), FileKind::Other);
}

// =============================================================================
// PYPROJECT.TOML PARSING TESTS
// =============================================================================

#[test]
fn parses_pyproject_toml_pep621() {
    let content = r#"
[project]
name = "myproject"
version = "1.0.0"
"#;
    assert_eq!(parse_pyproject_toml(content), Some("myproject".to_string()));
}

#[test]
fn parses_pyproject_toml_with_hyphens() {
    let content = r#"
[project]
name = "my-awesome-project"
"#;
    assert_eq!(parse_pyproject_toml(content), Some("my-awesome-project".to_string()));
}

#[test]
fn returns_none_for_pyproject_without_project_section() {
    let content = r#"
[tool.black]
line-length = 88
"#;
    assert_eq!(parse_pyproject_toml(content), None);
}

#[test]
fn returns_none_for_pyproject_without_name() {
    let content = r#"
[project]
version = "1.0.0"
"#;
    assert_eq!(parse_pyproject_toml(content), None);
}

#[test]
fn returns_none_for_invalid_toml() {
    let content = "not valid toml {{{{";
    assert_eq!(parse_pyproject_toml(content), None);
}

// =============================================================================
// SETUP.PY PARSING TESTS
// =============================================================================

#[test]
fn parses_setup_py_double_quotes() {
    let content = r#"
from setuptools import setup

setup(
    name="myproject",
    version="1.0.0",
)
"#;
    assert_eq!(parse_setup_py(content), Some("myproject".to_string()));
}

#[test]
fn parses_setup_py_single_quotes() {
    let content = r#"
from setuptools import setup

setup(
    name='myproject',
    version='1.0.0',
)
"#;
    assert_eq!(parse_setup_py(content), Some("myproject".to_string()));
}

#[test]
fn parses_setup_py_with_spaces() {
    let content = r#"
setup(
    name = "myproject",
)
"#;
    assert_eq!(parse_setup_py(content), Some("myproject".to_string()));
}

#[test]
fn parses_setup_py_with_hyphens() {
    let content = r#"
setup(name="my-awesome-project")
"#;
    assert_eq!(parse_setup_py(content), Some("my-awesome-project".to_string()));
}

#[test]
fn returns_none_for_setup_py_without_name() {
    let content = r#"
from setuptools import setup
setup(version="1.0.0")
"#;
    assert_eq!(parse_setup_py(content), None);
}

// =============================================================================
// LAYOUT DETECTION TESTS
// =============================================================================

#[test]
fn detect_layout_returns_unknown_for_empty_dir() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(detect_layout(temp.path(), None), PythonLayout::Unknown);
}

#[test]
fn detect_layout_finds_src_layout_with_package_name() {
    let temp = tempfile::tempdir().unwrap();

    // Create src/mypackage/__init__.py
    let pkg_dir = temp.path().join("src").join("mypackage");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    assert_eq!(detect_layout(temp.path(), Some("mypackage")), PythonLayout::SrcLayout);
}

#[test]
fn detect_layout_finds_src_layout_without_package_name() {
    let temp = tempfile::tempdir().unwrap();

    // Create src/somepackage/__init__.py
    let pkg_dir = temp.path().join("src").join("somepackage");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    assert_eq!(detect_layout(temp.path(), None), PythonLayout::SrcLayout);
}

#[test]
fn detect_layout_normalizes_hyphens_to_underscores() {
    let temp = tempfile::tempdir().unwrap();

    // Create src/my_package/__init__.py
    let pkg_dir = temp.path().join("src").join("my_package");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    // Query with hyphenated name
    assert_eq!(detect_layout(temp.path(), Some("my-package")), PythonLayout::SrcLayout);
}

#[test]
fn detect_layout_finds_flat_layout_with_package_name() {
    let temp = tempfile::tempdir().unwrap();

    // Create mypackage/__init__.py
    let pkg_dir = temp.path().join("mypackage");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    assert_eq!(detect_layout(temp.path(), Some("mypackage")), PythonLayout::FlatLayout);
}

#[test]
fn detect_layout_finds_flat_layout_without_package_name() {
    let temp = tempfile::tempdir().unwrap();

    // Create anypackage/__init__.py
    let pkg_dir = temp.path().join("anypackage");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    assert_eq!(detect_layout(temp.path(), None), PythonLayout::FlatLayout);
}

#[test]
fn detect_layout_prefers_src_layout_over_flat() {
    let temp = tempfile::tempdir().unwrap();

    // Create both layouts
    let src_pkg = temp.path().join("src").join("mypackage");
    std::fs::create_dir_all(&src_pkg).unwrap();
    std::fs::write(src_pkg.join("__init__.py"), "").unwrap();

    let flat_pkg = temp.path().join("mypackage");
    std::fs::create_dir_all(&flat_pkg).unwrap();
    std::fs::write(flat_pkg.join("__init__.py"), "").unwrap();

    // src-layout should take precedence
    assert_eq!(detect_layout(temp.path(), Some("mypackage")), PythonLayout::SrcLayout);
}
