// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript workspace detection for npm/yarn/pnpm monorepos.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

/// JavaScript workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct JsWorkspace {
    /// Is this a workspace root?
    pub is_workspace: bool,
    /// Package paths relative to root (e.g., ["packages/core", "packages/cli"]).
    pub package_paths: Vec<String>,
    /// Mapping from path to display name (short directory name for output).
    pub package_names: HashMap<String, String>,
    /// Workspace patterns from config (e.g., ["packages/*"]).
    pub patterns: Vec<String>,
}

impl JsWorkspace {
    /// Parse workspace info from project root.
    ///
    /// Checks in order:
    /// 1. pnpm-workspace.yaml (pnpm)
    /// 2. package.json workspaces field (npm/yarn)
    pub fn from_root(root: &Path) -> Self {
        // Check pnpm-workspace.yaml first
        if let Some(ws) = Self::from_pnpm_workspace(root) {
            return ws;
        }

        // Fall back to package.json workspaces
        Self::from_package_json(root).unwrap_or_default()
    }

    fn from_pnpm_workspace(root: &Path) -> Option<Self> {
        let path = root.join("pnpm-workspace.yaml");
        let content = fs::read_to_string(&path).ok()?;
        let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;

        let patterns = value
            .get("packages")?
            .as_sequence()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>();

        if patterns.is_empty() {
            return None;
        }

        let (package_paths, package_names) = expand_workspace_patterns(&patterns, root);
        Some(Self {
            is_workspace: true,
            package_paths,
            package_names,
            patterns,
        })
    }

    fn from_package_json(root: &Path) -> Option<Self> {
        let path = root.join("package.json");
        let content = fs::read_to_string(&path).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;

        let workspaces = value.get("workspaces")?;

        // Handle both array and object forms
        let patterns: Vec<String> = match workspaces {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Value::Object(obj) => {
                // { "packages": ["..."] } form
                obj.get("packages")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }
            _ => return None,
        };

        if patterns.is_empty() {
            return None;
        }

        let (package_paths, package_names) = expand_workspace_patterns(&patterns, root);
        Some(Self {
            is_workspace: true,
            package_paths,
            package_names,
            patterns,
        })
    }
}

/// Expand workspace patterns to find package paths and names.
/// Returns (paths, path_to_name_map).
fn expand_workspace_patterns(
    patterns: &[String],
    root: &Path,
) -> (Vec<String>, HashMap<String, String>) {
    let mut paths = Vec::new();
    let mut names = HashMap::new();

    for pattern in patterns {
        if let Some(base) = pattern.strip_suffix("/*") {
            // Single-level glob: packages/*
            expand_single_level(root, base, &mut paths, &mut names);
        } else if let Some(base) = pattern.strip_suffix("/**") {
            // Recursive glob: packages/** (treat as single level)
            expand_single_level(root, base, &mut paths, &mut names);
        } else if !pattern.contains('*') {
            // Direct path: packages/core
            let pkg_dir = root.join(pattern);
            if pkg_dir.join("package.json").exists() {
                // Use directory name as display name
                let dir_name = pkg_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
                    .unwrap_or_else(|| pattern.clone());
                paths.push(pattern.clone());
                names.insert(pattern.clone(), dir_name);
            }
        }
    }

    paths.sort();
    (paths, names)
}

fn expand_single_level(
    root: &Path,
    base: &str,
    paths: &mut Vec<String>,
    names: &mut HashMap<String, String>,
) {
    let dir = root.join(base);
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() && entry.path().join("package.json").exists() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                let rel_path = format!("{}/{}", base, dir_name);
                paths.push(rel_path.clone());
                names.insert(rel_path, dir_name);
            }
        }
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
