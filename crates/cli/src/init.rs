// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Init command detection and output.

use std::path::Path;

/// Languages that can be detected in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectedLanguage {
    Rust,
    Golang,
    JavaScript,
    Shell,
}

/// Detect all languages present in a project.
///
/// Returns a list of detected languages. Detection is additive:
/// a project with Cargo.toml and scripts/*.sh returns both Rust and Shell.
pub fn detect_languages(root: &Path) -> Vec<DetectedLanguage> {
    let mut languages = Vec::new();

    // Rust: Cargo.toml exists
    if root.join("Cargo.toml").exists() {
        languages.push(DetectedLanguage::Rust);
    }

    // Go: go.mod exists
    if root.join("go.mod").exists() {
        languages.push(DetectedLanguage::Golang);
    }

    // JavaScript: package.json, tsconfig.json, or jsconfig.json exists
    if root.join("package.json").exists()
        || root.join("tsconfig.json").exists()
        || root.join("jsconfig.json").exists()
    {
        languages.push(DetectedLanguage::JavaScript);
    }

    // Shell: *.sh in root, bin/, or scripts/
    if has_shell_markers(root) {
        languages.push(DetectedLanguage::Shell);
    }

    languages
}

/// Check if project has Shell markers.
fn has_shell_markers(root: &Path) -> bool {
    has_sh_files(root)
        || root.join("bin").is_dir() && has_sh_files(&root.join("bin"))
        || root.join("scripts").is_dir() && has_sh_files(&root.join("scripts"))
}

/// Check if a directory contains *.sh files.
fn has_sh_files(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sh")
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
