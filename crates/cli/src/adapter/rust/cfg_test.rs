// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! #[cfg(test)] block detection.
//!
//! Parses Rust source files to identify line ranges inside #[cfg(test)] blocks.

use std::ops::Range;

/// Lexer state for tracking what context we're in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LexerState {
    /// Normal code - braces count
    Code,
    /// Inside a regular string "..."
    String,
    /// Inside a raw string r"..." or r#"..."#
    /// The usize is the number of # delimiters
    RawString(usize),
    /// Inside a character literal '...'
    Char,
}

/// Information about a single #[cfg(test)] block.
#[derive(Debug)]
pub struct CfgTestBlock {
    /// Line number where the attribute starts (0-indexed).
    pub attr_line: usize,
    /// Line range of the entire block (attribute through closing brace).
    pub range: Range<usize>,
}

/// Result of parsing a Rust file for #[cfg(test)] blocks.
#[derive(Debug, Default)]
pub struct CfgTestInfo {
    /// Detailed block information (for violation reporting).
    pub blocks: Vec<CfgTestBlock>,
    /// Line ranges (0-indexed) that are inside #[cfg(test)] blocks.
    /// Derived from blocks for backward compatibility.
    pub test_ranges: Vec<Range<usize>>,
}

/// State for tracking multi-line attribute parsing.
struct MultiLineAttr {
    /// Accumulated content of the attribute.
    content: String,
    /// Line where the attribute started.
    start_line: usize,
}

impl CfgTestInfo {
    /// Parse a Rust source file to find #[cfg(test)] block ranges.
    ///
    /// Uses a brace-counting approach with proper lexer state tracking:
    /// 1. Scan for #[cfg(test)] attribute (handles multi-line)
    /// 2. Count { and } to track block depth (skipping string/char literals)
    /// 3. Block ends when brace depth returns to 0
    ///
    /// External module declarations (`mod tests;`) are NOT counted as inline tests.
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        let mut in_cfg_test = false;
        let mut brace_depth: i32 = 0;
        let mut block_start = 0;
        let mut pending_attr: Option<MultiLineAttr> = None;
        // Track if we've seen the first opening brace after #[cfg(test)]
        let mut waiting_for_block_start = false;
        // Track if we're inside a block comment
        let mut in_block_comment = false;

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Track block comment state and strip commented portions
            let code_part = strip_comments(trimmed, &mut in_block_comment);

            // If entire line is in comment, skip it
            if code_part.trim().is_empty() {
                continue;
            }

            let trimmed = code_part.trim();

            // Handle multi-line attribute accumulation
            if let Some(ref mut attr) = pending_attr {
                attr.content.push_str(trimmed);

                // Check if the attribute is complete (has closing bracket)
                if trimmed.contains(")]") || (trimmed.contains(')') && attr.content.contains(")]"))
                {
                    // Attribute complete, check if it's cfg(test)
                    if is_cfg_test_content(&attr.content) {
                        in_cfg_test = true;
                        waiting_for_block_start = true;
                        block_start = attr.start_line;
                        brace_depth = 0;
                    }
                    pending_attr = None;
                }
                continue;
            }

            // Check for #[cfg(test)] attribute (single-line or start of multi-line)
            if !in_cfg_test && let Some(attr_state) = detect_cfg_attr_start(trimmed, line_idx) {
                match attr_state {
                    CfgAttrState::Complete(is_test) => {
                        if is_test {
                            in_cfg_test = true;
                            waiting_for_block_start = true;
                            block_start = line_idx;
                            brace_depth = 0;
                        }
                    }
                    CfgAttrState::Incomplete(attr) => {
                        pending_attr = Some(attr);
                    }
                }
                if pending_attr.is_some() || in_cfg_test {
                    continue;
                }
            }

            if in_cfg_test {
                // Skip additional attributes (like #[path = "..."])
                if trimmed.starts_with("#[") {
                    continue;
                }

                let delta = count_braces(trimmed);

                // If we're still waiting for the block to start and we see a line
                // without an opening brace that ends with ';', it's an external module
                if waiting_for_block_start {
                    if delta > 0 {
                        // Found opening brace - this is an inline block
                        waiting_for_block_start = false;
                        brace_depth += delta;
                    } else if trimmed.ends_with(';') && !trimmed.is_empty() {
                        // External module declaration (e.g., "mod tests;")
                        // Not an inline test block
                        in_cfg_test = false;
                        waiting_for_block_start = false;
                        continue;
                    }
                    // Otherwise keep waiting (might be blank line or comment)
                    continue;
                }

                brace_depth += delta;

                if brace_depth == 0 && delta < 0 {
                    // Block ended (we saw a closing brace that brought us to 0)
                    let range = block_start..line_idx + 1;
                    info.blocks.push(CfgTestBlock {
                        attr_line: block_start,
                        range: range.clone(),
                    });
                    info.test_ranges.push(range);
                    in_cfg_test = false;
                }
            }
        }

        info
    }

    /// Check if a line (0-indexed) is inside a #[cfg(test)] block.
    pub fn is_test_line(&self, line_idx: usize) -> bool {
        self.test_ranges.iter().any(|r| r.contains(&line_idx))
    }

    /// Check if file has any inline #[cfg(test)] blocks.
    pub fn has_inline_tests(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Get the first inline test location (for violation reporting).
    /// Returns 0-indexed line number.
    pub fn first_inline_test_line(&self) -> Option<usize> {
        self.blocks.first().map(|b| b.attr_line)
    }
}

