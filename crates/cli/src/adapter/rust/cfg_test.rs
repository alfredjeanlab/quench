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

/// Result of parsing a Rust file for #[cfg(test)] blocks.
#[derive(Debug, Default)]
pub struct CfgTestInfo {
    /// Line ranges (0-indexed) that are inside #[cfg(test)] blocks.
    pub test_ranges: Vec<Range<usize>>,
}

impl CfgTestInfo {
    /// Parse a Rust source file to find #[cfg(test)] block ranges.
    ///
    /// Uses a brace-counting approach with proper lexer state tracking:
    /// 1. Scan for #[cfg(test)] attribute
    /// 2. Count { and } to track block depth (skipping string/char literals)
    /// 3. Block ends when brace depth returns to 0
    ///
    /// Limitations:
    /// - Multi-line attributes not fully supported
    /// - `mod tests;` (external module) declarations need file-level classification
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        let mut in_cfg_test = false;
        let mut brace_depth: i32 = 0;
        let mut block_start = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Check for #[cfg(test)] attribute
            if !in_cfg_test && is_cfg_test_attr(trimmed) {
                in_cfg_test = true;
                block_start = line_idx;
                brace_depth = 0;
                continue;
            }

            if in_cfg_test {
                let delta = count_braces(trimmed);
                brace_depth += delta;

                if brace_depth == 0 && delta < 0 {
                    // Block ended (we saw a closing brace that brought us to 0)
                    info.test_ranges.push(block_start..line_idx + 1);
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
}

/// Check if a line is a #[cfg(test)] attribute.
pub(crate) fn is_cfg_test_attr(line: &str) -> bool {
    // Match #[cfg(test)] with optional whitespace
    line.starts_with("#[cfg(test)]")
        || line.starts_with("#[cfg( test )]")
        || line.contains("#[cfg(test)]")
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
