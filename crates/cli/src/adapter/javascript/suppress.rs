// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript lint suppression directive parsing.
//!
//! Parses ESLint (`eslint-disable-next-line`, `eslint-disable`) and
//! Biome (`biome-ignore`) directives from source files.

use super::super::common::suppress::{CommentStyle, check_justification_comment};

// =============================================================================
// ESLint Types and Parsing
// =============================================================================

/// Represents a parsed ESLint suppress directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EslintSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Kind of directive.
    pub kind: EslintSuppressKind,
    /// Lint codes being suppressed (empty = all rules).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The actual comment text if found.
    pub comment_text: Option<String>,
}

/// Kind of ESLint suppress directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum EslintSuppressKind {
    /// `// eslint-disable-next-line [rules]`
    DisableNextLine,
    /// `/* eslint-disable */` ... `/* eslint-enable */` block
    DisableBlock,
    /// `/* eslint-disable */` at file top (no matching enable)
    DisableFile,
}

/// Comment style for JavaScript.
const JS_COMMENT_STYLE: CommentStyle = CommentStyle {
    prefix: "//",
    directive_patterns: &[
        "eslint-disable",
        "eslint-enable",
        "biome-ignore",
        "@ts-ignore",
        "@ts-expect-error",
    ],
    skip_prefixes: &[],
};

/// Parse ESLint rules from directive text.
///
/// Handles comma-separated rules and `-- reason` suffix.
/// Returns (codes, inline_reason).
fn parse_eslint_rules(text: &str) -> (Vec<String>, Option<String>) {
    let text = text.trim();
    if text.is_empty() {
        return (Vec::new(), None);
    }

    // Check for inline reason with `--`
    let (rules_part, reason) = if let Some(idx) = text.find(" -- ") {
        let (rules, reason) = text.split_at(idx);
        (rules.trim(), Some(reason[4..].trim().to_string()))
    } else if let Some(stripped) = text.strip_prefix("-- ") {
        // Only reason, no rules
        return (Vec::new(), Some(stripped.trim().to_string()));
    } else {
        (text, None)
    };

    // Parse comma-separated rules
    let codes: Vec<String> = rules_part
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .collect();

    (codes, reason)
}

/// Parse eslint-disable-next-line from a line.
/// Returns (rest_of_line_after_directive) if found.
fn parse_eslint_next_line_directive(line: &str) -> Option<&str> {
    let trimmed = line.trim();

    // Must start with //
    let rest = trimmed.strip_prefix("//")?;
    let rest = rest.trim_start();

    // Check for eslint-disable-next-line
    let rest = rest.strip_prefix("eslint-disable-next-line")?;

    // Return rest (rules and optional reason)
    Some(rest)
}

/// Parse /* eslint-disable */ block directive from a line.
/// Returns the rules part if found.
fn parse_eslint_block_disable(line: &str) -> Option<String> {
    // Find /* eslint-disable in the line
    let pos = line.find("/*")?;
    let rest = &line[pos + 2..];

    // Trim whitespace after /*
    let rest = rest.trim_start();

    // Check for eslint-disable
    let rest = rest.strip_prefix("eslint-disable")?;

    // Must not be eslint-disable-next-line
    if rest.starts_with("-next-line") {
        return None;
    }

    // Find closing */
    let end_pos = rest.find("*/")?;
    let rules = rest[..end_pos].trim().to_string();

    Some(rules)
}

/// Check if a line contains /* eslint-enable */
fn has_eslint_enable(line: &str) -> bool {
    if let Some(pos) = line.find("/*") {
        let rest = &line[pos + 2..];
        let rest = rest.trim_start();
        if let Some(after_enable) = rest.strip_prefix("eslint-enable") {
            return after_enable.trim_start().starts_with('*');
        }
    }
    false
}

/// Parse all ESLint suppress directives from content.
pub fn parse_eslint_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<EslintSuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Parse next-line directives
    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(rest) = parse_eslint_next_line_directive(line) {
            let (codes, inline_reason) = parse_eslint_rules(rest);

            // Check for justification: inline reason OR comment above
            let (has_comment, comment_text) = if inline_reason.is_some() {
                (true, inline_reason)
            } else {
                check_justification_comment(&lines, line_idx, comment_pattern, &JS_COMMENT_STYLE)
            };

            suppresses.push(EslintSuppress {
                line: line_idx,
                kind: EslintSuppressKind::DisableNextLine,
                codes,
                has_comment,
                comment_text,
            });
        }
    }

    // Parse block disables
    let mut block_starts: Vec<(usize, Vec<String>)> = Vec::new();
    let mut has_any_enable = false;

    for (line_idx, line) in lines.iter().enumerate() {
        // Find block disables
        if let Some(rules) = parse_eslint_block_disable(line) {
            let (codes, _) = parse_eslint_rules(&rules);
            block_starts.push((line_idx, codes));
        }

        // Track enables
        if has_eslint_enable(line) {
            has_any_enable = true;
        }
    }

    // Determine kind for each block start
    for (line_idx, codes) in block_starts {
        let kind = if has_any_enable {
            EslintSuppressKind::DisableBlock
        } else if line_idx < 5 {
            // File-level disable if near top of file
            EslintSuppressKind::DisableFile
        } else {
            EslintSuppressKind::DisableBlock
        };

        // Check for comment above
        let (has_comment, comment_text) =
            check_justification_comment(&lines, line_idx, comment_pattern, &JS_COMMENT_STYLE);

        suppresses.push(EslintSuppress {
            line: line_idx,
            kind,
            codes,
            has_comment,
            comment_text,
        });
    }

    suppresses
}

