// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Comment detection per language.
//!
//! Provides best-effort, line-by-line comment classification using simple
//! heuristics rather than full parsing. Consistent with how `cloc` works.

/// Comment syntax for a language.
pub struct CommentStyle {
    /// Single-line comment prefixes (e.g. `["//", "#"]`).
    pub line: &'static [&'static str],
    /// Block comment delimiter pairs `(open, close)`.
    pub block: &'static [(&'static str, &'static str)],
}

/// Line count breakdown.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LineCounts {
    pub blank: usize,
    pub comment: usize,
    pub code: usize,
}

/// Return the comment style for a file extension, or `None` if unknown.
pub fn comment_style(ext: &str) -> Option<CommentStyle> {
    match ext {
        "rs" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "go" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "py" | "rb" | "sh" | "bash" | "zsh" | "fish" | "bats" | "r" => Some(CommentStyle {
            line: &["#"],
            block: &[],
        }),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts" | "java" | "kt" | "scala"
        | "swift" | "c" | "cpp" | "h" | "hpp" | "cs" | "m" | "mm" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/")],
        }),
        "lua" => Some(CommentStyle {
            line: &["--"],
            block: &[("--[[", "]]")],
        }),
        "sql" => Some(CommentStyle {
            line: &["--"],
            block: &[("/*", "*/")],
        }),
        "php" => Some(CommentStyle {
            line: &["//", "#"],
            block: &[("/*", "*/")],
        }),
        "vue" | "svelte" => Some(CommentStyle {
            line: &["//"],
            block: &[("/*", "*/"), ("<!--", "-->")],
        }),
        "pl" | "pm" => Some(CommentStyle {
            line: &["#"],
            block: &[("=pod", "=cut")],
        }),
        "ps1" => Some(CommentStyle {
            line: &["#"],
            block: &[("<#", "#>")],
        }),
        "bat" | "cmd" => Some(CommentStyle {
            line: &["REM ", ":: "],
            block: &[],
        }),
        _ => None,
    }
}

/// Count blank, comment, and code lines in `content` using the given comment style.
///
/// Uses a simple state machine: tracks whether we're inside a block comment.
/// For each line, classifies as blank -> comment -> code (first match wins).
///
/// A line inside a block comment that contains code after the closing delimiter
/// is counted as code, consistent with `cloc` behavior.
pub fn count_lines(content: &str, style: &CommentStyle) -> LineCounts {
    let mut counts = LineCounts::default();
    let mut in_block: Option<&'static str> = None; // The close delimiter we're looking for

    for line in content.lines() {
        let trimmed = line.trim();

        // Blank line check (always takes priority)
        if trimmed.is_empty() {
            counts.blank += 1;
            continue;
        }

        // If inside a block comment, look for the close delimiter
        if let Some(close) = in_block {
            if let Some(pos) = trimmed.find(close) {
                // Found the close delimiter
                let after_close = trimmed[pos + close.len()..].trim();
                in_block = None;
                if after_close.is_empty() {
                    // Nothing after the close delimiter -> comment
                    counts.comment += 1;
                } else {
                    // Code follows the close delimiter -> code
                    counts.code += 1;
                }
            } else {
                // Still inside block comment
                counts.comment += 1;
            }
            continue;
        }

        // Check for block comment open
        let mut is_block_open = false;
        for &(open, close) in style.block {
            if let Some(after_open) = trimmed.strip_prefix(open) {
                // Check if the block closes on the same line
                if let Some(pos) = after_open.find(close) {
                    // Single-line block comment
                    let after_close = after_open[pos + close.len()..].trim();
                    if after_close.is_empty() {
                        counts.comment += 1;
                    } else {
                        // Code after single-line block comment -> code
                        counts.code += 1;
                    }
                } else {
                    // Multi-line block comment starts
                    in_block = Some(close);
                    counts.comment += 1;
                }
                is_block_open = true;
                break;
            }
        }
        if is_block_open {
            continue;
        }

        // Check for single-line comment
        let mut is_line_comment = false;
        for prefix in style.line {
            if trimmed.starts_with(prefix) {
                counts.comment += 1;
                is_line_comment = true;
                break;
            }
        }
        if is_line_comment {
            continue;
        }

        // Not blank, not a comment -> code
        counts.code += 1;
    }

    counts
}

#[cfg(test)]
#[path = "comment_tests.rs"]
mod tests;
