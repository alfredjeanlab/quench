// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File detection logic for agent files.

use std::path::{Path, PathBuf};

use globset::Glob;

/// A detected agent file with its scope.
#[derive(Debug)]
pub struct DetectedFile {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Scope at which the file was found.
    pub scope: Scope,
}

/// Scope at which an agent file was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Direct child of project root.
    Root,
    /// Under a configured package directory.
    Package(String),
    /// Nested deeper than root (non-package).
    Module,
}

/// Detect agent files under a directory.
///
/// Looks for files matching the given patterns at:
/// - Root directory (direct children and glob patterns)
/// - Package directories (if configured)
pub fn detect_agent_files(
    root: &Path,
    packages: &[String],
    patterns: &[String],
) -> Vec<DetectedFile> {
    let mut detected = Vec::new();

    // Check root for each pattern
    for pattern in patterns {
        let matches = match_pattern(pattern, root);
        for path in matches {
            let scope = classify_scope(&path, root, packages);
            detected.push(DetectedFile { path, scope });
        }
    }

    // Check package directories (expand globs like "crates/*")
    for pkg in packages {
        let pkg_dirs = expand_package_glob(root, pkg);
        for pkg_path in pkg_dirs {
            let pkg_name = pkg_path
                .strip_prefix(root)
                .unwrap_or(&pkg_path)
                .to_string_lossy()
                .to_string();

            for pattern in patterns {
                // Only check exact file names in packages, not glob patterns
                if pattern.contains('*') {
                    continue;
                }

                let file_path = pkg_path.join(pattern);
                if file_path.exists() {
                    detected.push(DetectedFile {
                        path: file_path,
                        scope: Scope::Package(pkg_name.clone()),
                    });
                }
            }
        }
    }

    detected
}

/// Expand a package pattern (which may contain globs) into actual directories.
///
/// For example, "crates/*" expands to ["crates/core", "crates/cli", ...].
fn expand_package_glob(root: &Path, pkg_pattern: &str) -> Vec<PathBuf> {
    if !pkg_pattern.contains('*') {
        // Exact path, no glob
        let pkg_path = root.join(pkg_pattern);
        if pkg_path.is_dir() {
            return vec![pkg_path];
        }
        return vec![];
    }

    // Handle glob pattern like "crates/*"
    let pattern_path = Path::new(pkg_pattern);
    let Some(parent) = pattern_path.parent() else {
        return vec![];
    };
    let Some(name_pattern) = pattern_path.file_name().and_then(|s| s.to_str()) else {
        return vec![];
    };

    let search_dir = root.join(parent);
    if !search_dir.is_dir() {
        return vec![];
    }

    let Ok(glob) = Glob::new(name_pattern) else {
        return vec![];
    };
    let matcher = glob.compile_matcher();

    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return vec![];
    };

    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = entry.file_name();
            let name_str = name.to_str()?;
            if matcher.is_match(name_str) {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

/// Match a pattern against files in a directory.
///
/// Supports both exact file names and glob patterns.
fn match_pattern(pattern: &str, root: &Path) -> Vec<PathBuf> {
    if pattern.contains('*') {
        // Glob pattern - we need to handle this manually
        match_glob_pattern(pattern, root)
    } else {
        // Exact file name
        let path = root.join(pattern);
        if path.exists() { vec![path] } else { vec![] }
    }
}

/// Match a glob pattern against files in a directory.
fn match_glob_pattern(pattern: &str, root: &Path) -> Vec<PathBuf> {
    // Extract the directory prefix and file pattern
    // e.g., ".cursor/rules/*.md" -> dir=".cursor/rules", file_pattern="*.md"
    let path_pattern = Path::new(pattern);
    let parent = path_pattern.parent();
    let file_pattern = path_pattern.file_name().and_then(|s| s.to_str());

    let Some(parent) = parent else {
        return vec![];
    };
    let Some(file_pattern) = file_pattern else {
        return vec![];
    };

    let search_dir = root.join(parent);
    if !search_dir.is_dir() {
        return vec![];
    }

    // Build a glob matcher for the file pattern
    let Ok(glob) = Glob::new(file_pattern) else {
        return vec![];
    };
    let matcher = glob.compile_matcher();

    // Read the directory and match files
    let Ok(entries) = std::fs::read_dir(&search_dir) else {
        return vec![];
    };

    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_str()?;

            if matcher.is_match(file_name_str) {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect()
}

/// Classify the scope of a file path.
pub fn classify_scope(file_path: &Path, root: &Path, packages: &[String]) -> Scope {
    let relative = file_path.strip_prefix(root).unwrap_or(file_path);

    // Check if under a package directory
    for pkg in packages {
        if is_in_package(relative, pkg) {
            return Scope::Package(extract_package_name(relative, pkg));
        }
    }

    // Direct child of root = Root scope
    // Deeper nesting = Module scope
    if relative.components().count() == 1 {
        Scope::Root
    } else {
        Scope::Module
    }
}

/// Check if a relative path is under a package pattern.
fn is_in_package(relative: &Path, pkg_pattern: &str) -> bool {
    let relative_str = relative.to_string_lossy();

    if pkg_pattern.contains('*') {
        // Handle wildcard patterns like "crates/*"
        let prefix = pkg_pattern.trim_end_matches('*').trim_end_matches('/');
        relative_str.starts_with(prefix)
    } else {
        relative_str.starts_with(pkg_pattern)
    }
}

/// Extract package name from a relative path given a package pattern.
fn extract_package_name(relative: &Path, pkg_pattern: &str) -> String {
    let relative_str = relative.to_string_lossy();

    if pkg_pattern.contains('*') {
        // For patterns like "crates/*", extract "crates/cli" from "crates/cli/CLAUDE.md"
        let prefix = pkg_pattern.trim_end_matches('*').trim_end_matches('/');
        let rest = relative_str
            .strip_prefix(prefix)
            .unwrap_or(&relative_str)
            .trim_start_matches('/');

        // Take up to the next path separator
        let pkg_name = rest.split('/').next().unwrap_or(rest);
        format!("{}/{}", prefix, pkg_name)
    } else {
        pkg_pattern.to_string()
    }
}

/// Check if a required file exists at the root.
pub fn file_exists_at_root(root: &Path, filename: &str) -> bool {
    if filename.contains('*') {
        // Glob pattern - check if any matches
        !match_glob_pattern(filename, root).is_empty()
    } else {
        root.join(filename).exists()
    }
}

#[cfg(test)]
#[path = "detection_tests.rs"]
mod tests;
