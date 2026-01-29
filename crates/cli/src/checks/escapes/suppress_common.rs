// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared suppress checking logic for all language suppress checkers.
//!
//! Provides common traits and functions to eliminate duplication across
//! Go, JavaScript, Shell, Ruby, and Rust suppress checkers.

use std::path::Path;

use crate::check::{CheckContext, Violation};
use crate::config::{
    GoSuppressConfig, RubySuppressConfig, ShellSuppressConfig, SuppressConfig, SuppressLevel,
    SuppressScopeConfig,
};

use super::violations::try_create_violation;

// =============================================================================
// Traits and Types for Generic Suppress Checking
// =============================================================================

/// Trait for accessing suppress configuration fields uniformly.
///
/// Implemented by GoSuppressConfig, ShellSuppressConfig, RubySuppressConfig,
/// and SuppressConfig (used by JavaScript and Rust).
pub trait SuppressConfigAccess {
    /// Get the base check level.
    fn check(&self) -> SuppressLevel;
    /// Get the global comment pattern (optional).
    fn comment(&self) -> Option<&str>;
    /// Get the source scope configuration.
    fn source(&self) -> &SuppressScopeConfig;
    /// Get the test scope configuration.
    fn test(&self) -> &SuppressScopeConfig;
}

/// A unified suppress directive for checking.
///
/// Each language parser converts its specific directive type to this unified
/// format before calling the generic checking function.
pub struct UnifiedSuppressDirective {
    /// Line number (0-indexed).
    pub line: usize,
    /// Lint codes being suppressed.
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
    /// Pre-formatted pattern string for violation messages.
    pub pattern: String,
}

// =============================================================================
// SuppressConfigAccess Implementations
// =============================================================================

impl SuppressConfigAccess for GoSuppressConfig {
    fn check(&self) -> SuppressLevel {
        self.check
    }
    fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn source(&self) -> &SuppressScopeConfig {
        &self.source
    }
    fn test(&self) -> &SuppressScopeConfig {
        &self.test
    }
}

impl SuppressConfigAccess for ShellSuppressConfig {
    fn check(&self) -> SuppressLevel {
        self.check
    }
    fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn source(&self) -> &SuppressScopeConfig {
        &self.source
    }
    fn test(&self) -> &SuppressScopeConfig {
        &self.test
    }
}

impl SuppressConfigAccess for RubySuppressConfig {
    fn check(&self) -> SuppressLevel {
        self.check
    }
    fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn source(&self) -> &SuppressScopeConfig {
        &self.source
    }
    fn test(&self) -> &SuppressScopeConfig {
        &self.test
    }
}

impl SuppressConfigAccess for SuppressConfig {
    fn check(&self) -> SuppressLevel {
        self.check
    }
    fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn source(&self) -> &SuppressScopeConfig {
        &self.source
    }
    fn test(&self) -> &SuppressScopeConfig {
        &self.test
    }
}

/// Check suppress violations from a list of directives.
///
/// This is the main entry point for the generic suppress checking logic.
/// Each language-specific checker parses directives, converts them to
/// UnifiedSuppressDirective, then calls this function.
// TODO(refactor): Consider grouping ctx+path+limit_reached into a CheckState struct
#[allow(clippy::too_many_arguments)]
pub fn check_suppress_violations_generic<C: SuppressConfigAccess>(
    ctx: &CheckContext,
    path: &Path,
    directives: Vec<UnifiedSuppressDirective>,
    config: &C,
    language: &str,
    violation_type_prefix: &str,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Get scope config and check level
    let (scope_config, scope_check) = if is_test_file {
        (
            config.test(),
            config.test().check.unwrap_or(SuppressLevel::Allow),
        )
    } else {
        (
            config.source(),
            config.source().check.unwrap_or(config.check()),
        )
    };

    // If allow, no checking needed
    if scope_check == SuppressLevel::Allow {
        return violations;
    }

    for directive in directives {
        if *limit_reached {
            break;
        }

        // Build params for shared checking logic
        let params = SuppressCheckParams {
            scope_config,
            scope_check,
            global_comment: config.comment(),
        };

        let attr_info = SuppressAttrInfo {
            codes: &directive.codes,
            has_comment: directive.has_comment,
            comment_text: directive.comment_text.as_deref(),
        };

        // Use shared checking logic
        if let Some(violation_kind) = check_suppress_attr(&params, &attr_info) {
            let (violation_type, advice) = match violation_kind {
                SuppressViolationKind::Forbidden { ref code } => {
                    let advice = format!(
                        "Suppressing `{}` is forbidden. Remove the suppression or address the issue.",
                        code
                    );
                    (format!("{}_forbidden", violation_type_prefix), advice)
                }
                SuppressViolationKind::MissingComment {
                    ref lint_code,
                    ref required_patterns,
                } => {
                    let advice = build_suppress_missing_comment_advice(
                        language,
                        lint_code.as_deref(),
                        required_patterns,
                    );
                    (format!("{}_missing_comment", violation_type_prefix), advice)
                }
                SuppressViolationKind::AllForbidden => {
                    let advice =
                        "Lint suppressions are forbidden. Remove and fix the underlying issue.";
                    (
                        format!("{}_forbidden", violation_type_prefix),
                        advice.to_string(),
                    )
                }
            };

            if let Some(v) = try_create_violation(
                ctx,
                path,
                (directive.line + 1) as u32,
                &violation_type,
                &advice,
                &directive.pattern,
            ) {
                violations.push(v);
            } else {
                *limit_reached = true;
            }
        }
    }

    violations
}

