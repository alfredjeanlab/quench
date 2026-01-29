// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python language adapter.
//!
//! Provides Python-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Python files
//! - Project layout detection (src-layout vs flat-layout)
//! - Package name extraction from pyproject.toml and setup.py
//! - Default escape patterns (debuggers, eval/exec)
//! - Lint config policy checking
//! - Suppress directive parsing (noqa, type: ignore, pylint)
//! - Package manager detection (pip, poetry, uv, pipenv)
//!
//! See docs/specs/langs/python.md for specification.

use std::path::Path;

mod package_manager;

pub use package_manager::{PackageManager, PythonTooling};

use globset::GlobSet;

mod suppress;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use suppress::{PythonSuppress, PythonSuppressKind, parse_python_suppresses};

use super::common;
use super::common::patterns::normalize_exclude_patterns;
use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::PythonPolicyConfig;

/// Default escape patterns for Python.
///
/// These patterns detect potentially dangerous or debug-only code:
/// - Debugger patterns (breakpoint, pdb) - forbidden even in tests
/// - Dynamic execution patterns (eval, exec, __import__, compile) - require comments
const PYTHON_ESCAPE_PATTERNS: &[EscapePattern] = &[
    // Debugger patterns - forbidden even in tests
    EscapePattern {
        name: "breakpoint",
        pattern: r"\bbreakpoint\s*\(",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove breakpoint() before committing.",
        in_tests: Some("forbid"),
    },
    EscapePattern {
        name: "pdb_set_trace",
        pattern: r"\bpdb\.set_trace\s*\(",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove pdb.set_trace() before committing.",
        in_tests: Some("forbid"),
    },
    EscapePattern {
        name: "import_pdb",
        pattern: r"^\s*import\s+pdb\b",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove import pdb before committing.",
        in_tests: Some("forbid"),
    },
    EscapePattern {
        name: "from_pdb",
        pattern: r"^\s*from\s+pdb\s+import\b",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove pdb import before committing.",
        in_tests: Some("forbid"),
    },
    // Dynamic execution patterns - allowed in tests by default
    EscapePattern {
        name: "eval",
        pattern: r"\beval\s*\(",
        action: EscapeAction::Comment,
        comment: Some("# EVAL:"),
        advice: "Add a # EVAL: comment explaining why eval is necessary.",
        in_tests: None,
    },
    EscapePattern {
        name: "exec",
        pattern: r"\bexec\s*\(",
        action: EscapeAction::Comment,
        comment: Some("# EXEC:"),
        advice: "Add a # EXEC: comment explaining why exec is necessary.",
        in_tests: None,
    },
    EscapePattern {
        name: "__import__",
        pattern: r"\b__import__\s*\(",
        action: EscapeAction::Comment,
        comment: Some("# DYNAMIC:"),
        advice: "Add a # DYNAMIC: comment explaining why __import__ is necessary.",
        in_tests: None,
    },
    EscapePattern {
        name: "compile",
        pattern: r"\bcompile\s*\(",
        action: EscapeAction::Comment,
        comment: Some("# DYNAMIC:"),
        advice: "Add a # DYNAMIC: comment explaining why compile is necessary for code execution.",
        in_tests: None,
    },
];

