// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python suppress directive parsing.
//!
//! Parses lint suppression comments in Python files:
//! - `# noqa` / `# noqa: E501` / `# noqa: E501, W503`
//! - `# type: ignore` / `# type: ignore[assignment]`
//! - `# pylint: disable=line-too-long`
//! - `# pragma: no cover`

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// Python suppress directive found in source code.
#[derive(Debug, Clone)]
pub struct PythonSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Directive type.
    pub kind: PythonSuppressKind,
    /// Codes being suppressed (e.g., ["E501"], ["assignment"]).
    /// Empty if suppressing all (blanket suppress).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Kind of Python suppress directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonSuppressKind {
    /// `# noqa` - flake8/ruff lint suppression
    Noqa,
    /// `# type: ignore` - mypy type checking suppression
    TypeIgnore,
    /// `# pylint: disable=...` - pylint suppression
    PylintDisable,
    /// `# pragma: no cover` - coverage exclusion
    PragmaNoCover,
}

impl std::fmt::Display for PythonSuppressKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Noqa => write!(f, "noqa"),
            Self::TypeIgnore => write!(f, "type: ignore"),
            Self::PylintDisable => write!(f, "pylint: disable"),
            Self::PragmaNoCover => write!(f, "pragma: no cover"),
        }
    }
}

/// Parse Python suppress directives from source.
pub fn parse_python_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<PythonSuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(suppress) = parse_python_directive(line, line_idx, &lines, comment_pattern) {
            suppresses.push(suppress);
        }
    }

    suppresses
}

/// Parse a Python suppress directive from a single line.
fn parse_python_directive(
    line: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<PythonSuppress> {
    let trimmed = line.trim();

    // Find the comment portion of the line
    let comment_start = if trimmed.starts_with('#') { Some(0) } else { trimmed.find('#') };

    let comment_start = comment_start?;
    let comment = &trimmed[comment_start..];
    let comment_content = comment.trim_start_matches('#').trim();

    // Try to parse different directive types
    if let Some(suppress) = parse_noqa(comment_content, line_idx, lines, comment_pattern) {
        return Some(suppress);
    }

    if let Some(suppress) = parse_type_ignore(comment_content, line_idx, lines, comment_pattern) {
        return Some(suppress);
    }

    if let Some(suppress) = parse_pylint_disable(comment_content, line_idx, lines, comment_pattern)
    {
        return Some(suppress);
    }

    if let Some(suppress) = parse_pragma_no_cover(comment_content, line_idx, lines, comment_pattern)
    {
        return Some(suppress);
    }

    None
}

/// Parse `# noqa` / `# noqa: E501` / `# noqa: E501, W503`
fn parse_noqa(
    comment_content: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<PythonSuppress> {
    // Match "noqa" at the start (case-insensitive)
    let lower = comment_content.to_lowercase();
    if !lower.starts_with("noqa") {
        return None;
    }

    let rest = &comment_content[4..]; // Skip "noqa"

    // Parse codes if present
    let codes = if let Some(codes_str) = rest.strip_prefix(':') {
        // Has specific codes: "noqa: E501" or "noqa: E501, W503"
        parse_comma_separated_codes(codes_str)
    } else if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        // Blanket noqa
        Vec::new()
    } else {
        // Not a valid noqa (e.g., "noqaX")
        return None;
    };

    // Check for justification comment
    let (has_comment, comment_text) =
        check_justification_comment(lines, line_idx, comment_pattern, &CommentStyle::PYTHON);

    Some(PythonSuppress {
        line: line_idx,
        kind: PythonSuppressKind::Noqa,
        codes,
        has_comment,
        comment_text,
    })
}

/// Parse `# type: ignore` / `# type: ignore[assignment]` / `# type: ignore[arg-type, return-value]`
fn parse_type_ignore(
    comment_content: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<PythonSuppress> {
    // Match "type: ignore" or "type:ignore" (with optional space)
    let lower = comment_content.to_lowercase();
    let rest = if lower.starts_with("type: ignore") {
        &comment_content[12..] // Skip "type: ignore"
    } else if lower.starts_with("type:ignore") {
        &comment_content[11..] // Skip "type:ignore"
    } else {
        return None;
    };

    // Parse codes if present in brackets
    let codes = if let Some(start) = rest.find('[') {
        if let Some(end) = rest.find(']') {
            let codes_str = &rest[start + 1..end];
            parse_comma_separated_codes(codes_str)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Check for justification comment
    let (has_comment, comment_text) =
        check_justification_comment(lines, line_idx, comment_pattern, &CommentStyle::PYTHON);

    Some(PythonSuppress {
        line: line_idx,
        kind: PythonSuppressKind::TypeIgnore,
        codes,
        has_comment,
        comment_text,
    })
}

/// Parse `# pylint: disable=line-too-long` / `# pylint: disable=all`
fn parse_pylint_disable(
    comment_content: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<PythonSuppress> {
    // Match "pylint: disable=" or "pylint:disable="
    let lower = comment_content.to_lowercase();
    let rest = if lower.starts_with("pylint: disable=") {
        &comment_content[16..] // Skip "pylint: disable="
    } else if lower.starts_with("pylint:disable=") {
        &comment_content[15..] // Skip "pylint:disable="
    } else {
        return None;
    };

    // Parse codes (comma-separated)
    let codes = parse_comma_separated_codes(rest);

    if codes.is_empty() {
        return None;
    }

    // Check for justification comment
    let (has_comment, comment_text) =
        check_justification_comment(lines, line_idx, comment_pattern, &CommentStyle::PYTHON);

    Some(PythonSuppress {
        line: line_idx,
        kind: PythonSuppressKind::PylintDisable,
        codes,
        has_comment,
        comment_text,
    })
}

/// Parse `# pragma: no cover`
fn parse_pragma_no_cover(
    comment_content: &str,
    line_idx: usize,
    lines: &[&str],
    comment_pattern: Option<&str>,
) -> Option<PythonSuppress> {
    // Match "pragma: no cover" or "pragma:no cover"
    let lower = comment_content.to_lowercase();
    if !lower.starts_with("pragma: no cover") && !lower.starts_with("pragma:no cover") {
        return None;
    }

    // Check for justification comment
    let (has_comment, comment_text) =
        check_justification_comment(lines, line_idx, comment_pattern, &CommentStyle::PYTHON);

    Some(PythonSuppress {
        line: line_idx,
        kind: PythonSuppressKind::PragmaNoCover,
        codes: vec!["coverage".to_string()],
        has_comment,
        comment_text,
    })
}

/// Parse comma-separated codes, trimming whitespace.
fn parse_comma_separated_codes(codes_str: &str) -> Vec<String> {
    // Stop at any trailing comment marker
    let codes_str = codes_str.split('#').next().unwrap_or(codes_str);

    codes_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
