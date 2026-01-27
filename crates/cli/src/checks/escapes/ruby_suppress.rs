//! Ruby RuboCop/Standard suppress directive checking.

use std::path::Path;

use crate::adapter::parse_ruby_suppresses;
use crate::check::{CheckContext, Violation};
use crate::config::{RubySuppressConfig, SuppressLevel};

use super::suppress_common::{
    SuppressAttrInfo, SuppressCheckParams, SuppressViolationKind, check_suppress_attr,
};
use super::try_create_violation;

/// Check RuboCop/Standard suppress directives in a Ruby file.
pub(super) fn check_ruby_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &RubySuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Get scope config and check level
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

    // Parse RuboCop/Standard suppress directives
    let suppresses = parse_ruby_suppresses(content, None);

    for suppress in suppresses {
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
            codes: &suppress.codes,
            has_comment: suppress.has_comment,
            comment_text: suppress.comment_text.as_deref(),
        };

        // Use shared checking logic
        if let Some(violation_kind) = check_suppress_attr(&params, &attr_info) {
            // Build pattern string for violation
            let code = suppress
                .codes
                .first()
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            let directive_type = if suppress.is_todo { "todo" } else { "disable" };
            let pattern = format!("# {}:{} {}", suppress.kind, directive_type, code);

            let (violation_type, advice) = match violation_kind {
                SuppressViolationKind::Forbidden { ref code } => {
                    let advice = format!(
                        "Suppressing {} {} is forbidden. Remove the suppression or fix the issue.",
                        suppress.kind, code
                    );
                    ("suppress_forbidden", advice)
                }
                SuppressViolationKind::MissingComment {
                    ref lint_code,
                    ref required_patterns,
                } => {
                    let advice = super::suppress_common::build_suppress_missing_comment_advice(
                        "ruby",
                        lint_code.as_deref(),
                        required_patterns,
                    );
                    ("suppress_missing_comment", advice)
                }
                SuppressViolationKind::AllForbidden => {
                    let advice = format!(
                        "{} suppressions are forbidden. Fix the underlying issue {} instead of disabling it.",
                        suppress.kind, code
                    );
                    ("suppress_forbidden", advice)
                }
            };

            if let Some(v) = try_create_violation(
                ctx,
                path,
                (suppress.line + 1) as u32,
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
