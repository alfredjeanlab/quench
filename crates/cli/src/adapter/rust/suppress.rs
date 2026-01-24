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

/// State for tracking multi-line attribute parsing.
struct PendingAttr {
    /// Accumulated content of the attribute.
    content: String,
    /// Line where the attribute started.
    start_line: usize,
}

/// Parse suppress attributes from Rust source.
/// Handles both single-line and multi-line attributes.
pub fn parse_suppress_attrs(content: &str, comment_pattern: Option<&str>) -> Vec<SuppressAttr> {
    let mut attrs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut pending: Option<PendingAttr> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Handle multi-line attribute accumulation
        if let Some(ref mut p) = pending {
            p.content.push(' ');
            p.content.push_str(trimmed);

            // Check if the attribute is complete (has closing paren or bracket)
            if trimmed.contains(')') || trimmed.contains(")]") {
                // Attribute complete, parse it
                if let Some(attr) = parse_suppress_line(&p.content) {
                    // Check for justification comment above the start line
                    let (has_comment, comment_text) = check_justification_comment(
                        &lines,
                        p.start_line,
                        comment_pattern,
                        &CommentStyle::RUST,
                    );

                    attrs.push(SuppressAttr {
                        line: p.start_line,
                        kind: attr.kind,
                        codes: attr.codes,
                        has_comment,
                        comment_text,
                    });
                }
                pending = None;
            }
            continue;
        }

        // Check if this line starts a suppress attribute
        if let Some(attr_start) = detect_suppress_attr_start(trimmed) {
            if attr_start.is_complete {
                // Single-line attribute
                if let Some(attr) = parse_suppress_line(trimmed) {
                    let (has_comment, comment_text) = check_justification_comment(
                        &lines,
                        line_idx,
                        comment_pattern,
                        &CommentStyle::RUST,
                    );

                    attrs.push(SuppressAttr {
                        line: line_idx,
                        kind: attr.kind,
                        codes: attr.codes,
                        has_comment,
                        comment_text,
                    });
                }
            } else {
                // Multi-line attribute starts here
                pending = Some(PendingAttr {
                    content: trimmed.to_string(),
                    start_line: line_idx,
                });
            }
        }
    }

    attrs
}

/// Result of detecting a suppress attribute start.
struct AttrStart {
    /// True if the attribute is complete on this line.
    is_complete: bool,
}

/// Detect if a line starts a #[allow(...)] or #[expect(...)] attribute.
fn detect_suppress_attr_start(line: &str) -> Option<AttrStart> {
    // Check for allow or expect patterns (both outer and inner attributes)
    let starts_attr = line.starts_with("#[allow(")
        || line.starts_with("#![allow(")
        || line.starts_with("#[expect(")
        || line.starts_with("#![expect(");

    if !starts_attr {
        return None;
    }

    // Check if the attribute is complete on this line
    // Need to find the matching closing paren/bracket
    let is_complete = line.contains(')') || line.contains(")]");

    Some(AttrStart { is_complete })
}

/// Parsed attribute info from a single line.
struct ParsedAttr {
    kind: &'static str,
    codes: Vec<String>,
}

/// Parse a single line for suppress attribute.
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    // Match both outer (#[...]) and inner (#![...]) attributes
    let kind = if line.starts_with("#[allow(") || line.starts_with("#![allow(") {
        "allow"
    } else if line.starts_with("#[expect(") || line.starts_with("#![expect(") {
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