// =============================================================================
// Original Shared Types and Functions
// =============================================================================

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
        /// The lint code being suppressed (for lint-specific guidance).
        lint_code: Option<String>,
        /// The required comment patterns (if any).
        required_patterns: Vec<String>,
    },
    /// All suppressions are forbidden at this scope level.
    AllForbidden,
}

/// Lint-specific fix guidance.
/// Returns (primary_fix_instruction, context_guidance).
fn get_rust_fix_guidance(lint_code: &str) -> (&'static str, &'static str) {
    match lint_code {
        "dead_code" => (
            "Remove this dead code.",
            "Dead code should be deleted to keep the codebase clean and maintainable.",
        ),
        "clippy::too_many_arguments" => (
            "Refactor this function to use fewer arguments.",
            "Consider grouping related parameters into a struct or using the builder pattern.",
        ),
        "clippy::cast_possible_truncation" => (
            "Verify this cast is safe and won't truncate data.",
            "Add explicit bounds checking or use safe conversion methods (e.g., try_into).",
        ),
        "deprecated" => (
            "Replace this deprecated API with the recommended alternative.",
            "Check the deprecation notice for the replacement API.",
        ),
        _ => (
            "Fix the underlying issue instead of suppressing the lint.",
            "Suppressions should only be used when the lint is a false positive.",
        ),
    }
}

/// Lint-specific fix guidance for Shell lints.
fn get_shell_fix_guidance(lint_code: &str) -> (&'static str, &'static str) {
    match lint_code {
        "SC2034" => (
            "Remove this unused variable.",
            "If the variable is used externally, export it or add a comment explaining its purpose.",
        ),
        "SC2086" => (
            "Quote the variable expansion to prevent word splitting.",
            "Use \"$var\" instead of $var unless word splitting is intentionally needed.",
        ),
        "SC2154" => (
            "Define this variable before use or document its external source.",
            "If set by the shell environment, add a comment explaining where it comes from.",
        ),
        "SC2155" => (
            "Split the declaration and assignment into separate statements.",
            "This allows error checking on the command substitution.",
        ),
        _ => (
            "Fix the ShellCheck warning instead of suppressing it.",
            "ShellCheck warnings usually indicate real issues or portability problems.",
        ),
    }
}

/// Lint-specific fix guidance for Go lints.
fn get_go_fix_guidance(lint_code: &str) -> (&'static str, &'static str) {
    match lint_code {
        "errcheck" => (
            "Handle this error properly.",
            "Add error handling or explicitly check and handle the error case.",
        ),
        "gosec" => (
            "Address the security issue identified by gosec.",
            "Review the security finding and apply the recommended fix.",
        ),
        _ => (
            "Fix the underlying issue instead of suppressing the lint.",
            "Suppressions should only be used when the lint is a false positive.",
        ),
    }
}

