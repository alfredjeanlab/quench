// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Comment detection utilities for escape pattern checking.

/// Check if there's a justifying comment for a pattern match.
///
/// The pattern must appear at the start of a comment's content, not embedded
/// within other text. For example, `// SAFETY:` embedded inside
/// `// VIOLATION: missing // SAFETY: comment` should NOT match.
///
/// Returns true if comment pattern is found at a valid comment boundary.
pub(super) fn has_justification_comment(
    content: &str,
    match_line: u32,
    comment_pattern: &str,
) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (match_line - 1) as usize;

    // Check same line first - look for pattern at start of inline comment
    if line_idx < lines.len() {
        let line = lines[line_idx];
        if let Some(comment_start) = find_comment_start(line) {
            let comment = &line[comment_start..];
            if comment_starts_with_pattern(comment, comment_pattern) {
                return true;
            }
        }
    }

    // Search upward through comments and blank lines
    if line_idx > 0 {
        for i in (0..line_idx).rev() {
            let line = lines[i].trim();

            // Check for comment pattern at start of comment
            if is_comment_line(line) && comment_starts_with_pattern(line, comment_pattern) {
                return true;
            }

            // Stop at non-blank, non-comment lines
            if !line.is_empty() && !is_comment_line(line) {
                break;
            }
        }
    }

    false
}

/// Check if comment content starts with the pattern (after comment marker).
///
/// Normalizes both the comment and pattern by stripping comment markers,
/// then checks if the comment content starts with the pattern content.
fn comment_starts_with_pattern(comment: &str, pattern: &str) -> bool {
    let comment_content = strip_comment_markers(comment);
    let pattern_content = strip_comment_markers(pattern);
    comment_content.starts_with(&pattern_content)
}

/// Strip comment markers and leading whitespace to get the content.
pub(super) fn strip_comment_markers(s: &str) -> String {
    let trimmed = s.trim();
    // Strip various comment markers
    let content = trimmed
        .strip_prefix("///")
        .or_else(|| trimmed.strip_prefix("//!"))
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix("/*"))
        .or_else(|| trimmed.strip_prefix('#'))
        .or_else(|| trimmed.strip_prefix("--"))
        .or_else(|| trimmed.strip_prefix(";;"))
        .or_else(|| trimmed.strip_prefix('*'))
        .unwrap_or(trimmed);
    content.trim_start().to_string()
}

/// Find the start of a comment in a line (returns byte offset of comment marker).
fn find_comment_start(line: &str) -> Option<usize> {
    // Find // comment (most common)
    if let Some(pos) = line.find("//") {
        return Some(pos);
    }
    // Find # comment (shell/Python) - avoid matching # in strings
    if let Some(pos) = line.find('#') {
        // Simple heuristic: # at start or preceded by whitespace
        if pos == 0 || line.as_bytes().get(pos.saturating_sub(1)) == Some(&b' ') {
            return Some(pos);
        }
    }
    None
}

/// Check if a line is a comment line (language-agnostic heuristics).
pub(super) fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//")      // C-style
        || trimmed.starts_with('#')   // Shell/Python/Ruby
        || trimmed.starts_with("/*")  // C block comment start
        || trimmed.starts_with('*')   // C block comment continuation
        || trimmed.starts_with("--")  // SQL/Lua
        || trimmed.starts_with(";;") // Lisp
}

/// Check if a match at a given offset within a line is inside a comment.
///
/// Returns true if the match is entirely within the comment portion of the line.
/// This helps avoid false positives when patterns appear in comments but not in code.
///
/// Special case: Returns false for Go directive patterns (//go:) at the start of a line,
/// since these are intentionally comment-like but should be detected as escape patterns.
pub(super) fn is_match_in_comment(line_content: &str, match_offset_in_line: usize) -> bool {
    // Find comment start in the line
    if let Some(comment_start) = find_comment_start(line_content) {
        // Special case: Go directive patterns at line start should be detected, not skipped.
        // These look like comments but are actually compiler directives we want to check.
        if match_offset_in_line == comment_start && is_go_directive(line_content) {
            return false;
        }
        // If match starts at or after the comment marker, it's in a comment
        return match_offset_in_line >= comment_start;
    }
    false
}

/// Check if a line is a Go compiler directive (//go:xxx).
fn is_go_directive(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//go:")
}
