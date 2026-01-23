// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Common suppress directive utilities.

/// Comment style configuration for different languages.
pub struct CommentStyle {
    /// Comment line prefix (e.g., "//" for Rust, "#" for Shell).
    pub prefix: &'static str,
    /// Patterns that indicate a directive line, not a justification comment.
    pub directive_patterns: &'static [&'static str],
}

impl CommentStyle {
    /// Rust comment style: `//` prefix, `#[` directives.
    pub const RUST: Self = Self {
        prefix: "//",
        directive_patterns: &["#["],
    };

    /// Go comment style: `//` prefix, `//go:` directives.
    pub const GO: Self = Self {
        prefix: "//",
        directive_patterns: &["//go:", "//nolint"],
    };

    /// Shell comment style: `#` prefix, `shellcheck` directives.
    pub const SHELL: Self = Self {
        prefix: "#",
        directive_patterns: &["shellcheck"],
    };
}

/// Check if there's a justification comment above a directive line.
///
/// Walks backward from `directive_line` looking for a comment that serves
/// as justification. Stops at blank lines or non-comment code.
///
/// # Arguments
/// * `lines` - All lines of the source file
/// * `directive_line` - Line index of the directive (0-indexed)
/// * `required_pattern` - Optional pattern the comment must match
/// * `style` - Language-specific comment style
///
/// # Returns
/// A tuple of (found justification, comment text if found)
pub fn check_justification_comment(
    lines: &[&str],
    directive_line: usize,
    required_pattern: Option<&str>,
    style: &CommentStyle,
) -> (bool, Option<String>) {
    let mut check_line = directive_line;

    while check_line > 0 {
        check_line -= 1;
        let line = lines[check_line].trim();

        // Stop at blank lines
        if line.is_empty() {
            break;
        }

        // Check for comment
        if line.starts_with(style.prefix) {
            // Skip directive lines (not justification comments)
            if style.directive_patterns.iter().any(|p| line.contains(p)) {
                continue;
            }

            let comment_text = line.trim_start_matches(style.prefix).trim();

            // If specific pattern required, check for it
            if let Some(pattern) = required_pattern {
                let pattern_prefix = pattern.trim_start_matches(style.prefix).trim();
                if comment_text.starts_with(pattern_prefix) || line.starts_with(pattern) {
                    return (true, Some(comment_text.to_string()));
                }
                // Continue looking for the pattern
                continue;
            }

            // Any non-empty comment counts as justification
            if !comment_text.is_empty() {
                return (true, Some(comment_text.to_string()));
            }
        } else if !line.starts_with('#') {
            // For Rust: stop at non-attribute, non-comment line
            // For Shell: this branch won't be reached since # is the prefix
            break;
        }
    }

    (false, None)
}
