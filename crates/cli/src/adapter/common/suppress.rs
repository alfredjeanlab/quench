// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Common suppress directive utilities.

/// Comment style configuration for different languages.
pub struct CommentStyle {
    /// Comment line prefix (e.g., "//" for Rust, "#" for Shell).
    pub prefix: &'static str,
    /// Patterns that indicate a directive line, not a justification comment.
    /// Checked with `contains`.
    pub directive_patterns: &'static [&'static str],
    /// Line prefixes to skip when walking backward (e.g., "@" for Python decorators).
    /// Checked with `starts_with` on the trimmed line.
    pub skip_prefixes: &'static [&'static str],
}

impl CommentStyle {
    /// Rust comment style: `//` prefix, `#[` directives.
    pub const RUST: Self = Self {
        prefix: "//",
        directive_patterns: &["#["],
        skip_prefixes: &[],
    };

    /// Go comment style: `//` prefix, `//go:` directives.
    pub const GO: Self = Self {
        prefix: "//",
        directive_patterns: &["//go:", "//nolint"],
        skip_prefixes: &[],
    };

    /// Shell comment style: `#` prefix, `shellcheck` directives.
    pub const SHELL: Self = Self {
        prefix: "#",
        directive_patterns: &["shellcheck", "!"],
        skip_prefixes: &[],
    };

    /// Ruby comment style: `#` prefix, rubocop/standard directives.
    pub const RUBY: Self = Self {
        prefix: "#",
        directive_patterns: &["rubocop:", "standard:", "!"],
        skip_prefixes: &[],
    };

    /// Python comment style: `#` prefix, noqa/type/pylint/pragma directives.
    /// Skips `@decorator` lines when walking backward for justification comments.
    pub const PYTHON: Self = Self {
        prefix: "#",
        directive_patterns: &["noqa", "type:", "pylint:", "pragma:", "!"],
        skip_prefixes: &["@"],
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

        // Skip directive lines (not justification comments)
        // This includes: Rust attributes (#[...]), Shell shebangs (#!/...), etc.
        if style.directive_patterns.iter().any(|p| line.contains(p)) {
            continue;
        }

        // Skip lines matching prefix patterns (e.g., Python @decorators)
        if style.skip_prefixes.iter().any(|p| line.starts_with(p)) {
            continue;
        }

        // Check for comment
        if line.starts_with(style.prefix) {
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
        } else {
            // Stop at non-comment, non-directive line (i.e., code)
            break;
        }
    }

    (false, None)
}
