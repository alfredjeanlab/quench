// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Line counting module shared by `quench cloc` and `quench check --cloc`.
//!
//! Provides file-level metrics (blank, comment, code, tokens) and
//! per-language comment detection.

pub mod comment;

/// Metrics for a single file.
pub struct FileMetrics {
    /// Total line count (matches `wc -l`).
    pub lines: usize,
    /// Blank lines (whitespace only).
    pub blank: usize,
    /// Comment lines.
    pub comment: usize,
    /// Code lines (= lines - blank - comment).
    pub code: usize,
    /// Non-blank lines (= comment + code).
    pub nonblank: usize,
    /// Token estimate (chars / 4).
    pub tokens: usize,
}

/// Count metrics from file content using the file extension for comment detection.
///
/// If no comment style is known for the extension, all non-blank lines are
/// counted as code (matching `cloc` behavior for unknown languages).
pub fn count_file_metrics(content: &str, ext: &str) -> FileMetrics {
    let lines = content.lines().count();
    let tokens = content.chars().count() / 4;

    let (blank, comment_count, code) = match comment::comment_style(ext) {
        Some(style) => {
            let lc = comment::count_lines(content, &style);
            (lc.blank, lc.comment, lc.code)
        }
        None => {
            // Unknown language: blank vs code only
            let blank = content.lines().filter(|l| l.trim().is_empty()).count();
            let code = lines - blank;
            (blank, 0, code)
        }
    };

    FileMetrics {
        lines,
        blank,
        comment: comment_count,
        code,
        nonblank: comment_count + code,
        tokens,
    }
}

/// Map file extension to human-readable language name.
pub fn language_name(ext: &str) -> &str {
    match ext {
        "rs" => "Rust",
        "go" => "Go",
        "py" => "Python",
        "rb" => "Ruby",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "ts" | "tsx" | "mts" | "cts" => "TypeScript",
        "java" => "Java",
        "kt" => "Kotlin",
        "scala" => "Scala",
        "swift" => "Swift",
        "c" => "C",
        "cpp" | "hpp" => "C++",
        "h" => "C/C++ Header",
        "cs" => "C#",
        "m" => "Objective-C",
        "mm" => "Objective-C++",
        "sh" | "bash" | "zsh" | "fish" | "bats" => "Shell",
        "ps1" => "PowerShell",
        "bat" | "cmd" => "Batch",
        "php" => "PHP",
        "lua" => "Lua",
        "sql" => "SQL",
        "r" => "R",
        "pl" | "pm" => "Perl",
        "vue" => "Vue",
        "svelte" => "Svelte",
        other => other,
    }
}

/// Check if a file extension is a known text/source file for LOC counting.
pub fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        // Systems languages
        "rs" | "c" | "cpp" | "h" | "hpp" | "go"
        // JVM languages
        | "java" | "kt" | "scala"
        // Dynamic languages
        | "py" | "rb" | "php" | "lua" | "pl" | "pm" | "r"
        // JavaScript/TypeScript
        | "js" | "ts" | "jsx" | "tsx"
        // Module variants
        | "mjs" | "mts" | "cjs" | "cts"
        // Apple platforms
        | "swift" | "m" | "mm"
        // .NET
        | "cs"
        // Shell scripts
        | "sh" | "bash" | "zsh" | "fish" | "bats" | "ps1" | "bat" | "cmd"
        // Web (code only)
        | "vue" | "svelte"
        // SQL
        | "sql"
    )
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
