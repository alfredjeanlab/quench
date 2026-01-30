// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Color detection and terminal styling.
//!
//! Detection logic per docs/specs/03-output.md#colorization:
//! 1. NO_COLOR env var → no color
//! 2. COLOR env var → use color
//! 3. default:
//!    - If not stdout.is_tty() → no color
//!    - If CLAUDE_CODE, CODEX, CI, or CURSOR env var set → no color
//!    - Else → use color
//!
//! This module provides two color systems:
//! - ANSI 256-color for help text (via `codes` module and helper functions)
//! - termcolor ColorSpec for check output (via `scheme` module)

use std::io::IsTerminal;
use std::sync::OnceLock;
use termcolor::ColorChoice;

/// ANSI 256-color codes for terminal styling.
pub mod codes {
    /// Section headers: pastel cyan/steel blue
    pub const HEADER: u8 = 74;
    /// Commands/literals: light grey
    pub const LITERAL: u8 = 250;
    /// Default values/context: medium grey
    pub const CONTEXT: u8 = 245;
}

// =============================================================================
// Help text colorization
// =============================================================================

/// Check if colors should be enabled based on TTY and environment variables.
///
/// This is cached once per process for consistent behavior.
pub fn should_colorize() -> bool {
    static SHOULD_COLORIZE: OnceLock<bool> = OnceLock::new();
    *SHOULD_COLORIZE.get_or_init(|| {
        // NO_COLOR=1 disables colors
        if std::env::var("NO_COLOR").is_ok_and(|v| v == "1") {
            return false;
        }

        // COLOR=1 forces colors even without TTY
        if std::env::var("COLOR").is_ok_and(|v| v == "1") {
            return true;
        }

        // Default: enable colors only if stdout is a TTY
        std::io::stdout().is_terminal()
    })
}

/// Format a 256-color ANSI escape sequence for foreground color.
pub fn fg256(code: u8) -> String {
    format!("\x1b[38;5;{code}m")
}

/// ANSI reset sequence.
pub const RESET: &str = "\x1b[0m";

/// Apply header color (section titles) to text.
pub fn header(text: &str) -> String {
    if should_colorize() {
        format!("{}{}{}", fg256(codes::HEADER), text, RESET)
    } else {
        text.to_string()
    }
}

/// Apply literal color (commands, options) to text.
pub fn literal(text: &str) -> String {
    if should_colorize() {
        format!("{}{}{}", fg256(codes::LITERAL), text, RESET)
    } else {
        text.to_string()
    }
}

/// Apply context color (default values, hints) to text.
pub fn context(text: &str) -> String {
    if should_colorize() {
        format!("{}{}{}", fg256(codes::CONTEXT), text, RESET)
    } else {
        text.to_string()
    }
}

/// Colorize an examples help block.
///
/// Expects format like:
/// ```text
/// Examples:
///   quench check .          Run checks on current directory
///   quench check --fix      Fix violations automatically
///
/// Output Formats:
///   Syntax: -o FORMAT
///   Formats: text, json, html
/// ```
///
/// Colorizes:
/// - Section headers (lines ending with `:`) as header color
/// - Commands (before `  `) as literal color
/// - Documentation labels (e.g., "Syntax:") as literal, values as context
pub fn examples(text: &str) -> String {
    if !should_colorize() {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 256);

    for line in text.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];

        // Header line (e.g., "Examples:", "Output Formats:")
        if trimmed.ends_with(':') && !trimmed.contains("  ") {
            result.push_str(indent);
            result.push_str(&header(trimmed));
            continue;
        }

        // Example line: "  quench check .    Description"
        // Find where command ends (multiple spaces before description)
        if let Some(cmd_end) = find_description_start(trimmed) {
            let cmd = &trimmed[..cmd_end];
            let desc = &trimmed[cmd_end..];
            result.push_str(indent);
            result.push_str(&colorize_command(cmd));
            result.push_str(desc);
            continue;
        }

        // Documentation line: "Label: value" (e.g., "Syntax: FIELD [OPERATOR VALUE]")
        // Label stays uncolored, value colored as literal
        if let Some(colon_pos) = trimmed.find(": ") {
            let label = &trimmed[..=colon_pos]; // Include the colon
            let value = &trimmed[colon_pos + 2..]; // Skip ": " to get value
            result.push_str(indent);
            result.push_str(label);
            result.push(' ');
            result.push_str(&literal(value));
            continue;
        }

        // No pattern matched, output as-is
        result.push_str(line);
    }

    result
}

