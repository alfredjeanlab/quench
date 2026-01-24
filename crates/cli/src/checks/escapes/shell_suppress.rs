//! Shell shellcheck suppress directive checking.

use std::path::Path;

use crate::adapter::parse_shellcheck_suppresses;
use crate::check::{CheckContext, Violation};
use crate::config::{ShellSuppressConfig, SuppressLevel};

use super::suppress_common::{
    SuppressAttrInfo, SuppressCheckParams, SuppressViolationKind, check_suppress_attr,
};
use super::try_create_violation;

/// Check shellcheck suppress directives in a Shell file.
pub(super) fn check_shell_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &ShellSuppressConfig,
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

    // Parse shellcheck suppress directives (don't filter by global pattern - let checker handle per-lint patterns)
    let suppresses = parse_shellcheck_suppresses(content, None);

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
            let pattern = format!("# shellcheck disable={}", code);

            let (violation_type, advice) = match violation_kind {
                SuppressViolationKind::Forbidden { ref code } => {
                    let advice = format!(
                        "Suppressing shellcheck {} is forbidden. Remove the suppression or fix the issue.",
                        code
                    );
                    ("shellcheck_forbidden", advice)
                }
                SuppressViolationKind::MissingComment {
                    lint_code: _,
                    required_patterns,
                } => {
                    let advice = if let Some(pat) = required_patterns.first() {
                        format!(
                            "Shellcheck suppression requires justification. Add a {} comment.",
                            pat
                        )
                    } else {
                        "Shellcheck suppression requires justification. Add a comment above explaining why."
                            .to_string()
                    };
                    ("shellcheck_missing_comment", advice)
                }
                SuppressViolationKind::AllForbidden => {
                    let advice = format!(
                        "Shellcheck suppressions are forbidden. Fix the underlying issue {} instead of disabling it.",
                        code
                    );
                    ("shellcheck_forbidden", advice)
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
