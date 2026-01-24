// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go nolint directive checking for the escapes check.
//!
//! Checks `//nolint` directives and enforces comment requirements.

use std::path::Path;
use std::sync::atomic::Ordering;

use crate::adapter::parse_nolint_directives;
use crate::check::{CheckContext, Violation};
use crate::config::{GoSuppressConfig, SuppressLevel};

/// Check Go nolint directives and return violations.
pub fn check_go_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &GoSuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Determine effective check level based on source vs test
    let effective_check = if is_test_file {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.source.check.unwrap_or(config.check)
    };

    // If allow, no checking needed
    if effective_check == SuppressLevel::Allow {
        return violations;
    }

    // Parse nolint directives
    let directives = parse_nolint_directives(content, config.comment.as_deref());

    for directive in directives {
        if *limit_reached {
            break;
        }

        // Get scope config (source or test)
        let scope_config = if is_test_file {
            &config.test
        } else {
            &config.source
        };

        // Format pattern for reporting
        let pattern = if directive.codes.is_empty() {
            "//nolint".to_string()
        } else {
            format!("//nolint:{}", directive.codes.join(","))
        };

        // Check each lint code (or all if codes is empty)
        let codes_to_check = if directive.codes.is_empty() {
            vec!["*".to_string()] // Represents "all linters"
        } else {
            directive.codes.clone()
        };

        for code in &codes_to_check {
            if *limit_reached {
                break;
            }

            // Check forbid list first
            if scope_config.forbid.contains(code) {
                let advice = format!(
                    "Suppressing `{}` is forbidden. Remove the suppression or address the issue.",
                    code
                );
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (directive.line + 1) as u32,
                    "suppress_forbidden",
                    &advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
                continue;
            }

            // Check allow list (skip comment check)
            if scope_config.allow.contains(code) {
                continue;
            }

            // Check if comment is required
            if effective_check == SuppressLevel::Comment && !directive.has_comment {
                let advice = if let Some(ref pat) = config.comment {
                    format!(
                        "Lint suppression requires justification. Add a {} comment or inline reason.",
                        pat
                    )
                } else {
                    "Lint suppression requires justification. Add a comment above the directive or inline (//nolint:code // reason).".into()
                };
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (directive.line + 1) as u32,
                    "suppress_missing_comment",
                    &advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
            }

            // Forbid level means all suppressions fail
            if effective_check == SuppressLevel::Forbid {
                let advice =
                    "Lint suppressions are forbidden. Remove and fix the underlying issue.";
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (directive.line + 1) as u32,
                    "suppress_forbidden",
                    advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
            }
        }
    }

    violations
}

fn try_create_violation(
    ctx: &CheckContext,
    path: &Path,
    line: u32,
    violation_type: &str,
    advice: &str,
    pattern_name: &str,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    Some(Violation::file(path, line, violation_type, advice).with_pattern(pattern_name))
}