/// Lint-specific fix guidance for JavaScript/TypeScript lints.
fn get_js_fix_guidance(lint_code: &str) -> (&'static str, &'static str) {
    match lint_code {
        "no-console" => (
            "Remove console.log statements from production code.",
            "Use a proper logging library or remove debugging statements.",
        ),
        "no-explicit-any"
        | "@typescript-eslint/no-explicit-any"
        | "lint/suspicious/noExplicitAny" => (
            "Replace 'any' with a proper type.",
            "Use specific types, generics, or 'unknown' for better type safety.",
        ),
        "no-unused-vars" | "@typescript-eslint/no-unused-vars" => (
            "Remove this unused variable.",
            "If needed for future use, prefix with underscore or remove until actually needed.",
        ),
        _ => (
            "Fix the underlying issue instead of suppressing the lint.",
            "Suppressions should only be used when the lint is a false positive.",
        ),
    }
}

/// Format suppression instructions as a last resort.
///
/// Always presents suppression with justification as the fallback option
/// after attempting to fix the underlying issue.
fn format_suppression_fallback(patterns: &[String]) -> String {
    if patterns.is_empty() {
        return String::new();
    }

    if patterns.len() == 1 {
        // Single pattern
        format!(
            "Only if fixing is not feasible, add:\n  {} ...",
            patterns[0]
        )
    } else {
        // Multiple patterns
        let formatted_patterns = patterns
            .iter()
            .map(|p| format!("  {} ...", p))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Only if fixing is not feasible, add one of:\n{}", formatted_patterns)
    }
}

/// Build the fix-first suppress missing comment advice message.
///
/// New format encourages fixing the underlying issue first:
/// 1. Primary instruction: Fix the issue (imperative, actionable)
/// 2. Context/guidance: Why and how to fix it
/// 3. Last resort: Only if fixing is not feasible, add justification comment
pub fn build_suppress_missing_comment_advice(
    language: &str,
    lint_code: Option<&str>,
    required_patterns: &[String],
) -> String {
    let mut parts = Vec::new();

    // Get fix-first guidance
    let (primary_fix, context) = if let Some(code) = lint_code {
        match language {
            "rust" => get_rust_fix_guidance(code),
            "shell" => get_shell_fix_guidance(code),
            "go" => get_go_fix_guidance(code),
            "javascript" => get_js_fix_guidance(code),
            _ => (
                "Fix the underlying issue instead of suppressing the lint.",
                "Suppressions should only be used when the lint is a false positive.",
            ),
        }
    } else {
        (
            "Fix the underlying issue instead of suppressing the lint.",
            "Suppressions should only be used when the lint is a false positive.",
        )
    };

    // Part 1: Primary fix instruction
    parts.push(primary_fix.to_string());

    // Part 2: Context and guidance
    parts.push(context.to_string());

    // Part 3: Suppression as last resort
    if !required_patterns.is_empty() {
        parts.push(format_suppression_fallback(required_patterns));
    } else {
        // No specific patterns - generic fallback
        let msg = match language {
            "rust" => "Only if the lint is a false positive, add a comment above the attribute.",
            "shell" => "Only if the lint is a false positive, add a comment above the directive.",
            "go" => "Only if the lint is a false positive, add a comment above the directive or inline (//nolint:code // reason).",
            "javascript" => "Only if the lint is a false positive, add a comment above the directive or use inline reason (-- reason).",
            _ => "Only if the lint is a false positive, add a comment above the directive.",
        };
        parts.push(msg.to_string());
    }

    parts.join("\n")
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
        let (lint_code, required_patterns) = find_required_patterns(params, attr);
        if !has_valid_comment(attr, &required_patterns) {
            return Some(SuppressViolationKind::MissingComment {
                lint_code,
                required_patterns,
            });
        }
    }

    None
}

/// Find the required comment patterns for an attribute.
/// Checks per-lint patterns first, then falls back to global.
/// Returns the lint code (if found) and a list of valid patterns (any match is acceptable).
fn find_required_patterns(
    params: &SuppressCheckParams,
    attr: &SuppressAttrInfo,
) -> (Option<String>, Vec<String>) {
    // Check per-lint patterns first (first matching code wins)
    for code in attr.codes {
        if let Some(patterns) = params.scope_config.patterns.get(code) {
            return (Some(code.clone()), patterns.clone());
        }
    }
    // Fall back to global pattern
    let patterns = params
        .global_comment
        .map(|p| vec![p.to_string()])
        .unwrap_or_default();
    (attr.codes.first().cloned(), patterns)
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
