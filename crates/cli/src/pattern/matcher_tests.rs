#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

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
