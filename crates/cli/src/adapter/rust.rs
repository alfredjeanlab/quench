//! Rust language adapter.
//!
//! Provides Rust-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Rust projects
//! - (Future) Inline test detection via #[cfg(test)]
//!
//! See docs/specs/langs/rust.md for specification.

use std::fs;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use toml::Value;

use super::{Adapter, FileKind};

/// Rust language adapter.
pub struct RustAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RustAdapter {
    /// Create a new Rust adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.rs".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**".to_string(),
                "test/**/*.rs".to_string(),
                "*_test.rs".to_string(),
                "*_tests.rs".to_string(),
            ]),
            ignore_patterns: build_glob_set(&["target/**".to_string()]),
        }
    }

    /// Check if a path should be ignored (e.g., target/).
    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Default for RustAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for RustAdapter {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
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
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

// =============================================================================
// WORKSPACE PARSING
// =============================================================================

/// Cargo workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct CargoWorkspace {
    /// Is this a workspace root?
    pub is_workspace: bool,
    /// Package names in the workspace.
    pub packages: Vec<String>,
    /// Member glob patterns (e.g., "crates/*").
    pub member_patterns: Vec<String>,
}

impl CargoWorkspace {
    /// Parse workspace info from Cargo.toml at the given root.
    pub fn from_root(root: &Path) -> Self {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(&cargo_toml) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let value: Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        Self::from_toml(&value, root)
    }

    fn from_toml(value: &Value, root: &Path) -> Self {
        let workspace = value.get("workspace");

        if workspace.is_none() {
            // Single package, not a workspace
            if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
                return Self {
                    is_workspace: false,
                    packages: vec![pkg.as_str().unwrap_or("").to_string()],
                    member_patterns: vec![],
                };
            }
            return Self::default();
        }

        // Safe: we checked workspace.is_none() above
        let Some(workspace) = workspace else {
            return Self::default();
        };
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Expand member patterns to find actual packages
        let packages = expand_workspace_members(&members, root);

        Self {
            is_workspace: true,
            packages,
            member_patterns: members,
        }
    }
}

/// Expand workspace member patterns to package names.
fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        // Handle glob patterns like "crates/*"
        if pattern.contains('*') {
            if let Some(base) = pattern.strip_suffix("/*") {
                let dir = root.join(base);
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir()
                            && let Some(name) = read_package_name(&entry.path())
                        {
                            packages.push(name);
                        }
                    }
                }
            }
        } else {
            // Direct path to package
            let pkg_dir = root.join(pattern);
            if let Some(name) = read_package_name(&pkg_dir) {
                packages.push(name);
            }
        }
    }

    packages.sort();
    packages
}

/// Read package name from a directory's Cargo.toml.
fn read_package_name(dir: &Path) -> Option<String> {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).ok()?;
    let value: Value = toml::from_str(&content).ok()?;
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(String::from)
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod tests;
