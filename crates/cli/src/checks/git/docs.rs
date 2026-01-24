// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Agent documentation detection for commit format.
//!
//! Checks that commit format is documented in agent files.

use std::path::Path;

use regex::Regex;

/// Agent files to check for commit documentation.
const AGENT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", ".cursorrules"];

/// Default commit types to search for.
const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style",
];

/// Result of searching for commit format documentation.
#[derive(Debug)]
pub enum DocsResult {
    /// Documentation found in the specified file.
    // NOTE(lifetime): Used in tests to verify which file matched
    #[allow(dead_code)]
    Found(String),
    /// No documentation found; lists checked files.
    NotFound(Vec<String>),
    /// No agent files exist at root.
    NoAgentFiles,
}

/// Check if commit format is documented in agent files.
///
/// Searches for:
/// - Type prefixes followed by `:` or `(` (e.g., `feat:`, `fix(`)
/// - The phrase "conventional commits" (case-insensitive)
pub fn check_commit_docs(root: &Path) -> DocsResult {
    let mut checked_files = Vec::new();

    for filename in AGENT_FILES {
        let path = root.join(filename);
        if !path.exists() {
            continue;
        }

        checked_files.push(filename.to_string());

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        if has_commit_documentation(&content) {
            return DocsResult::Found(filename.to_string());
        }
    }

    if checked_files.is_empty() {
        DocsResult::NoAgentFiles
    } else {
        DocsResult::NotFound(checked_files)
    }
}

/// Check if content contains commit format documentation.
pub fn has_commit_documentation(content: &str) -> bool {
    has_type_prefix(content) || has_conventional_commits_phrase(content)
}

/// Check for type prefixes followed by `:` or `(`.
///
/// Matches: `feat:`, `fix(`, `chore:`, etc.
fn has_type_prefix(content: &str) -> bool {
    // Build regex pattern: (feat|fix|chore|...)[:({]
    let types_pattern = COMMIT_TYPES.join("|");
    let pattern = format!(r"(?i)\b({})[:(\(]", types_pattern);

    Regex::new(&pattern)
        .map(|re| re.is_match(content))
        .unwrap_or(false)
}

/// Check for "conventional commits" phrase (case-insensitive).
fn has_conventional_commits_phrase(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("conventional commits") || lower.contains("conventional commit")
}

/// Get the primary agent file name for violation reporting.
///
/// Returns the first agent file that exists, or "CLAUDE.md" as default.
pub fn primary_agent_file(root: &Path) -> &'static str {
    for filename in AGENT_FILES {
        if root.join(filename).exists() {
            return filename;
        }
    }
    "CLAUDE.md"
}

#[cfg(test)]
#[path = "docs_tests.rs"]
mod tests;
