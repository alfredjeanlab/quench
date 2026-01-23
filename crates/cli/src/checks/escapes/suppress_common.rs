// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared suppress checking logic for Rust and Shell.
//!
//! Extracted common patterns from Rust `#[allow(...)]` and Shell `# shellcheck disable=...`
//! checking to eliminate duplication.

use crate::config::{SuppressLevel, SuppressScopeConfig};

/// Parameters for checking suppress attributes.
pub struct SuppressCheckParams<'a> {
    /// The scope-specific config (source or test).
    pub scope_config: &'a SuppressScopeConfig,
    /// Effective check level for this scope.
    pub scope_check: SuppressLevel,
    /// Global comment pattern (fallback when no per-lint pattern).
    pub global_comment: Option<&'a str>,
}

/// Information about a suppress attribute being checked.
pub struct SuppressAttrInfo<'a> {
    /// Lint codes being suppressed.
    pub codes: &'a [String],
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The actual comment text if found.
    pub comment_text: Option<&'a str>,
}

/// Type of suppress violation detected.
#[derive(Debug, PartialEq, Eq)]
pub enum SuppressViolationKind {
    /// A forbidden lint code was suppressed.
    Forbidden { code: String },
    /// Missing required justification comment.
    MissingComment {
        /// The required comment pattern (if any).
        required_pattern: Option<String>,
    },
    /// All suppressions are forbidden at this scope level.
    AllForbidden,
}

/// Check a suppress attribute against scope config.
///
/// Returns `None` if no violation, `Some(kind)` if violation detected.
/// Stops at the first violation found.
pub fn check_suppress_attr(
    params: &SuppressCheckParams,
    attr: &SuppressAttrInfo,
) -> Option<SuppressViolationKind> {
    // 1. Check forbid list first
    for code in attr.codes {
        if is_code_in_list(code, &params.scope_config.forbid) {
            return Some(SuppressViolationKind::Forbidden { code: code.clone() });
        }
    }

    // 2. Check allow list - if any code matches, skip remaining checks
    for code in attr.codes {
        if is_code_in_list(code, &params.scope_config.allow) {
            return None;
        }
    }

    // 3. Check if all suppressions are forbidden at this level
    if params.scope_check == SuppressLevel::Forbid {
        return Some(SuppressViolationKind::AllForbidden);
    }

    // 4. Check comment requirement
    if params.scope_check == SuppressLevel::Comment {
        let required_pattern = find_required_pattern(params, attr);
        if !has_valid_comment(attr, required_pattern.as_deref()) {
            return Some(SuppressViolationKind::MissingComment { required_pattern });
        }
    }

    None
}

/// Find the required comment pattern for an attribute.
/// Checks per-lint patterns first, then falls back to global.
fn find_required_pattern(params: &SuppressCheckParams, attr: &SuppressAttrInfo) -> Option<String> {
    // Check per-lint patterns first (first matching code wins)
    for code in attr.codes {
        if let Some(pattern) = params.scope_config.patterns.get(code) {
            return Some(pattern.clone());
        }
    }
    // Fall back to global pattern
    params.global_comment.map(String::from)
}

/// Check if the attribute has a valid justification comment.
fn has_valid_comment(attr: &SuppressAttrInfo, required_pattern: Option<&str>) -> bool {
    if !attr.has_comment {
        return false;
    }

    match (required_pattern, &attr.comment_text) {
        (Some(pattern), Some(text)) => {
            // Normalize both pattern and text for comparison
            let norm_pattern = normalize_comment_pattern(pattern);
            let norm_text = normalize_comment_text(text);
            norm_text.starts_with(&norm_pattern)
        }
        (Some(_), None) => false,
        (None, _) => attr.has_comment,
    }
}

/// Normalize a comment pattern by stripping common prefixes.
fn normalize_comment_pattern(pattern: &str) -> String {
    pattern
        .trim()
        .trim_start_matches("//")
        .trim_start_matches('#')
        .trim()
        .to_string()
}

/// Normalize comment text by stripping common prefixes.
fn normalize_comment_text(text: &str) -> String {
    text.trim()
        .trim_start_matches("//")
        .trim_start_matches('#')
        .trim()
        .to_string()
}

/// Check if a lint code matches any pattern in a list.
/// Supports exact match and prefix match (e.g., "clippy" matches "clippy::unwrap_used").
fn is_code_in_list(code: &str, list: &[String]) -> bool {
    list.iter().any(|pattern| code_matches(code, pattern))
}

/// Check if a code matches a pattern.
/// Supports exact match and prefix match with `::` separator.
fn code_matches(code: &str, pattern: &str) -> bool {
    code == pattern || code.starts_with(&format!("{}::", pattern))
}

#[cfg(test)]
#[path = "suppress_common_tests.rs"]
mod tests;
