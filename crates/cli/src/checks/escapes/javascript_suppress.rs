// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript lint suppression directive checking for the escapes check.
//!
//! Checks `eslint-disable`, `eslint-disable-next-line`, and `biome-ignore` directives
//! and enforces comment requirements.

use std::path::Path;

use crate::adapter::javascript::{SuppressTool, parse_javascript_suppresses};
use crate::check::{CheckContext, Violation};
use crate::config::JavaScriptSuppressConfig;

use super::suppress_common::{UnifiedSuppressDirective, check_suppress_violations_generic};

/// Check JavaScript suppress directives and return violations.
pub fn check_javascript_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &JavaScriptSuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    // Parse JavaScript suppress directives
    let directives = parse_javascript_suppresses(content, config.comment.as_deref());

    // Convert to unified format
    let unified: Vec<UnifiedSuppressDirective> = directives
        .into_iter()
        .map(|d| {
            let pattern = match d.tool {
                SuppressTool::Eslint => {
                    if d.codes.is_empty() {
                        "eslint-disable".to_string()
                    } else {
                        format!("eslint-disable-next-line {}", d.codes.join(", "))
                    }
                }
                SuppressTool::Biome => {
                    if d.codes.is_empty() {
                        "biome-ignore".to_string()
                    } else {
                        format!("biome-ignore {}", d.codes.join(" "))
                    }
                }
            };
            UnifiedSuppressDirective {
                line: d.line,
                codes: d.codes,
                has_comment: d.has_comment,
                comment_text: d.comment_text,
                pattern,
            }
        })
        .collect();

    check_suppress_violations_generic(
        ctx,
        path,
        unified,
        config,
        "javascript",
        "suppress",
        is_test_file,
        limit_reached,
    )
}
