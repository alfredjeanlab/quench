// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Custom help formatting with consolidated --[no-] flags and colorization.
//!
//! Clap displays negatable flags as separate lines:
//!   --limit      Set limit
//!   --no-limit   Disable limit
//!
//! This module consolidates them into a single line:
//!   --[no-]limit   Enable/disable limit
//!
//! Since we capture help to a string for consolidation (regex parsing),
//! we apply colors ourselves rather than using clap's built-in colorization.

use crate::color;
use clap::Command;
use clap::builder::styling::Styles;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Generate clap Styles for help output.
///
/// Returns plain styles since we apply colors ourselves after flag
/// consolidation. Clap's auto-colorization doesn't work when capturing
/// help to a string (for regex-based consolidation).
pub fn styles() -> Styles {
    // We apply colors ourselves in format_help() after consolidation
    Styles::plain()
}

/// Regex to match option lines in help output.
/// Captures: (leading_space, short_opt?, long_opt, value?, description)
#[allow(clippy::expect_used)]
static OPTION_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s+)(?:(-\w),\s+)?(--[\w-]+)(?:\s+<([^>]+)>)?(?:\s+(.*))?$")
        .expect("valid regex")
});

/// Regex to detect --no-X flags.
#[allow(clippy::expect_used)]
static NO_FLAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^--no-(.+)$").expect("valid regex"));

/// Formats help text with consolidated --[no-] flags and colorization.
pub fn format_help(cmd: &mut Command) -> String {
    let mut help = Vec::new();
    // write_help only fails on IO errors, which can't happen with Vec<u8>
    let _ = cmd.write_help(&mut help);
    // Help output from clap is always valid UTF-8
    let raw_help = String::from_utf8_lossy(&help);
    let consolidated = consolidate_negatable_flags(&raw_help);
    colorize_help(&consolidated)
}

/// Apply colors to help text.
///
/// Colorizes:
/// - Section headers (Usage:, Commands:, Options:, Arguments:)
/// - Command/option names (literals)
/// - Placeholders like <PATH>
fn colorize_help(help: &str) -> String {
    if !color::should_colorize() {
        return help.to_string();
    }

    let mut result = String::with_capacity(help.len() + 512);

    for line in help.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        // Section headers: "Commands:", "Options:", "Arguments:"
        if is_section_header(line) {
            result.push_str(&color::header(line));
            continue;
        }

        // Usage line: "Usage: quench [OPTIONS] [COMMAND]"
        if let Some(colored) = colorize_usage_line(line) {
            result.push_str(&colored);
            continue;
        }

        // Command/subcommand lines: "  check   Run quality checks"
        if let Some(colored) = colorize_command_line(line) {
            result.push_str(&colored);
            continue;
        }

        // Option lines: "  -o, --output <FORMAT>  Description"
        if let Some(colored) = colorize_option_line(line) {
            result.push_str(&colored);
            continue;
        }

        // Default: pass through unchanged
        result.push_str(line);
    }

    result
}

/// Check if a line is a section header (standalone, not followed by content).
fn is_section_header(line: &str) -> bool {
    let trimmed = line.trim();
    // "Usage:" is handled separately by colorize_usage_line since it has content after
    matches!(trimmed, "Commands:" | "Options:" | "Arguments:")
}

/// Colorize a usage line like "Usage: quench [OPTIONS] [COMMAND]".
fn colorize_usage_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("Usage:") {
        return None;
    }

    let indent = &line[..line.len() - trimmed.len()];
    let after_usage = &trimmed["Usage:".len()..];

    let mut result = String::new();
    result.push_str(indent);
    result.push_str(&color::header("Usage:"));

    // Colorize the usage pattern: command name as literal, placeholders as context
    for word in after_usage.split_whitespace() {
        result.push(' ');
        if word.starts_with('[') || word.starts_with('<') {
            // Placeholder like [OPTIONS] or <PATH>
            result.push_str(&color::context(word));
        } else {
            // Command name
            result.push_str(&color::literal(word));
        }
    }

    Some(result)
}

/// Colorize a command/subcommand line like "  check   Run quality checks".
fn colorize_command_line(line: &str) -> Option<String> {
    // Must start with exactly 2 spaces, then a word, then 2+ spaces, then description
    if !line.starts_with("  ") || line.starts_with("   ") {
        return None;
    }

    let content = &line[2..];
    // Skip if it looks like an option (starts with -)
    if content.starts_with('-') {
        return None;
    }

    // Find the command name (first word) and description separator (2+ spaces)
    let mut parts = content.splitn(2, "  ");
    let cmd_name = parts.next()?;

    // Command name should be a simple word (no spaces, no special chars except -)
    if cmd_name.is_empty() || cmd_name.contains(' ') {
        return None;
    }

    // Get everything after the command name (preserves spacing)
    let rest = &content[cmd_name.len()..];

    Some(format!("  {}{}", color::literal(cmd_name), rest))
}

