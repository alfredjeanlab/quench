// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shellcheck suppress directive parsing.
//!
//! Parses `# shellcheck disable=SC2034,SC2086` comments in shell scripts.

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// Shellcheck suppress directive found in source code.
#[derive(Debug, Clone)]
pub struct ShellcheckSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Shellcheck codes being suppressed (e.g., ["SC2034", "SC2086"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse shellcheck suppress directives from shell source.
pub fn parse_shellcheck_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<ShellcheckSuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match # shellcheck disable=...
        if let Some(codes) = parse_shellcheck_disable(trimmed) {
            let (has_comment, comment_text) = check_justification_comment(
                &lines,
                line_idx,
                comment_pattern,
                &CommentStyle::SHELL,
            );

            suppresses.push(ShellcheckSuppress {
                line: line_idx,
                codes,
                has_comment,
                comment_text,
            });
        }
    }

    suppresses
}

/// Parse shellcheck disable directive from a single line.
/// Returns list of codes if found (e.g., ["SC2034", "SC2086"]).
fn parse_shellcheck_disable(line: &str) -> Option<Vec<String>> {
    // Match: # shellcheck disable=SC2034 or # shellcheck disable=SC2034,SC2086
    // Also handles: #shellcheck disable=... (no space after #)
    let line = line.trim_start_matches('#').trim();

    if !line.starts_with("shellcheck") {
        return None;
    }

    let rest = line.strip_prefix("shellcheck")?.trim();

    // Must be "disable=" directive (not "source=" or other)
    let codes_str = rest.strip_prefix("disable=")?;

    // Strip inline comment if present (e.g., "SC2090  # explanation")
    let codes_str = codes_str.split('#').next().unwrap_or(codes_str);

    let codes: Vec<String> =
        codes_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    if codes.is_empty() {
        return None;
    }

    Some(codes)
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
