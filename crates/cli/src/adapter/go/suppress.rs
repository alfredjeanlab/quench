// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go nolint directive parsing.
//!
//! Parses `//nolint` and `//nolint:code1,code2` directives in Go source.

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// Nolint directive found in Go source code.
#[derive(Debug, Clone)]
pub struct NolintDirective {
    /// Line number (0-indexed).
    pub line: usize,
    /// Linter codes being suppressed (empty = all linters).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse //nolint directives from Go source.
pub fn parse_nolint_directives(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<NolintDirective> {
    let mut directives = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match //nolint or //nolint:codes
        if let Some(parsed) = parse_nolint_line(trimmed) {
            let (has_comment, comment_text) = if parsed.has_inline_comment {
                // Inline comment counts as justification
                (true, parsed.inline_comment.clone())
            } else {
                check_justification_comment(&lines, line_idx, comment_pattern, &CommentStyle::GO)
            };

            directives.push(NolintDirective {
                line: line_idx,
                codes: parsed.codes,
                has_comment,
                comment_text,
            });
        }
    }

    directives
}

/// Parsed nolint directive from a single line.
struct ParsedNolint {
    codes: Vec<String>,
    has_inline_comment: bool,
    inline_comment: Option<String>,
}

/// Parse nolint directive from a single line.
/// Returns None if line doesn't contain //nolint.
fn parse_nolint_line(line: &str) -> Option<ParsedNolint> {
    // Find //nolint in the line
    let nolint_pos = line.find("//nolint")?;
    let rest = &line[nolint_pos + 8..]; // Skip "//nolint"

    // Check for inline comment after the directive
    // Format: //nolint:errcheck // reason here
    // Or: //nolint // reason here
    let (codes_part, inline_comment) = if let Some(comment_pos) = rest.find(" //") {
        let codes = &rest[..comment_pos];
        let comment = rest[comment_pos + 3..].trim();
        (codes, if comment.is_empty() { None } else { Some(comment.to_string()) })
    } else {
        (rest, None)
    };

    // Parse codes if present (format: :code1,code2)
    let codes = if let Some(codes_str) = codes_part.strip_prefix(':') {
        codes_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("//"))
            .collect()
    } else {
        Vec::new()
    };

    Some(ParsedNolint { codes, has_inline_comment: inline_comment.is_some(), inline_comment })
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
