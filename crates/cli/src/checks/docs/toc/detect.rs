// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tree detection heuristics.
//!
//! Determines whether a fenced code block is a directory tree.

use super::parse::{FencedBlock, parse_tree_block};

/// Language tag that forces TOC validation.
pub(crate) const TOC_LANGUAGE: &str = "toc";

/// Language tags that indicate the block is NOT a directory tree.
#[rustfmt::skip]
const NON_TREE_LANGUAGES: &[&str] = &[
    // Explicit skip annotations
    "no-toc", "ignore",
    // Code languages
    "rust", "rs", "go", "python", "py", "javascript", "js", "typescript", "ts",
    "java", "c", "cpp", "csharp", "cs", "ruby", "rb", "php", "swift", "kotlin",
    "scala", "perl", "lua", "r", "julia", "haskell", "hs", "ocaml", "ml",
    "elixir", "ex", "erlang", "clojure", "clj", "lisp", "scheme", "racket",
    "zig", "nim", "d", "v", "odin", "jai", "carbon",
    // Shell and scripting
    "bash", "sh", "zsh", "fish", "powershell", "pwsh", "bat", "cmd",
    // Config and data
    "toml", "yaml", "yml", "json", "xml", "ini", "cfg",
    // Output and plain text
    "text", "txt", "output", "console", "terminal", "log",
    // Markup and other
    "html", "css", "scss", "sass", "less", "sql", "graphql", "gql",
    "dockerfile", "makefile", "cmake",
];

/// Check if a fenced block looks like a directory tree.
pub(crate) fn looks_like_tree(block: &FencedBlock) -> bool {
    // Explicit toc tag forces validation
    if block.language.as_deref() == Some(TOC_LANGUAGE) {
        return true;
    }
    // Blocks with known non-tree language tags are skipped
    if let Some(ref lang) = block.language
        && NON_TREE_LANGUAGES.contains(&lang.as_str())
    {
        return false;
    }

    // Must have at least one line
    if block.lines.is_empty() {
        return false;
    }

    // Box diagram detection: if any line contains a top corner, it's a box diagram, not a tree
    // Top corners: ┌ (U+250C), ╔ (U+2554), ╭ (U+256D)
    if block
        .lines
        .iter()
        .any(|line| line.contains('┌') || line.contains('╔') || line.contains('╭'))
    {
        return false;
    }

    // Count different types of tree signals
    let box_drawing_lines = block
        .lines
        .iter()
        .filter(|line| {
            let t = line.trim();
            t.contains('├') || t.contains('└') || t.contains('│')
        })
        .count();

    let directory_lines = block
        .lines
        .iter()
        .filter(|line| {
            let t = line.trim();
            t.ends_with('/') && !t.contains(' ') && !t.contains('=')
        })
        .count();

    let file_like_lines = block.lines.iter().filter(|line| is_tree_line(line)).count();

    // Strong signal: any box-drawing characters
    if box_drawing_lines >= 1 {
        return true;
    }

    // Strong signal: directory lines (ending with /)
    if directory_lines >= 1 && file_like_lines >= 2 {
        return true;
    }

    // Weak signal: multiple file-like lines
    // Require MORE evidence (3+ lines instead of 2)
    // AND no indication this is error output
    if file_like_lines >= 3 {
        // Check that NO lines look like error output
        let has_error_output = block
            .lines
            .iter()
            .any(|line| looks_like_error_output(line.trim()));
        if !has_error_output {
            return true;
        }
    }

    false
}

/// Check if a line looks like compiler/linter error output.
///
/// Matches patterns like:
/// - `file.ext:123:` (file:line:)
/// - `file.ext:123:45:` (file:line:col:)
/// - `file.ext:123: message` (file:line: message)
#[cfg_attr(test, allow(dead_code))]
pub(super) fn looks_like_error_output(line: &str) -> bool {
    // Look for pattern: something.ext:digits:
    // Must have: extension with dot, colon, digits, colon
    let Some(colon_pos) = line.find(':') else {
        return false;
    };

    let before_colon = &line[..colon_pos];

    // Must look like a file path (contains dot for extension)
    if !before_colon.contains('.') {
        return false;
    }

    // Check if what follows the colon starts with digits
    let after_colon = &line[colon_pos + 1..];
    let first_after = after_colon.chars().next();

    match first_after {
        Some(c) if c.is_ascii_digit() => {
            // Looks like file.ext:123...
            // Check if followed by another colon (file:line: or file:line:col:)
            if let Some(next_colon) = after_colon.find(':') {
                let between = &after_colon[..next_colon];
                // All digits between first and second colon
                if between.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
        }
        _ => {}
    }

    false
}

/// Check if a line looks like a directory tree entry.
fn is_tree_line(line: &str) -> bool {
    let trimmed = line.trim();

    // Empty lines don't count
    if trimmed.is_empty() {
        return false;
    }

    // Box-drawing characters are strong tree indicators
    if trimmed.contains('├') || trimmed.contains('└') || trimmed.contains('│') {
        return true;
    }

    // Reject TOML/config patterns
    // [section] headers
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return false;
    }
    // key = value assignments
    if trimmed.contains(" = ") {
        return false;
    }
    // TOML table arrays [[...]]
    if trimmed.starts_with("[[") {
        return false;
    }

    // Reject error output patterns
    // Pattern: file.ext:line: or file.ext:line:col:
    // Examples: "foo.rs:23:", "src/main.go:45:12:", "script.sh:10: error"
    if looks_like_error_output(trimmed) {
        return false;
    }

    // Directory paths ending with /
    if trimmed.ends_with('/') && !trimmed.contains(' ') && !trimmed.contains('=') {
        return true;
    }

    // Check for file-like patterns that aren't code or config
    // A file path typically looks like: foo/bar.rs, lib.rs, etc.
    if trimmed.contains('.') && !trimmed.starts_with('.') {
        // Reject if it looks like code or config
        let code_patterns = ['(', ')', '=', ';', '{', '}', '"', '\'', '[', ']'];
        let code_keywords = [
            "let ", "fn ", "use ", "pub ", "mod ", "const ", "static ", "name ", "path ",
        ];

        let has_code_pattern = code_patterns.iter().any(|&c| trimmed.contains(c));
        let has_code_keyword = code_keywords.iter().any(|kw| trimmed.contains(kw));

        if !has_code_pattern && !has_code_keyword && !trimmed.contains("//") {
            // Looks like a file path: no spaces except possibly in comments
            let before_comment = trimmed.split('#').next().unwrap_or(trimmed).trim();
            if !before_comment.contains(' ')
                || before_comment.starts_with("├")
                || before_comment.starts_with("└")
            {
                return true;
            }
        }
    }

    false
}

/// Check if a block matches a valid tree format (box-drawing or indentation).
pub(crate) fn is_valid_tree_format(block: &FencedBlock) -> bool {
    if block.lines.is_empty() {
        return false;
    }
    let has_format = block.lines.iter().any(|line| {
        let t = line.trim();
        line.contains('├')
            || line.contains('└')
            || line.contains('│')
            || line.starts_with(' ')
            || line.starts_with('\t')
            || (t.ends_with('/') && !t.contains(' '))
    });
    has_format && !parse_tree_block(block).is_empty()
}
