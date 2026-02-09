// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parser for `.mdc` (Cursor rule) files.
//!
//! `.mdc` files use YAML frontmatter between `---` delimiters to encode
//! metadata about when and how a rule applies. This module parses that
//! frontmatter and classifies the rule's scope.

use std::path::{Path, PathBuf};

/// Parsed `.mdc` rule file.
#[derive(Debug)]
pub struct MdcRule {
    /// Rule description (for "apply intelligently" mode).
    pub description: Option<String>,
    /// Glob patterns for file-scoped rules.
    pub globs: Option<Vec<String>>,
    /// Whether this rule always applies.
    pub always_apply: bool,
    /// Markdown body (content after frontmatter).
    pub body: String,
    /// Original file path.
    pub path: PathBuf,
}

/// Classification of how a rule applies.
#[derive(Debug, PartialEq, Eq)]
pub enum RuleScope {
    /// `alwaysApply: true` - reconciles with root CLAUDE.md.
    AlwaysApply,
    /// Single directory glob (e.g. `src/api/**`) - reconciles with dir agent file.
    SingleDirectory(PathBuf),
    /// File-pattern globs (e.g. `**/*.tsx`) - no reconciliation target.
    FilePattern,
    /// Manual or intelligent application - no reconciliation target.
    OnDemand,
}

/// Error when parsing `.mdc` frontmatter.
#[derive(Debug)]
pub struct MdcParseError {
    pub message: String,
    pub path: PathBuf,
}

impl std::fmt::Display for MdcParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

/// Parse an `.mdc` file's content into an `MdcRule`.
///
/// The format uses YAML frontmatter between `---` delimiters:
/// ```markdown
/// ---
/// description: "Standards for API endpoints"
/// globs: "src/api/**"
/// alwaysApply: false
/// ---
///
/// ## API Conventions
/// ...
/// ```
pub fn parse_mdc(content: &str, path: PathBuf) -> Result<MdcRule, MdcParseError> {
    let mut lines = content.lines().peekable();

    // Check for frontmatter delimiter
    if lines.peek().map(|l| l.trim()) != Some("---") {
        // No frontmatter - treat as plain markdown
        return Ok(MdcRule {
            description: None,
            globs: None,
            always_apply: false,
            body: content.to_string(),
            path,
        });
    }
    lines.next(); // skip opening ---

    let mut description = None;
    let mut globs = None;
    let mut always_apply = false;
    let mut found_closing = false;

    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_closing = true;
            break;
        }
        if let Some(value) = line.strip_prefix("description:") {
            description = Some(unquote(value.trim()));
        } else if let Some(value) = line.strip_prefix("globs:") {
            globs = Some(parse_globs(value.trim()));
        } else if let Some(value) = line.strip_prefix("alwaysApply:") {
            always_apply = value.trim() == "true";
        }
        // Ignore unknown keys gracefully
    }

    if !found_closing {
        return Err(MdcParseError {
            message: "unterminated frontmatter (missing closing ---)".to_string(),
            path,
        });
    }

    // Collect remaining lines as the body.
    // Skip one leading blank line after frontmatter if present.
    let remaining: Vec<&str> = lines.collect();
    let body = if remaining.first().map(|l| l.is_empty()).unwrap_or(false) {
        remaining[1..].join("\n")
    } else {
        remaining.join("\n")
    };

    Ok(MdcRule { description, globs, always_apply, body, path })
}

/// Classify the scope of an `MdcRule`.
pub fn classify_scope(rule: &MdcRule) -> RuleScope {
    if rule.always_apply {
        return RuleScope::AlwaysApply;
    }

    let Some(ref globs) = rule.globs else {
        return RuleScope::OnDemand;
    };

    if globs.is_empty() {
        return RuleScope::OnDemand;
    }

    if globs.len() != 1 {
        return RuleScope::FilePattern;
    }

    let glob = &globs[0];

    // Check if glob is "{dir}/**", "{dir}/**/*", or "{dir}/*"
    let dir = glob
        .strip_suffix("/**/*")
        .or_else(|| glob.strip_suffix("/**"))
        .or_else(|| glob.strip_suffix("/*"));

    match dir {
        Some(d) if !d.contains('*') && !d.contains('?') && !d.is_empty() => {
            RuleScope::SingleDirectory(PathBuf::from(d))
        }
        _ => RuleScope::FilePattern,
    }
}

/// Remove surrounding quotes from a string value.
fn unquote(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse the `globs` field value, which may be a single string or a YAML array.
///
/// Handles:
/// - `"src/api/**"` (single quoted string)
/// - `src/api/**` (unquoted string)
/// - `["src/**", "lib/**"]` (YAML array)
fn parse_globs(value: &str) -> Vec<String> {
    let trimmed = value.trim();

    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        // YAML array: ["src/**", "lib/**"]
        let inner = &trimmed[1..trimmed.len() - 1];
        inner.split(',').map(|s| unquote(s.trim())).filter(|s| !s.is_empty()).collect()
    } else {
        // Single value
        let unquoted = unquote(trimmed);
        if unquoted.is_empty() { vec![] } else { vec![unquoted] }
    }
}

/// Strip the leading `# Header` line from markdown content.
///
/// Returns the content starting from the line after the first `# ` heading,
/// or the original content if no leading heading is found.
pub fn strip_leading_header(content: &str) -> &str {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("# ") {
        // Find end of header line
        rest.find('\n').map(|i| rest[i + 1..].trim_start_matches('\n')).unwrap_or("")
    } else {
        content
    }
}

/// Discover all `.mdc` files under `.cursor/rules/`.
pub fn discover_mdc_files(root: &Path) -> Vec<PathBuf> {
    let rules_dir = root.join(".cursor").join("rules");
    if !rules_dir.is_dir() {
        return vec![];
    }

    let Ok(entries) = std::fs::read_dir(&rules_dir) else {
        return vec![];
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| {
            let entry = e.ok()?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "mdc") { Some(path) } else { None }
        })
        .collect();

    paths.sort();
    paths
}

#[cfg(test)]
#[path = "mdc_tests.rs"]
mod tests;