/// Result of detecting a cfg attribute start.
enum CfgAttrState {
    /// Complete single-line attribute, bool indicates if it's cfg(test).
    Complete(bool),
    /// Incomplete multi-line attribute that needs more lines.
    Incomplete(MultiLineAttr),
}

/// Strip comments from a line, handling both line comments and block comments.
/// Updates the `in_block_comment` state for multi-line block comments.
/// Returns the code portion of the line (empty if entire line is comment).
fn strip_comments(line: &str, in_block_comment: &mut bool) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            // Looking for end of block comment */
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next(); // consume '/'
                *in_block_comment = false;
            }
            // Skip everything inside block comment
        } else if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment - rest of line is comment
                    break;
                }
                Some('*') => {
                    // Start of block comment
                    chars.next(); // consume '*'
                    *in_block_comment = true;
                }
                _ => {
                    // Just a regular '/' character
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Detect if a line starts a #[cfg(...)] attribute.
/// Returns the state of the attribute parsing.
fn detect_cfg_attr_start(line: &str, line_idx: usize) -> Option<CfgAttrState> {
    // Check for #[cfg( pattern
    let has_cfg = line.starts_with("#[cfg(") || line.contains("#[cfg(");

    if !has_cfg {
        return None;
    }

    Some({
        // Check if the attribute is complete on this line
        if line.contains(")]") {
            // Single-line case
            CfgAttrState::Complete(is_cfg_test_content(line))
        } else {
            // Multi-line case - attribute continues on next line(s)
            CfgAttrState::Incomplete(MultiLineAttr {
                content: line.to_string(),
                start_line: line_idx,
            })
        }
    })
}

/// Check if accumulated cfg attribute content contains "test".
fn is_cfg_test_content(content: &str) -> bool {
    // Look for cfg(test) pattern with optional whitespace
    // The content may be the full attribute like "#[cfg(test)]"
    // or accumulated multi-line content like "#[cfg(\n    test\n)]"

    // Extract the part between #[cfg( and )]
    if let Some(start) = content.find("#[cfg(") {
        let after_cfg = &content[start + 6..];
        if let Some(end) = after_cfg.find(")]") {
            let inner = &after_cfg[..end];
            // Check if inner contains "test" as a standalone word
            // Handle cases like "test", " test ", "all(test, ...)", etc.
            return inner
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|word| word == "test");
        }
    }
    false
}

/// Count brace depth changes in a line, accounting for string/char literals.
fn count_braces(line: &str) -> i32 {
    let mut depth_change: i32 = 0;
    let mut state = LexerState::Code;
    let mut chars = line.chars().peekable();
    let mut prev_char = '\0';

    while let Some(ch) = chars.next() {
        match state {
            LexerState::Code => {
                match ch {
                    '"' => {
                        state = LexerState::String;
                    }
                    'r' => {
                        // Check for raw string: r"..." or r#"..."#
                        if let Some(&next) = chars.peek() {
                            if next == '"' {
                                chars.next(); // consume "
                                state = LexerState::RawString(0);
                            } else if next == '#' {
                                // Count consecutive #s
                                let mut hash_count = 0;
                                while chars.peek() == Some(&'#') {
                                    chars.next();
                                    hash_count += 1;
                                }
                                // Must be followed by "
                                if chars.peek() == Some(&'"') {
                                    chars.next();
                                    state = LexerState::RawString(hash_count);
                                }
                            }
                        }
                    }
                    '\'' => {
                        // Character literal - but be careful about lifetimes
                        // Lifetime syntax: 'a, 'static, etc.
                        // Char literal: 'x', '\n', '\''
                        // Peek ahead to determine which
                        if let Some(&next) = chars.peek() {
                            // Check if this looks like a char literal
                            // Char literals are 'x' (single char) or '\x' (escaped)
                            let mut temp_chars = chars.clone();
                            if next == '\\' {
                                // Escape sequence: '\n', '\'', etc.
                                temp_chars.next(); // skip backslash
                                temp_chars.next(); // skip escaped char
                                if temp_chars.peek() == Some(&'\'') {
                                    state = LexerState::Char;
                                }
                            } else if temp_chars.next().is_some() {
                                // Single character 'x'
                                if temp_chars.peek() == Some(&'\'') {
                                    state = LexerState::Char;
                                }
                            }
                        }
                    }
                    '{' => depth_change += 1,
                    '}' => depth_change -= 1,
                    _ => {}
                }
            }
            LexerState::String => {
                if ch == '"' && prev_char != '\\' {
                    state = LexerState::Code;
                }
            }
            LexerState::RawString(hash_count) => {
                // Raw string ends with "### where # count matches
                if ch == '"' {
                    let mut matched = 0;
                    while matched < hash_count && chars.peek() == Some(&'#') {
                        chars.next();
                        matched += 1;
                    }
                    if matched == hash_count {
                        state = LexerState::Code;
                    }
                }
            }
            LexerState::Char => {
                // Char literal ends at closing '
                if ch == '\'' && prev_char != '\\' {
                    state = LexerState::Code;
                }
            }
        }
        prev_char = ch;
    }

    depth_change
}

#[cfg(test)]
#[path = "cfg_test_tests.rs"]
mod tests;
