// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript lint suppression directive checking for the escapes check.
//!
//! Checks `eslint-disable`, `eslint-disable-next-line`, and `biome-ignore` directives
//! and enforces comment requirements.

use std::path::Path;

use crate::adapter::javascript::{SuppressTool, parse_javascript_suppresses};
use crate::check::{CheckContext, Violation};
use crate::config::{SuppressConfig, SuppressLevel};

use super::suppress_common::{
    SuppressAttrInfo, SuppressCheckParams, SuppressViolationKind, check_suppress_attr,
};
use super::try_create_violation;

/// Get lint-specific guidance for JavaScript/TypeScript lints.
fn get_js_lint_guidance(lint_code: &str) -> &'static str {
    match lint_code {
        "no-console" => "Is this console output needed in production?",
        "no-explicit-any"
        | "@typescript-eslint/no-explicit-any"
        | "lint/suspicious/noExplicitAny" => "Can this be properly typed instead?",
        "no-unused-vars" | "@typescript-eslint/no-unused-vars" => "Is this variable still needed?",
        _ => "Is this suppression necessary?",
    }
}

/// Build the three-part suppress missing comment advice message for JavaScript.
fn build_js_missing_comment_advice(
    lint_code: Option<&str>,
    required_patterns: &[String],
) -> String {
    let mut parts = Vec::new();

    // Part 1: General statement
    parts.push("Lint suppression requires justification.".to_string());

    // Part 2: Lint-specific guidance
    let guidance = if let Some(code) = lint_code {
        get_js_lint_guidance(code)
    } else {
        "Is this suppression necessary?"
    };
    parts.push(guidance.to_string());

    // Part 3: Pattern instructions
    if !required_patterns.is_empty() {
        if required_patterns.len() == 1 {
            parts.push(format!(
                "Add a comment above or use inline reason (e.g., `// {} ...` or `-- reason`).",
                required_patterns[0]
            ));
        } else {
            let formatted = required_patterns
                .iter()
                .map(|p| format!("  {} ...", p))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("Add a comment above with one of:\n{}", formatted));
        }
    } else {
        parts.push(
            "Add a comment above the directive or use inline reason (-- reason).".to_string(),
        );
    }

    parts.join("\n")
}

/// Check JavaScript suppress directives and return violations.
pub fn check_javascript_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &SuppressConfig,
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

    // Parse JavaScript suppress directives
    let directives = parse_javascript_suppresses(content, config.comment.as_deref());

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
            let pattern = match directive.tool {
                SuppressTool::Eslint => {
                    if directive.codes.is_empty() {
                        "eslint-disable".to_string()
                    } else {
                        format!("eslint-disable-next-line {}", directive.codes.join(", "))
                    }
                }
                SuppressTool::Biome => {
                    if directive.codes.is_empty() {
                        "biome-ignore".to_string()
                    } else {
                        format!("biome-ignore {}", directive.codes.join(" "))
                    }
                }
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
                    let advice =
                        build_js_missing_comment_advice(lint_code.as_deref(), required_patterns);
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
