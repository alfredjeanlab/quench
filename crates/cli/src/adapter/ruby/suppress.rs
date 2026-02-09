// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! RuboCop/Standard suppress directive parsing.
//!
//! Parses `# rubocop:disable Style/StringLiterals` and similar comments in Ruby files.

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// RuboCop/Standard suppress directive found in source code.
#[derive(Debug, Clone)]
pub struct RubySuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Directive type: "rubocop" or "standard"
    pub kind: RubySuppressKind,
    /// Cop codes being suppressed (e.g., ["Style/StringLiterals"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
    /// Whether this is a "todo" directive.
    pub is_todo: bool,
}

/// Kind of Ruby suppress directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RubySuppressKind {
    Rubocop,
    Standard,
}

impl std::fmt::Display for RubySuppressKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rubocop => write!(f, "rubocop"),
            Self::Standard => write!(f, "standard"),
        }
    }
}

/// Parse RuboCop/Standard suppress directives from Ruby source.
pub fn parse_ruby_suppresses(content: &str, comment_pattern: Option<&str>) -> Vec<RubySuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(suppress) = parse_ruby_directive(line, line_idx, &lines, comment_pattern) {
            suppresses.push(suppress);
        }
    }

    suppresses
}

/// Parse a RuboCop/Standard directive from a single line.
fn parse_ruby_directive(
    line: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<RubySuppress> {
    let trimmed = line.trim();

    // Find the comment portion of the line
    // Could be a full line comment or inline comment
    let comment_start = if trimmed.starts_with('#') { Some(0) } else { trimmed.find('#') };

    let comment_start = comment_start?;
    let comment = &trimmed[comment_start..];

    // Parse the directive from the comment
    let comment_content = comment.trim_start_matches('#').trim();

    // Check for rubocop: or standard: prefix
    let (kind, rest, is_todo) = if let Some(rest) = comment_content.strip_prefix("rubocop:") {
        let rest = rest.trim();
        if let Some(codes_str) = rest.strip_prefix("disable") {
            (RubySuppressKind::Rubocop, codes_str.trim(), false)
        } else if let Some(codes_str) = rest.strip_prefix("todo") {
            (RubySuppressKind::Rubocop, codes_str.trim(), true)
        } else {
            return None; // Not a disable or todo directive
        }
    } else if let Some(rest) = comment_content.strip_prefix("standard:") {
        let rest = rest.trim();
        if let Some(codes_str) = rest.strip_prefix("disable") {
            (RubySuppressKind::Standard, codes_str.trim(), false)
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Parse the cop codes
    // Strip any inline comment after the codes
    let codes_str = rest.split('#').next().unwrap_or(rest).trim();

    let codes: Vec<String> =
        codes_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    if codes.is_empty() {
        return None;
    }

    // Check for justification comment
    let (has_comment, comment_text) =
        check_justification_comment(lines, line_idx, comment_pattern, &CommentStyle::RUBY);

    Some(RubySuppress { line: line_idx, kind, codes, has_comment, comment_text, is_todo })
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