/// Python language adapter.
pub struct PythonAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl PythonAdapter {
    /// Create a new Python adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.py".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**/*.py".to_string(),
                "**/tests/**/*.py".to_string(),
                "test/**/*.py".to_string(),
                "**/test/**/*.py".to_string(),
                "**/test_*.py".to_string(),
                "**/*_test.py".to_string(),
                "**/conftest.py".to_string(),
            ]),
            exclude_patterns: build_glob_set(&[
                ".venv/**".to_string(),
                "venv/**".to_string(),
                ".env/**".to_string(),
                "env/**".to_string(),
                "__pycache__/**".to_string(),
                "**/__pycache__/**".to_string(),
                ".mypy_cache/**".to_string(),
                ".pytest_cache/**".to_string(),
                ".ruff_cache/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                "*.egg-info/**".to_string(),
                "**/*.egg-info/**".to_string(),
                ".tox/**".to_string(),
                ".nox/**".to_string(),
            ]),
        }
    }

    /// Create a Python adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        let exclude_globs = normalize_exclude_patterns(&patterns.exclude);

        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            exclude_patterns: build_glob_set(&exclude_globs),
        }
    }

    /// Check if a path matches exclude patterns.
    pub fn should_exclude(&self, path: &Path) -> bool {
        common::patterns::check_exclude_patterns(
            path,
            &self.exclude_patterns,
            Some(&[
                ".venv",
                "venv",
                ".env",
                "env",
                "__pycache__",
                ".mypy_cache",
                ".pytest_cache",
                ".ruff_cache",
                "dist",
                "build",
                ".tox",
                ".nox",
            ]),
        )
    }
}

impl Default for PythonAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for PythonAdapter {
    fn name(&self) -> &'static str {
        "python"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Check exclude patterns first
        if self.should_exclude(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Source patterns
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }

        FileKind::Other
    }

    fn default_escapes(&self) -> &'static [EscapePattern] {
        PYTHON_ESCAPE_PATTERNS
    }
}

impl PythonAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &PythonPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
    }
}

// =============================================================================
// PACKAGE NAME EXTRACTION
// =============================================================================

/// Parse pyproject.toml to extract project name.
/// Looks for [project].name per PEP 621.
pub fn parse_pyproject_toml(content: &str) -> Option<String> {
    let table: toml::Table = content.parse().ok()?;
    table
        .get("project")?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

/// Parse setup.py to extract package name.
/// Uses regex to find name="..." or name='...' argument.
pub fn parse_setup_py(content: &str) -> Option<String> {
    // Match: name="package" or name='package' or name = "package"
    let re = regex::Regex::new(r#"name\s*=\s*["']([^"']+)["']"#).ok()?;
    re.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

// =============================================================================
// LAYOUT DETECTION
// =============================================================================

/// Project layout type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonLayout {
    /// src/package_name/ structure (PEP 517 recommended)
    SrcLayout,
    /// package_name/ in root (traditional)
    FlatLayout,
    /// No detectable layout
    Unknown,
}

/// Detect Python project layout.
pub fn detect_layout(root: &Path, package_name: Option<&str>) -> PythonLayout {
    // Check for src-layout first (recommended)
    let src_dir = root.join("src");
    if src_dir.is_dir() {
        // If we have a package name, verify it exists under src/
        if let Some(name) = package_name {
            let pkg_dir = src_dir.join(name.replace('-', "_"));
            if pkg_dir.join("__init__.py").exists() || pkg_dir.is_dir() {
                return PythonLayout::SrcLayout;
            }
        }
        // Otherwise, check if any package exists under src/
        if has_python_package(&src_dir) {
            return PythonLayout::SrcLayout;
        }
    }

    // Check for flat-layout
    if let Some(name) = package_name {
        let pkg_dir = root.join(name.replace('-', "_"));
        if pkg_dir.join("__init__.py").exists() {
            return PythonLayout::FlatLayout;
        }
    }

    // Check if root has any Python package
    if has_python_package(root) {
        return PythonLayout::FlatLayout;
    }

    PythonLayout::Unknown
}

/// Check if a directory contains Python packages (directories with __init__.py).
fn has_python_package(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_dir() && path.join("__init__.py").exists()
            })
        })
        .unwrap_or(false)
}

/// Check if a directory contains Python files (*.py).
fn has_python_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension()
                && ext == "py"
            {
                return true;
            }
        }
    }
    false
}

/// Detect Python package from project root.
///
/// Returns (package_path, package_name) if detected.
/// Uses pyproject.toml or setup.py for package name, and layout detection
/// for the package path.
pub fn detect_package(root: &Path) -> Option<(String, String)> {
    // Try to get package name from pyproject.toml first
    let package_name = if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
        parse_pyproject_toml(&content)
    } else if let Ok(content) = std::fs::read_to_string(root.join("setup.py")) {
        parse_setup_py(&content)
    } else {
        None
    };

    // Detect layout and build package path
    let layout = detect_layout(root, package_name.as_deref());

    match layout {
        PythonLayout::SrcLayout => {
            // src/package_name/ structure
            let src_dir = root.join("src");
            if let Ok(entries) = std::fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = entry.file_name().to_string_lossy().to_string();
                        // Skip __pycache__ and similar
                        if dir_name.starts_with('_') || dir_name.starts_with('.') {
                            continue;
                        }
                        // Check if it's a Python package
                        if path.join("__init__.py").exists() || path.join("py.typed").exists() {
                            let pkg_path = format!("src/{}", dir_name);
                            let pkg_name =
                                package_name.unwrap_or_else(|| dir_name.replace('-', "_"));
                            return Some((pkg_path, pkg_name));
                        }
                    }
                }
            }
            // Fall through: src/ exists but no package directory found
            // Treat src/ itself as the package if it has Python files
            if has_python_files(&src_dir)
                && let Some(name) = package_name
            {
                return Some(("src".to_string(), name));
            }
        }
        PythonLayout::FlatLayout => {
            // package_name/ in root
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = entry.file_name().to_string_lossy().to_string();
                        // Skip common non-package directories
                        if dir_name.starts_with('_')
                            || dir_name.starts_with('.')
                            || dir_name == "tests"
                            || dir_name == "test"
                            || dir_name == "docs"
                            || dir_name == "build"
                            || dir_name == "dist"
                        {
                            continue;
                        }
                        // Check if it's a Python package
                        if path.join("__init__.py").exists() {
                            let pkg_name =
                                package_name.unwrap_or_else(|| dir_name.replace('-', "_"));
                            return Some((dir_name, pkg_name));
                        }
                    }
                }
            }
            // Fall through: no package directory found
            // Treat root as the package if we have a package name from config
            if let Some(name) = package_name {
                return Some((".".to_string(), name));
            }
        }
        PythonLayout::Unknown => {
            // No detected layout, but if we have package name from config,
            // treat root as the package
            if let Some(name) = package_name {
                return Some((".".to_string(), name));
            }
        }
    }

    None
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