// =============================================================================
// Biome Types and Parsing
// =============================================================================

/// Represents a parsed Biome suppress directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiomeSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Lint codes being suppressed.
    pub codes: Vec<String>,
    /// Whether the directive has an explanation after the colon.
    pub has_explanation: bool,
    /// The explanation text after the colon.
    pub explanation_text: Option<String>,
    /// Whether a justification comment was found above.
    pub has_comment: bool,
    /// The actual comment text if found above.
    pub comment_text: Option<String>,
}

/// Parsed biome-ignore directive from a single line.
struct ParsedBiome {
    codes: Vec<String>,
    has_explanation: bool,
    explanation_text: Option<String>,
}

/// Parse biome-ignore from a line.
fn parse_biome_ignore_line(line: &str) -> Option<ParsedBiome> {
    let trimmed = line.trim();

    // Must start with //
    let rest = trimmed.strip_prefix("//")?;
    let rest = rest.trim_start();

    // Check for biome-ignore
    let rest = rest.strip_prefix("biome-ignore")?;

    // Must have whitespace after biome-ignore
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();

    // Split on colon to separate codes from explanation
    let (codes_part, explanation) = if let Some(colon_pos) = rest.find(':') {
        let codes = &rest[..colon_pos];
        let expl = rest[colon_pos + 1..].trim();
        (
            codes,
            if expl.is_empty() {
                None
            } else {
                Some(expl.to_string())
            },
        )
    } else {
        (rest, None)
    };

    // Parse lint codes (space-separated, all must start with "lint/")
    let codes: Vec<String> = codes_part
        .split_whitespace()
        .filter(|s| s.starts_with("lint/"))
        .map(|s| s.to_string())
        .collect();

    if codes.is_empty() {
        return None;
    }

    Some(ParsedBiome {
        codes,
        has_explanation: explanation.is_some(),
        explanation_text: explanation,
    })
}

/// Parse all Biome suppress directives from content.
pub fn parse_biome_suppresses(content: &str, comment_pattern: Option<&str>) -> Vec<BiomeSuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(parsed) = parse_biome_ignore_line(line) {
            // Check for comment above (for custom pattern requirements)
            let (has_comment, comment_text) =
                check_justification_comment(&lines, line_idx, comment_pattern, &JS_COMMENT_STYLE);

            suppresses.push(BiomeSuppress {
                line: line_idx,
                codes: parsed.codes,
                has_explanation: parsed.has_explanation,
                explanation_text: parsed.explanation_text,
                has_comment,
                comment_text,
            });
        }
    }

    suppresses
}

// =============================================================================
// Unified JavaScript Suppress Types
// =============================================================================

/// The tool that generated the suppress directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressTool {
    Eslint,
    Biome,
}

/// Unified suppress directive for violation checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaScriptSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Which tool's directive this is.
    pub tool: SuppressTool,
    /// Lint codes being suppressed (empty = all rules for ESLint).
    pub codes: Vec<String>,
    /// Whether a justification comment/explanation was found.
    pub has_comment: bool,
    /// The actual comment/explanation text.
    pub comment_text: Option<String>,
}

impl From<EslintSuppress> for JavaScriptSuppress {
    fn from(s: EslintSuppress) -> Self {
        Self {
            line: s.line,
            tool: SuppressTool::Eslint,
            codes: s.codes,
            has_comment: s.has_comment,
            comment_text: s.comment_text,
        }
    }
}

impl From<BiomeSuppress> for JavaScriptSuppress {
    fn from(s: BiomeSuppress) -> Self {
        // For Biome, count either the inline explanation OR comment above as valid
        let has_comment = s.has_explanation || s.has_comment;
        let comment_text = s.explanation_text.or(s.comment_text);

        Self {
            line: s.line,
            tool: SuppressTool::Biome,
            codes: s.codes,
            has_comment,
            comment_text,
        }
    }
}

/// Parse all JavaScript suppress directives (ESLint + Biome) from content.
pub fn parse_javascript_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<JavaScriptSuppress> {
    let mut suppresses: Vec<JavaScriptSuppress> = Vec::new();

    // Parse ESLint directives
    for s in parse_eslint_suppresses(content, comment_pattern) {
        suppresses.push(s.into());
    }

    // Parse Biome directives
    for s in parse_biome_suppresses(content, comment_pattern) {
        suppresses.push(s.into());
    }

    // Sort by line number for consistent ordering
    suppresses.sort_by_key(|s| s.line);

    suppresses
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
