// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Suppress attribute parsing.
//!
//! Parses #[allow(...)] and #[expect(...)] attributes in Rust source.

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// Suppress attribute found in source code.
#[derive(Debug, Clone)]
pub struct SuppressAttr {
    /// Line number (0-indexed).
    pub line: usize,
    /// Attribute type: "allow" or "expect".
    pub kind: &'static str,
    /// Lint codes being suppressed (e.g., ["dead_code", "unused"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse suppress attributes from Rust source.
pub fn parse_suppress_attrs(content: &str, comment_pattern: Option<&str>) -> Vec<SuppressAttr> {
    let mut attrs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match #[allow(...)] or #[expect(...)]
        if let Some(attr) = parse_suppress_line(trimmed) {
            // Check for justification comment above
            let (has_comment, comment_text) =
                check_justification_comment(&lines, line_idx, comment_pattern, &CommentStyle::RUST);

            attrs.push(SuppressAttr {
                line: line_idx,
                kind: attr.kind,
                codes: attr.codes,
                has_comment,
                comment_text,
            });
        }
    }

    attrs
}

/// Parsed attribute info from a single line.
struct ParsedAttr {
    kind: &'static str,
    codes: Vec<String>,
}

/// Parse a single line for suppress attribute.
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    // Match #[allow(code1, code2)] or #[expect(code1, code2)]
    let kind = if line.starts_with("#[allow(") {
        "allow"
    } else if line.starts_with("#[expect(") {
        "expect"
    } else {
        return None;
    };

    // Extract codes between parentheses
    let start = line.find('(')? + 1;
    let end = line.rfind(')')?;
    if start >= end {
        return None;
    }

    let codes_str = &line[start..end];
    let codes: Vec<String> = codes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Some(ParsedAttr { kind, codes })
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