/// Colorize an option line like "  -o, --output <FORMAT>  Description".
fn colorize_option_line(line: &str) -> Option<String> {
    static OPT_RE: LazyLock<Regex> = LazyLock::new(|| {
        // Match: leading spaces, optional short (-x, ), long option (--name or --[no-]name),
        // optional value (<VAL> or [VAL]), then spaces and description
        #[allow(clippy::expect_used)]
        Regex::new(
            r"^(\s+)((?:-\w,\s+)?)(--\[?[\w-]+\]?[\w-]*)(\s+(?:<[^>]+>|\[[^\]]+\]))?(\s{2,}.+)?$",
        )
        .expect("valid regex")
    });

    let caps = OPT_RE.captures(line)?;

    let indent = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let short = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let long = caps.get(3).map(|m| m.as_str()).unwrap_or("");
    let value = caps.get(4).map(|m| m.as_str()).unwrap_or("");
    let desc = caps.get(5).map(|m| m.as_str()).unwrap_or("");

    // Colorize short option if present
    let colored_short = if short.is_empty() {
        String::new()
    } else {
        // short is like "-o, " - colorize just the -o part
        let opt_part = short.trim_end_matches(", ").trim_end();
        format!("{}, ", color::literal(opt_part))
    };

    // Colorize long option, handling --[no-]name specially
    let colored_long = if long.contains("[no-]") {
        // Split into prefix and name: --[no-]limit -> [no-] + limit
        let without_dashes = long.strip_prefix("--").unwrap_or(long);
        let name = without_dashes
            .strip_prefix("[no-]")
            .unwrap_or(without_dashes);
        format!("--{}{}", color::context("[no-]"), color::literal(name))
    } else {
        color::literal(long)
    };

    // Colorize value placeholder
    let colored_value = if value.is_empty() {
        String::new()
    } else {
        format!(" {}", color::context(value.trim()))
    };

    Some(format!(
        "{}{}{}{}{}",
        indent, colored_short, colored_long, colored_value, desc
    ))
}

/// Parsed option line information.
#[derive(Debug, Clone)]
struct OptionInfo {
    /// Line index in original help
    line_idx: usize,
    /// Leading whitespace
    leading: String,
    /// Short option (e.g., "-o")
    short: Option<String>,
    /// Long option without -- prefix
    long_name: String,
    /// Value placeholder (e.g., "N" for --limit <N>)
    value: Option<String>,
    /// Description text
    description: String,
}

/// Identifies --flag/--no-flag pairs and merges them.
fn consolidate_negatable_flags(help: &str) -> String {
    let lines: Vec<&str> = help.lines().collect();
    let mut options: HashMap<String, OptionInfo> = HashMap::new();
    let mut no_flags: Vec<OptionInfo> = Vec::new();
    let mut lines_to_remove: Vec<usize> = Vec::new();
    let mut replacements: HashMap<usize, String> = HashMap::new();

    // Parse all option lines
    for (idx, line) in lines.iter().enumerate() {
        if let Some(caps) = OPTION_LINE_RE.captures(line) {
            let leading = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let short = caps.get(2).map(|m| m.as_str().to_string());
            let long_with_dashes = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let value = caps.get(4).map(|m| m.as_str().to_string());
            let description = caps.get(5).map(|m| m.as_str()).unwrap_or("").to_string();

            let info = OptionInfo {
                line_idx: idx,
                leading,
                short,
                long_name: long_with_dashes
                    .strip_prefix("--")
                    .unwrap_or(long_with_dashes)
                    .to_string(),
                value,
                description,
            };

            if NO_FLAG_RE.is_match(long_with_dashes) {
                // This is a --no-X flag
                no_flags.push(info);
            } else {
                // This is a regular flag
                options.insert(info.long_name.clone(), info);
            }
        }
    }

    // Process --no-X flags
    for no_info in no_flags {
        let positive_name = no_info
            .long_name
            .strip_prefix("no-")
            .unwrap_or(&no_info.long_name);

        if let Some(positive_info) = options.get(positive_name) {
            // Found a matching positive flag - consolidate!
            let consolidated = build_consolidated_line(positive_info, &no_info);
            replacements.insert(positive_info.line_idx, consolidated);
            lines_to_remove.push(no_info.line_idx);
        }
        // If no positive counterpart, leave --no-X as-is
    }

    // Build output
    let mut result = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if lines_to_remove.contains(&idx) {
            continue;
        }
        if let Some(replacement) = replacements.get(&idx) {
            result.push(replacement.as_str());
        } else {
            result.push(*line);
        }
    }

    result.join("\n")
}

/// Build a consolidated line from positive and negative flag info.
fn build_consolidated_line(positive: &OptionInfo, _negative: &OptionInfo) -> String {
    let mut parts = Vec::new();

    // Leading whitespace
    parts.push(positive.leading.as_str());

    // Short option if present
    if let Some(ref short) = positive.short {
        parts.push(short);
        parts.push(", ");
    }

    // Consolidated long option
    let consolidated_long = format!("--[no-]{}", positive.long_name);
    parts.push(&consolidated_long);

    // Value placeholder (make optional if present)
    let value_str;
    if let Some(ref val) = positive.value {
        value_str = format!(" [{}]", val);
        parts.push(&value_str);
    }

    // Calculate padding to align descriptions
    // Standard clap help uses about 25-30 chars for option names
    let opt_len = parts.iter().map(|s| s.len()).sum::<usize>();
    let target_len = 27; // Approximate column for descriptions
    let padding = if opt_len < target_len {
        " ".repeat(target_len - opt_len)
    } else {
        "  ".to_string()
    };

    // Description (use positive flag's description as primary)
    let desc = if positive.description.is_empty() {
        "Enable/disable this option".to_string()
    } else {
        positive.description.clone()
    };

    format!("{}{}{}", parts.concat(), padding, desc)
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
