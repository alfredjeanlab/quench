// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Document and verify the pattern matcher selection logic.
///
/// Patterns are classified into three tiers for optimal performance:
/// 1. LiteralMatcher (memchr) - For patterns without regex metacharacters
/// 2. MultiLiteralMatcher (Aho-Corasick) - For pure alternations of literals
/// 3. RegexMatcher (regex crate) - For everything else
mod matcher_selection {
    use super::*;

    fn matcher_type(pattern: &str) -> &'static str {
        match CompiledPattern::compile(pattern).unwrap() {
            CompiledPattern::Literal(_) => "Literal",
            CompiledPattern::MultiLiteral(_) => "MultiLiteral",
            CompiledPattern::Regex(_) => "Regex",
        }
    }

    #[test]
    fn literal_for_plain_strings() {
        // Single literal patterns -> LiteralMatcher (fastest, SIMD-optimized memchr)
        assert_eq!(matcher_type("FIXME"), "Literal");
        assert_eq!(matcher_type("TODO"), "Literal");
        assert_eq!(matcher_type("unsafe"), "Literal");
        assert_eq!(matcher_type("panic!"), "Literal"); // ! is not a regex metachar
    }

    #[test]
    fn multi_literal_for_pure_alternations() {
        // Pure alternations -> MultiLiteralMatcher (Aho-Corasick automaton)
        assert_eq!(matcher_type("TODO|FIXME|XXX"), "MultiLiteral");
        assert_eq!(matcher_type("foo|bar"), "MultiLiteral");
        assert_eq!(matcher_type("panic|abort|exit"), "MultiLiteral");
    }

    #[test]
    fn regex_for_metacharacters() {
        // Patterns with regex metacharacters -> RegexMatcher
        // Escaped dots and parens for literal matching
        assert_eq!(matcher_type(r"\.unwrap\(\)"), "Regex");
        assert_eq!(matcher_type(r"\.expect\("), "Regex");

        // Word boundaries
        assert_eq!(matcher_type(r"\bunsafe\b"), "Regex");
        assert_eq!(matcher_type(r"\b(TODO|FIXME|XXX)\b"), "Regex");

        // Character classes
        assert_eq!(matcher_type("[abc]"), "Regex");
        assert_eq!(matcher_type(r"\w+"), "Regex");

        // Quantifiers
        assert_eq!(matcher_type("foo+"), "Regex");
        assert_eq!(matcher_type("bar*"), "Regex");
        assert_eq!(matcher_type("baz?"), "Regex");

        // Anchors
        assert_eq!(matcher_type("^start"), "Regex");
        assert_eq!(matcher_type("end$"), "Regex");
    }

    #[test]
    fn alternation_with_metachar_falls_back_to_regex() {
        // If any part of an alternation has metacharacters, use regex
        assert_eq!(matcher_type(r"foo|\d+"), "Regex");
        assert_eq!(matcher_type(r"TODO.*|FIXME"), "Regex");
    }
}

#[test]
fn literal_matcher_finds_single_occurrence() {
    let m = LiteralMatcher::new("hello");
    let matches = m.find_all("say hello world");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 4);
    assert_eq!(matches[0].end, 9);
}

#[test]
fn literal_matcher_finds_multiple_occurrences() {
    let m = LiteralMatcher::new("a");
    let matches = m.find_all("abracadabra");
    assert_eq!(matches.len(), 5);
}

#[test]
fn multi_literal_matcher_finds_all_patterns() {
    let m = MultiLiteralMatcher::new(&["foo".to_string(), "bar".to_string()]).unwrap();
    let matches = m.find_all("foo and bar and foo");
    assert_eq!(matches.len(), 3);
}

#[test]
fn regex_matcher_handles_complex_patterns() {
    let m = RegexMatcher::new(r"\bunwrap\s*\(\s*\)").unwrap();
    let matches = m.find_all("x.unwrap() and y.unwrap( )");
    assert_eq!(matches.len(), 2);
}

#[test]
fn regex_matcher_invalid_pattern_errors() {
    let result = RegexMatcher::new(r"[invalid");
    assert!(result.is_err());
}

#[test]
fn is_literal_detects_plain_strings() {
    assert!(is_literal("hello"));
    assert!(is_literal("hello world"));
    assert!(is_literal("foo_bar"));
}

#[test]
fn is_literal_rejects_metacharacters() {
    assert!(!is_literal(r"\.unwrap"));
    assert!(!is_literal("foo|bar"));
    assert!(!is_literal("foo*"));
    assert!(!is_literal("foo?"));
    assert!(!is_literal("foo+"));
    assert!(!is_literal("foo.bar"));
    assert!(!is_literal("[abc]"));
    assert!(!is_literal("(foo)"));
    assert!(!is_literal("^foo"));
    assert!(!is_literal("foo$"));
}

#[test]
fn extract_alternation_returns_none_for_single() {
    assert!(extract_alternation_literals("foo").is_none());
}

#[test]
fn extract_alternation_returns_literals() {
    let result = extract_alternation_literals("foo|bar|baz");
    assert_eq!(
        result,
        Some(vec![
            "foo".to_string(),
            "bar".to_string(),
            "baz".to_string()
        ])
    );
}

#[test]
fn extract_alternation_returns_none_for_regex_alternatives() {
    assert!(extract_alternation_literals(r"foo|bar\d+").is_none());
}

#[test]
fn byte_offset_to_line_handles_empty_content() {
    assert_eq!(byte_offset_to_line("", 0), 1);
}

#[test]
fn byte_offset_to_line_handles_no_newlines() {
    assert_eq!(byte_offset_to_line("hello world", 5), 1);
}

#[test]
fn byte_offset_to_line_handles_multiple_newlines() {
    let content = "a\nb\nc\nd";
    assert_eq!(byte_offset_to_line(content, 0), 1); // 'a'
    assert_eq!(byte_offset_to_line(content, 2), 2); // 'b'
    assert_eq!(byte_offset_to_line(content, 4), 3); // 'c'
    assert_eq!(byte_offset_to_line(content, 6), 4); // 'd'
}

#[test]
fn get_line_at_offset_returns_full_line() {
    let content = "first line\nsecond line\nthird line";
    assert_eq!(get_line_at_offset(content, 0), "first line");
    assert_eq!(get_line_at_offset(content, 5), "first line");
    assert_eq!(get_line_at_offset(content, 11), "second line");
    assert_eq!(get_line_at_offset(content, 15), "second line");
    assert_eq!(get_line_at_offset(content, 23), "third line");
}

#[test]
fn get_line_at_offset_handles_single_line() {
    let content = "single line content";
    assert_eq!(get_line_at_offset(content, 0), "single line content");
    assert_eq!(get_line_at_offset(content, 10), "single line content");
}

#[test]
fn lines_with_numbers_iterates_correctly() {
    let content = "first\nsecond\nthird";
    let lines: Vec<_> = lines_with_numbers(content).collect();
    assert_eq!(lines, vec![(1, "first"), (2, "second"), (3, "third")]);
}

#[test]
fn find_all_with_lines_includes_line_content() {
    let pattern = CompiledPattern::compile("target").unwrap();
    let content = "before\ntarget here\nafter";
    let matches = pattern.find_all_with_lines(content);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 2);
    assert_eq!(matches[0].text, "target");
    assert_eq!(matches[0].line_content, "target here");
}
