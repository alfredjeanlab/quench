// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python suppress directive checking for the escapes check.
//!
//! Checks `# noqa`, `# type: ignore`, `# pylint: disable`, and `# pragma: no cover` directives.

use std::path::Path;

use crate::adapter::python::{PythonSuppressKind, parse_python_suppresses};
use crate::check::{CheckContext, Violation};
use crate::config::PythonSuppressConfig;

use super::suppress_common::{UnifiedSuppressDirective, check_suppress_violations_generic};

/// Check Python suppress directives and return violations.
pub(super) fn check_python_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &PythonSuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    // Parse suppress directives
    let suppresses = parse_python_suppresses(content, config.comment.as_deref());

    // Convert to unified format
    let unified: Vec<UnifiedSuppressDirective> = suppresses
        .into_iter()
        .map(|s| {
            let pattern = match s.kind {
                PythonSuppressKind::Noqa => {
                    if s.codes.is_empty() {
                        "# noqa".to_string()
                    } else {
                        format!("# noqa: {}", s.codes.join(", "))
                    }
                }
                PythonSuppressKind::TypeIgnore => {
                    if s.codes.is_empty() {
                        "# type: ignore".to_string()
                    } else {
                        format!("# type: ignore[{}]", s.codes.join(", "))
                    }
                }
                PythonSuppressKind::PylintDisable => {
                    format!("# pylint: disable={}", s.codes.join(","))
                }
                PythonSuppressKind::PragmaNoCover => "# pragma: no cover".to_string(),
            };
            UnifiedSuppressDirective {
                line: s.line,
                codes: s.codes,
                has_comment: s.has_comment,
                comment_text: s.comment_text,
                pattern,
            }
        })
        .collect();

    check_suppress_violations_generic(
        ctx,
        path,
        unified,
        config,
        "python",
        "suppress",
        is_test_file,
        limit_reached,
    )
}
