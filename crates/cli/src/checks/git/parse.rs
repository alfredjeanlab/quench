// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Conventional commit message parsing.
//!
//! Parses commit messages in the format: `<type>(<scope>): <description>`
//! where scope is optional.

use std::sync::LazyLock;

use regex::Regex;

/// Pattern for conventional commit format.
///
/// Captures:
/// - Group 1: type (required)
/// - Group 2: scope with parens (optional)
/// - Group 3: scope without parens (optional)
/// - Group 4: description (required)
#[allow(clippy::expect_used)]
static CONVENTIONAL_COMMIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-z]+)(\(([^)]+)\))?:\s*(.+)$").expect("valid regex"));

/// Default conventional commit types.
pub const DEFAULT_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style",
];

/// A parsed conventional commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommit {
    /// Commit type (e.g., "feat", "fix").
    pub commit_type: String,
    /// Optional scope (e.g., "api", "cli").
    pub scope: Option<String>,
    /// Commit description.
    pub description: String,
}

impl ParsedCommit {
    /// Check if the commit type is in the allowed list.
    ///
    /// If `allowed_types` is empty, any type is accepted (structure-only validation).
    /// If `allowed_types` is `None`, default types are used.
    pub fn is_type_allowed(&self, allowed_types: Option<&[String]>) -> bool {
        match allowed_types {
            None => DEFAULT_TYPES.contains(&self.commit_type.as_str()),
            Some([]) => true, // Empty = any type
            Some(types) => types.iter().any(|t| t == &self.commit_type),
        }
    }

    /// Check if the scope is in the allowed list.
    ///
    /// If `allowed_scopes` is `None`, any scope (or no scope) is accepted.
    /// If `allowed_scopes` is `Some(&[])`, no scopes are allowed.
    /// If `allowed_scopes` is `Some(&[...])`, only those scopes are allowed.
    pub fn is_scope_allowed(&self, allowed_scopes: Option<&[String]>) -> bool {
        match (allowed_scopes, &self.scope) {
            // No scope restriction configured
            (None, _) => true,
            // Scope restriction configured, but no scope in commit
            (Some(_), None) => true, // Allow commits without scope
            // Scope restriction configured and scope present
            (Some(scopes), Some(scope)) => scopes.iter().any(|s| s == scope),
        }
    }

    /// Get the scope if present, for error reporting.
    pub fn scope_str(&self) -> Option<&str> {
        self.scope.as_deref()
    }
}

/// Parse result for a commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// Successfully parsed conventional commit.
    Conventional(ParsedCommit),
    /// Message does not match conventional format.
    NonConventional,
}

/// Parse a commit message as a conventional commit.
///
/// Returns `ParseResult::Conventional` if the message matches the format,
/// or `ParseResult::NonConventional` if it doesn't.
///
/// # Examples
///
/// ```
/// use quench::checks::git::parse::{parse_conventional_commit, ParseResult};
///
/// let result = parse_conventional_commit("feat(api): add endpoint");
/// assert!(matches!(result, ParseResult::Conventional(_)));
///
/// let result = parse_conventional_commit("update stuff");
/// assert!(matches!(result, ParseResult::NonConventional));
/// ```
pub fn parse_conventional_commit(message: &str) -> ParseResult {
    let Some(caps) = CONVENTIONAL_COMMIT.captures(message) else {
        return ParseResult::NonConventional;
    };

    // Groups 1 and 4 are always present when the regex matches
    let (Some(commit_type), Some(description)) = (caps.get(1), caps.get(4)) else {
        return ParseResult::NonConventional;
    };

    let scope = caps.get(3).map(|m| m.as_str().to_string());

    ParseResult::Conventional(ParsedCommit {
        commit_type: commit_type.as_str().to_string(),
        scope,
        description: description.as_str().to_string(),
    })
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
