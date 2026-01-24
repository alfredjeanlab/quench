// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Compiled pattern matchers with automatic optimization.

use aho_corasick::AhoCorasick;
use memchr::memmem::Finder;
use regex::Regex;

/// A compiled pattern optimized for its structure.
pub enum CompiledPattern {
    /// Single literal string (fastest).
    Literal(Box<LiteralMatcher>),
    /// Multiple literal strings (Aho-Corasick).
    MultiLiteral(MultiLiteralMatcher),
    /// Full regex (most flexible).
    Regex(RegexMatcher),
}

/// Matcher for single literal strings using SIMD-optimized memchr.
pub struct LiteralMatcher {
    pattern: String,
    finder: Finder<'static>,
}

/// Matcher for multiple literal strings using Aho-Corasick automaton.
pub struct MultiLiteralMatcher {
    automaton: AhoCorasick,
}

/// Matcher for complex regex patterns.
pub struct RegexMatcher {
    regex: Regex,
}

/// A match found in content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// Byte offset where match starts.
    pub start: usize,
    /// Byte offset where match ends.
    pub end: usize,
}

/// A match with resolved line number.
#[derive(Debug, Clone)]
pub struct LineMatch {
    /// 1-based line number.
    pub line: u32,
    /// The matched text.
    pub text: String,
    /// Byte offset in file.
    pub offset: usize,
    /// The full line containing the match.
    pub line_content: String,
}

/// Error during pattern compilation.
#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),

    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
}

impl CompiledPattern {
    /// Compile a pattern string into an optimized matcher.
    ///
    /// Automatically selects the best matcher based on pattern structure:
    /// - Plain literal -> LiteralMatcher (fastest)
    /// - Pure alternation of literals -> MultiLiteralMatcher
    /// - Complex regex -> RegexMatcher
    pub fn compile(pattern: &str) -> Result<Self, PatternError> {
        if is_literal(pattern) {
            Ok(CompiledPattern::Literal(Box::new(LiteralMatcher::new(
                pattern,
            ))))
        } else if let Some(literals) = extract_alternation_literals(pattern) {
            Ok(CompiledPattern::MultiLiteral(MultiLiteralMatcher::new(
                &literals,
            )?))
        } else {
            Ok(CompiledPattern::Regex(RegexMatcher::new(pattern)?))
        }
    }

    /// Find all matches in content.
    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        match self {
            CompiledPattern::Literal(m) => m.find_all(content),
            CompiledPattern::MultiLiteral(m) => m.find_all(content),
            CompiledPattern::Regex(m) => m.find_all(content),
        }
    }

    /// Find all matches with line numbers.
    pub fn find_all_with_lines(&self, content: &str) -> Vec<LineMatch> {
        self.find_all(content)
            .into_iter()
            .map(|m| {
                let line = byte_offset_to_line(content, m.start);
                let text = content[m.start..m.end].to_string();
                let line_content = get_line_at_offset(content, m.start).to_string();
                LineMatch {
                    line,
                    text,
                    offset: m.start,
                    line_content,
                }
            })
            .collect()
    }
}

/// Check if pattern is a plain literal (no regex metacharacters).
fn is_literal(pattern: &str) -> bool {
    !pattern.chars().any(|c| {
        matches!(
            c,
            '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|'
        )
    })
}

/// Extract literals from patterns like "foo|bar|baz".
///
/// Returns None if the pattern is not a pure alternation of literals.
fn extract_alternation_literals(pattern: &str) -> Option<Vec<String>> {
    // Pattern must be pure alternation: "lit1|lit2|lit3"
    // Each alternative must be a literal
    let parts: Vec<&str> = pattern.split('|').collect();
    if parts.len() < 2 {
        return None;
    }

    for part in &parts {
        if !is_literal(part) {
            return None;
        }
    }

    Some(parts.into_iter().map(String::from).collect())
}

impl LiteralMatcher {
    /// Create a new literal matcher.
    ///
    /// Note: We leak the pattern string to get a 'static lifetime for Finder.
    /// This is acceptable since patterns are compiled once at startup and live
    /// for the program duration.
    pub fn new(pattern: &str) -> Self {
        let pattern_owned = pattern.to_string();
        let pattern_static: &'static str = Box::leak(pattern_owned.clone().into_boxed_str());
        Self {
            pattern: pattern_owned,
            finder: Finder::new(pattern_static),
        }
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.finder
            .find_iter(content.as_bytes())
            .map(|pos| PatternMatch {
                start: pos,
                end: pos + self.pattern.len(),
            })
            .collect()
    }
}

impl MultiLiteralMatcher {
    /// Create a new multi-literal matcher using Aho-Corasick.
    pub fn new(patterns: &[String]) -> Result<Self, PatternError> {
        let automaton = AhoCorasick::new(patterns)
            .map_err(|e| PatternError::InvalidPattern(format!("aho-corasick error: {}", e)))?;
        Ok(Self { automaton })
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.automaton
            .find_iter(content)
            .map(|m| PatternMatch {
                start: m.start(),
                end: m.end(),
            })
            .collect()
    }
}

impl RegexMatcher {
    /// Create a new regex matcher.
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        let regex = Regex::new(pattern)?;
        Ok(Self { regex })
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.regex
            .find_iter(content)
            .map(|m| PatternMatch {
                start: m.start(),
                end: m.end(),
            })
            .collect()
    }
}

/// Convert byte offset to 1-based line number.
pub fn byte_offset_to_line(content: &str, offset: usize) -> u32 {
    // Count newlines before offset
    content[..offset].bytes().filter(|&b| b == b'\n').count() as u32 + 1
}

/// Get the full line containing a byte offset.
pub fn get_line_at_offset(content: &str, offset: usize) -> &str {
    let start = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = content[offset..]
        .find('\n')
        .map(|i| offset + i)
        .unwrap_or(content.len());
    &content[start..end]
}

/// Get the line number and content for all lines in content.
/// Returns iterator of (1-based line number, line content).
pub fn lines_with_numbers(content: &str) -> impl Iterator<Item = (u32, &str)> {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| (i as u32 + 1, line))
}

#[cfg(test)]
#[path = "matcher_tests.rs"]
mod tests;
