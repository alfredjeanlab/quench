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

/// Agents that can be detected in a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetectedAgent {
    Claude,
    /// Cursor agent with the actual required file/pattern found.
    Cursor(CursorMarker),
}

/// Cursor marker type detected in the project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CursorMarker {
    /// .cursorrules file exists
    Cursorrules,
    /// .cursor/rules/*.md or *.mdc files exist
    CursorRulesDir,
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

/// Detect all agents present in a project.
///
/// Returns a list of detected agents. Detection is additive:
/// a project with CLAUDE.md and .cursorrules returns both Claude and Cursor.
pub fn detect_agents(root: &Path) -> Vec<DetectedAgent> {
    let mut agents = Vec::new();

    // Claude: CLAUDE.md exists
    if root.join("CLAUDE.md").exists() {
        agents.push(DetectedAgent::Claude);
    }

    // Cursor: .cursorrules takes precedence over .cursor/rules/*.mdc
    if root.join(".cursorrules").exists() {
        agents.push(DetectedAgent::Cursor(CursorMarker::Cursorrules));
    } else if has_cursor_rules_dir(root) {
        agents.push(DetectedAgent::Cursor(CursorMarker::CursorRulesDir));
    }

    agents
}

/// Check if project has .cursor/rules/*.md[c] files.
fn has_cursor_rules_dir(root: &Path) -> bool {
    let rules_dir = root.join(".cursor/rules");
    if rules_dir.is_dir()
        && let Ok(entries) = rules_dir.read_dir()
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && (ext == "md" || ext == "mdc")
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
