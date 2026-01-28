// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python language adapter.
//!
//! Provides Python-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Python files
//! - Project layout detection (src-layout vs flat-layout)
//! - Package name extraction from pyproject.toml and setup.py
//!
//! See docs/specs/langs/python.md for specification.

use std::path::Path;

use globset::GlobSet;

use super::glob::build_glob_set;
use super::{Adapter, EscapePattern, FileKind};

/// Python language adapter.
pub struct PythonAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl PythonAdapter {
    /// Create a new Python adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.py".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**/*.py".to_string(),
                "**/test_*.py".to_string(),
                "**/*_test.py".to_string(),
                "**/conftest.py".to_string(),
            ]),
            ignore_patterns: build_glob_set(&[
                ".venv/**".to_string(),
                "venv/**".to_string(),
                "__pycache__/**".to_string(),
                ".mypy_cache/**".to_string(),
                ".pytest_cache/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                "*.egg-info/**".to_string(),
                ".tox/**".to_string(),
                ".nox/**".to_string(),
            ]),
        }
    }

    /// Create a Python adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        // Convert ignore patterns to glob patterns (add ** if needed)
        let ignore_globs: Vec<String> = patterns
            .ignore
            .iter()
            .map(|p| {
                if p.ends_with('/') {
                    format!("{}**", p)
                } else if !p.contains('*') {
                    format!("{}/**", p.trim_end_matches('/'))
                } else {
                    p.clone()
                }
            })
            .collect();

        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&ignore_globs),
        }
    }

    /// Check if a path matches ignore patterns.
    pub fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check explicit ignore patterns
        if self.ignore_patterns.is_match(path) {
            return true;
        }

        // Also check for common ignored directories by path prefix
        // This handles cases where the path starts with these directories
        let parts: Vec<&str> = path_str.split('/').collect();
        if !parts.is_empty() {
            let first = parts[0];
            if first == ".venv"
                || first == "venv"
                || first == "__pycache__"
                || first == ".mypy_cache"
                || first == ".pytest_cache"
                || first == "dist"
                || first == "build"
                || first == ".tox"
                || first == ".nox"
                || first.ends_with(".egg-info")
            {
                return true;
            }
        }

        // Check for __pycache__ anywhere in the path
        if parts.contains(&"__pycache__") {
            return true;
        }

        false
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
        // Check ignore patterns first
        if self.should_ignore(path) {
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
        &[] // Phase 445 will add escape patterns
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

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