/// Colorize a markdown guide block (used by `quench config <feature>`).
///
/// Applies ANSI 256-color to markdown elements:
/// - `# Heading` / `## Heading` → header color
/// - Fence markers (`` ``` ``) → context color
/// - TOML code lines → literal color
/// - TOML `# comments` inside code blocks → context color
/// - Plain prose outside code blocks → uncolored
pub fn guide(text: &str) -> String {
    format_guide(text, should_colorize())
}

/// Inner implementation of [`guide`] that accepts an explicit colorize flag
/// so tests can exercise both paths without depending on TTY state.
fn format_guide(text: &str, colorize: bool) -> String {
    if !colorize {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 256);
    let mut in_code_block = false;

    for line in text.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        let trimmed = line.trim_start();

        // Fence lines toggle code block state
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(&format!("{}{}{}", fg256(codes::CONTEXT), line, RESET));
            continue;
        }

        if in_code_block {
            if trimmed.starts_with('[') {
                // TOML table headers: [section] or [[array]] — context color (dark)
                result.push_str(&format!("{}{}{}", fg256(codes::CONTEXT), line, RESET));
            } else if trimmed.starts_with('#') {
                // TOML full-line comments
                result.push_str(&format!("{}{}{}", fg256(codes::CONTEXT), line, RESET));
            } else if let Some(pos) = find_toml_comment(line) {
                // Code with trailing comment: split into literal + context
                let code_part = &line[..pos];
                let comment_part = &line[pos..];
                result.push_str(&format!(
                    "{}{}{}{}{}{}",
                    fg256(codes::LITERAL),
                    code_part,
                    RESET,
                    fg256(codes::CONTEXT),
                    comment_part,
                    RESET
                ));
            } else {
                result.push_str(&format!("{}{}{}", fg256(codes::LITERAL), line, RESET));
            }
            continue;
        }

        // Headings outside code blocks
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") {
            result.push_str(&format!("{}{}{}", fg256(codes::HEADER), line, RESET));
            continue;
        }

        // Plain prose — pass through uncolored
        result.push_str(line);
    }

    // Preserve trailing newline if present
    if text.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Colorize a command string, highlighting quoted content, placeholders, and flag values as context.
fn colorize_command(cmd: &str) -> String {
    let mut result = String::with_capacity(cmd.len() + 128);
    let mut chars = cmd.char_indices().peekable();
    let mut current_word_start = 0;
    let mut in_flag_value = false;

    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                // Flush any pending literal content before the quote
                if i > current_word_start {
                    let before = &cmd[current_word_start..i];
                    if in_flag_value {
                        result.push_str(&context(before));
                        in_flag_value = false;
                    } else {
                        result.push_str(&literal(before));
                    }
                }

                // Find closing quote
                let quote_start = i;
                let mut quote_end = cmd.len();
                for (j, ch) in chars.by_ref() {
                    if ch == '"' {
                        quote_end = j + 1;
                        break;
                    }
                }
                result.push_str(&context(&cmd[quote_start..quote_end]));
                current_word_start = quote_end;
            }
            '<' => {
                // Flush any pending literal content before the angle bracket
                if i > current_word_start {
                    let before = &cmd[current_word_start..i];
                    if in_flag_value {
                        result.push_str(&context(before));
                        in_flag_value = false;
                    } else {
                        result.push_str(&literal(before));
                    }
                }

                // Find closing angle bracket for placeholder like <id>
                let bracket_start = i;
                let mut bracket_end = cmd.len();
                for (j, ch) in chars.by_ref() {
                    if ch == '>' {
                        bracket_end = j + 1;
                        break;
                    }
                }
                result.push_str(&context(&cmd[bracket_start..bracket_end]));
                current_word_start = bracket_end;
            }
            ' ' => {
                // Flush current segment
                if i > current_word_start {
                    let segment = &cmd[current_word_start..i];
                    if in_flag_value {
                        result.push_str(&context(segment));
                        in_flag_value = false;
                    } else {
                        result.push_str(&literal(segment));
                        // Check if this segment is a flag (starts with -)
                        if segment.starts_with('-') && !segment.contains('=') {
                            in_flag_value = true;
                        }
                    }
                }
                result.push(' ');
                current_word_start = i + 1;
            }
            _ => {}
        }
    }

    // Flush remaining content
    if current_word_start < cmd.len() {
        let remaining = &cmd[current_word_start..];
        if in_flag_value {
            result.push_str(&context(remaining));
        } else {
            result.push_str(&literal(remaining));
        }
    }

    result
}

