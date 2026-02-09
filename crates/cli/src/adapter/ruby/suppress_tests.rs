// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for RuboCop/Standard suppress directive parsing.

use super::*;

#[test]
fn parse_rubocop_disable_single_cop() {
    let content = "# rubocop:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, RubySuppressKind::Rubocop);
    assert_eq!(suppresses[0].codes, vec!["Style/StringLiterals"]);
    assert_eq!(suppresses[0].line, 0);
    assert!(!suppresses[0].is_todo);
}

#[test]
fn parse_rubocop_disable_multiple_cops() {
    let content = "# rubocop:disable Style/StringLiterals, Metrics/LineLength\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["Style/StringLiterals", "Metrics/LineLength"]);
}

#[test]
fn parse_rubocop_todo() {
    let content = "# rubocop:todo Metrics/MethodLength\ndef long_method\nend";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, RubySuppressKind::Rubocop);
    assert!(suppresses[0].is_todo);
}

#[test]
fn parse_standard_disable() {
    let content = "# standard:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, RubySuppressKind::Standard);
    assert_eq!(suppresses[0].codes, vec!["Style/StringLiterals"]);
}

#[test]
fn parse_inline_disable() {
    let content = "x = foo() # rubocop:disable Lint/UselessAssignment";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["Lint/UselessAssignment"]);
}

#[test]
fn detects_justification_comment() {
    let content =
        "# This is necessary for DSL support\n# rubocop:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
    assert_eq!(suppresses[0].comment_text.as_deref(), Some("This is necessary for DSL support"));
}

#[test]
fn no_comment_when_blank_line_separates() {
    let content = "# Comment\n\n# rubocop:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment);
}

#[test]
fn requires_specific_pattern_when_configured() {
    let content = "# Random comment\n# rubocop:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, Some("# LEGACY:"));

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment, "should require # LEGACY: pattern");
}

#[test]
fn matches_specific_pattern() {
    let content =
        "# LEGACY: old code that needs this\n# rubocop:disable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, Some("# LEGACY:"));

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
}

#[test]
fn ignores_rubocop_enable() {
    let content = "# rubocop:enable Style/StringLiterals\nfoo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert!(suppresses.is_empty(), "enable directive should not be detected");
}

#[test]
fn parse_multiple_suppress_directives() {
    let content = "# rubocop:disable Style/A\nfoo\n\n# rubocop:disable Style/B\nbar";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 2);
    assert_eq!(suppresses[0].codes, vec!["Style/A"]);
    assert_eq!(suppresses[0].line, 0);
    assert_eq!(suppresses[1].codes, vec!["Style/B"]);
    assert_eq!(suppresses[1].line, 3);
}

#[test]
fn indented_directive() {
    let content = "  # rubocop:disable Style/StringLiterals\n  foo = 'bar'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["Style/StringLiterals"]);
}

#[test]
fn comment_not_found_when_code_above() {
    let content = "foo = 1\n# rubocop:disable Style/StringLiterals\nbar = 'baz'";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment, "code above should stop comment search");
}

#[test]
fn parse_with_spaces_around_cops() {
    let content = "# rubocop:disable Style/A, Style/B , Style/C\nfoo";
    let suppresses = parse_ruby_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["Style/A", "Style/B", "Style/C"]);
}
