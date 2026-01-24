// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go nolint directive checking for the escapes check.
//!
//! Checks `//nolint` directives and enforces comment requirements.

use std::path::Path;

use crate::adapter::parse_nolint_directives;
use crate::check::{CheckContext, Violation};
use crate::config::{GoSuppressConfig, SuppressLevel};

use super::suppress_common::{
    SuppressAttrInfo, SuppressCheckParams, SuppressViolationKind, check_suppress_attr,
};
use super::try_create_violation;

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

    // Get scope config (source or test)
    let (scope_config, scope_check) = if is_test_file {
        (
            &config.test,
            config.test.check.unwrap_or(SuppressLevel::Allow),
        )
    } else {
        (&config.source, config.source.check.unwrap_or(config.check))
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
            global_comment: config.comment.as_deref(),
        };

        let attr_info = SuppressAttrInfo {
            codes: &directive.codes,
            has_comment: directive.has_comment,
            comment_text: directive.comment_text.as_deref(),
        };

        // Use shared checking logic
        if let Some(violation_kind) = check_suppress_attr(&params, &attr_info) {
            // Format pattern for reporting
            let pattern = if directive.codes.is_empty() {
                "//nolint".to_string()
            } else {
                format!("//nolint:{}", directive.codes.join(","))
            };

            let (violation_type, advice) = match violation_kind {
                SuppressViolationKind::Forbidden { ref code } => {
                    let advice = format!(
                        "Suppressing `{}` is forbidden. Remove the suppression or address the issue.",
                        code
                    );
                    ("suppress_forbidden", advice)
                }
                SuppressViolationKind::MissingComment {
                    ref lint_code,
                    ref required_patterns,
                } => {
                    let advice = super::suppress_common::build_suppress_missing_comment_advice(
                        "go",
                        lint_code.as_deref(),
                        required_patterns,
                    );
                    ("suppress_missing_comment", advice)
                }
                SuppressViolationKind::AllForbidden => {
                    let advice =
                        "Lint suppressions are forbidden. Remove and fix the underlying issue.";
                    ("suppress_forbidden", advice.to_string())
                }
            };

            if let Some(v) = try_create_violation(
                ctx,
                path,
                (directive.line + 1) as u32,
                violation_type,
                &advice,
                &pattern,
            ) {
                violations.push(v);
            } else {
                *limit_reached = true;
            }
        }
    }

    violations
}