/// Find where the description starts (after 2+ spaces following the command).
fn find_description_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_spaces = false;
    let mut space_start = 0;

    while i < bytes.len() {
        if bytes[i] == b' ' {
            if !in_spaces {
                in_spaces = true;
                space_start = i;
            }
        } else {
            if in_spaces && i - space_start >= 2 {
                // Found 2+ spaces, command ends at space_start
                return Some(space_start);
            }
            in_spaces = false;
        }
        i += 1;
    }

    None
}

/// Find the start of a trailing TOML comment (`# ...`) in a code line,
/// skipping `#` characters inside quoted strings.
///
/// Returns the byte offset of the space before `#`, or `None` if no
/// trailing comment is found.
fn find_toml_comment(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut in_quote = false;
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_quote || (i > 0 && bytes[i - 1] != b'\\') => {
                in_quote = !in_quote;
            }
            b'#' if !in_quote && i > 0 && bytes[i - 1] == b' ' => {
                // Return position including leading whitespace run
                let mut start = i - 1;
                while start > 0 && bytes[start - 1] == b' ' {
                    start -= 1;
                }
                return Some(start);
            }
            _ => {}
        }
        i += 1;
    }

    None
}

// =============================================================================
// Check output colorization (termcolor)
// =============================================================================

/// Resolve color choice from environment variables.
///
/// Priority: NO_COLOR > COLOR > auto-detect
///
/// Per [no-color.org](https://no-color.org/), `NO_COLOR` when set to any value
/// (including empty string) disables color. The `COLOR` env var follows a
/// similar convention for forcing color output.
pub fn resolve_color() -> ColorChoice {
    // NO_COLOR spec: any value (including empty) disables color
    if std::env::var_os("NO_COLOR").is_some() {
        return ColorChoice::Never;
    }
    // COLOR=1 forces color (non-standard but common)
    if std::env::var_os("COLOR").is_some() {
        return ColorChoice::Always;
    }
    // Auto-detect
    if !std::io::stdout().is_terminal() {
        return ColorChoice::Never;
    }
    if is_agent_environment() {
        return ColorChoice::Never;
    }
    ColorChoice::Auto
}

/// Check if running in an AI agent environment.
fn is_agent_environment() -> bool {
    std::env::var_os("CLAUDE_CODE").is_some()
        || std::env::var_os("CODEX").is_some()
        || std::env::var_os("CURSOR").is_some()
        || std::env::var_os("CI").is_some()
}

/// Color scheme for output per spec.
pub mod scheme {
    use termcolor::{Color, ColorSpec};

    /// Bold check name (e.g., "cloc").
    pub fn check_name() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        spec
    }

    /// Red "FAIL" indicator.
    pub fn fail() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red)).set_bold(true);
        spec
    }

    /// Green "PASS" indicator.
    pub fn pass() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green)).set_bold(true);
        spec
    }

    /// Green "FIXED" indicator.
    pub fn fixed() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green)).set_bold(true);
        spec
    }

    /// Yellow "SKIP" indicator.
    pub fn skip() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow)).set_bold(true);
        spec
    }

    /// Yellow "WARN" indicator.
    pub fn warn() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow)).set_bold(true);
        spec
    }

    /// Cyan file path.
    pub fn path() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Cyan));
        spec
    }

    /// Yellow line number.
    pub fn line_number() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Yellow));
        spec
    }

    /// Default (no color) for advice text.
    pub fn advice() -> ColorSpec {
        ColorSpec::new()
    }

    /// Red for diff removed lines.
    pub fn diff_remove() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Red));
        spec
    }

    /// Green for diff added lines.
    pub fn diff_add() -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(Color::Green));
        spec
    }
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
