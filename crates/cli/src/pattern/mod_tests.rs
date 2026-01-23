// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn literal_pattern_matches_exact_string() {
    let p = CompiledPattern::compile("TODO").unwrap();
    let matches = p.find_all("line1\n// TODO: fix this\nline3");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 9); // "line1\n// " = 9 bytes
}

#[test]
fn literal_pattern_no_match() {
    let p = CompiledPattern::compile("FIXME").unwrap();
    let matches = p.find_all("line1\n// TODO: fix this\nline3");
    assert!(matches.is_empty());
}

#[test]
fn alternation_uses_multi_literal() {
    let p = CompiledPattern::compile("TODO|FIXME|XXX").unwrap();
    assert!(matches!(p, CompiledPattern::MultiLiteral(_)));
}

#[test]
fn multi_literal_finds_all_variants() {
    let p = CompiledPattern::compile("TODO|FIXME").unwrap();
    let matches = p.find_all("TODO here\nFIXME there");
    assert_eq!(matches.len(), 2);
}

#[test]
fn regex_pattern_with_metacharacters() {
    let p = CompiledPattern::compile(r"\.unwrap\(\)").unwrap();
    assert!(matches!(p, CompiledPattern::Regex(_)));
    let matches = p.find_all("x.unwrap() and y.unwrap()");
    assert_eq!(matches.len(), 2);
}

#[test]
fn line_number_first_line() {
    let content = "match here";
    assert_eq!(byte_offset_to_line(content, 0), 1);
}

#[test]
fn line_number_second_line() {
    let content = "line1\nmatch here";
    assert_eq!(byte_offset_to_line(content, 6), 2);
}

#[test]
fn line_number_third_line() {
    let content = "line1\nline2\nmatch here";
    assert_eq!(byte_offset_to_line(content, 12), 3);
}

#[test]
fn find_with_lines_returns_correct_data() {
    let p = CompiledPattern::compile("unwrap").unwrap();
    let content = "line1\nx.unwrap()\nline3";
    let matches = p.find_all_with_lines(content);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 2);
    assert_eq!(matches[0].text, "unwrap");
}
