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
        let required_patterns = find_required_patterns(params, attr);
        if !has_valid_comment(attr, &required_patterns) {
            // For error message, show first pattern as the required one
            let required_pattern = required_patterns.first().cloned();
            return Some(SuppressViolationKind::MissingComment { required_pattern });
        }
    }

    None
}

/// Find the required comment patterns for an attribute.
/// Checks per-lint patterns first, then falls back to global.
/// Returns a list of valid patterns (any match is acceptable).
fn find_required_patterns(params: &SuppressCheckParams, attr: &SuppressAttrInfo) -> Vec<String> {
    // Check per-lint patterns first (first matching code wins)
    for code in attr.codes {
        if let Some(patterns) = params.scope_config.patterns.get(code) {
            return patterns.clone();
        }
    }
    // Fall back to global pattern
    params
        .global_comment
        .map(|p| vec![p.to_string()])
        .unwrap_or_default()
}

/// Check if the attribute has a valid justification comment.
/// If required_patterns is non-empty, comment must match one of them.
/// If required_patterns is empty, any non-empty comment is valid.
fn has_valid_comment(attr: &SuppressAttrInfo, required_patterns: &[String]) -> bool {
    if !attr.has_comment {
        return false;
    }

    // If no specific patterns required, any comment is valid
    if required_patterns.is_empty() {
        return true;
    }

    // Need to match one of the patterns
    let Some(text) = &attr.comment_text else {
        return false;
    };

    let norm_text = normalize_comment_text(text);
    required_patterns.iter().any(|pattern| {
        let norm_pattern = normalize_comment_pattern(pattern);
        norm_text.starts_with(&norm_pattern)
    })
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
