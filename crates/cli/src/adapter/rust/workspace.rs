// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cargo workspace parsing.
//!
//! Parses Cargo.toml to detect workspace structure and member packages.

use std::fs;
use std::path::Path;

use toml::Value;

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
        let Some(workspace) = value.get("workspace") else {
            // Single package, not a workspace
            if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
                return Self {
                    is_workspace: false,
                    packages: vec![pkg.as_str().unwrap_or("").to_string()],
                    member_patterns: vec![],
                };
            }
            return Self::default();
        };

        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
            .unwrap_or_default();

        // Expand member patterns to find actual packages
        let packages = expand_workspace_members(&members, root);

        Self { is_workspace: true, packages, member_patterns: members }
    }
}

/// Expand workspace member patterns to package names.
fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            // Handle glob patterns
            if let Some(base) = pattern.strip_suffix("/*") {
                // Single-level glob: crates/*
                expand_single_level(root, base, &mut packages);
            } else if let Some(base) = pattern.strip_suffix("/**") {
                // Recursive glob: crates/** (treat as single level for now)
                // Most workspaces use ** as a future-proofing pattern
                expand_single_level(root, base, &mut packages);
            } else if pattern.ends_with("/*") || pattern.contains("/*") {
                // Pattern like path/to/* - extract base
                if let Some(pos) = pattern.rfind("/*") {
                    let base = &pattern[..pos];
                    expand_single_level(root, base, &mut packages);
                }
            }
            // Other glob patterns are not supported - skip silently
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

/// Expand a single-level directory to find packages.
fn expand_single_level(root: &Path, base: &str, packages: &mut Vec<String>) {
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

/// Read package name from a directory's Cargo.toml.
fn read_package_name(dir: &Path) -> Option<String> {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).ok()?;
    let value: Value = toml::from_str(&content).ok()?;
    value.get("package")?.get("name")?.as_str().map(String::from)
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
