// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for shellcheck suppress directive parsing.

use super::*;

#[test]
fn parse_single_code() {
    let content = "# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
    assert_eq!(suppresses[0].line, 0);
}

#[test]
fn parse_multiple_codes() {
    let content = "# shellcheck disable=SC2034,SC2086\necho $var";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034", "SC2086"]);
}

#[test]
fn parse_no_space_after_hash() {
    let content = "#shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
}

#[test]
fn parse_with_spaces_around_codes() {
    let content = "# shellcheck disable=SC2034, SC2086 , SC2154\necho $var";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034", "SC2086", "SC2154"]);
}

#[test]
fn detects_justification_comment() {
    let content = "# This variable is used by subprocesses\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
    assert_eq!(
        suppresses[0].comment_text.as_deref(),
        Some("This variable is used by subprocesses")
    );
}

#[test]
fn no_comment_when_blank_line_separates() {
    let content = "# Comment\n\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment);
}

#[test]
fn requires_specific_pattern_when_configured() {
    let content = "# Random comment\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, Some("# OK:"));

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment, "should require # OK: pattern");
}

#[test]
fn matches_specific_pattern() {
    let content = "# OK: exported for subprocesses\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, Some("# OK:"));

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
}

#[test]
fn ignores_shellcheck_source_directive() {
    let content = "# shellcheck source=./lib.sh\n. ./lib.sh";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert!(suppresses.is_empty(), "source directive should not be detected");
}

#[test]
fn ignores_other_shellcheck_directives() {
    let content = "# shellcheck shell=bash\necho 'test'";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert!(suppresses.is_empty(), "shell directive should not be detected");
}

#[test]
fn parse_multiple_suppress_directives() {
    let content = "# shellcheck disable=SC2034\nUNUSED=1\n\n# shellcheck disable=SC2086\necho $var";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 2);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
    assert_eq!(suppresses[0].line, 0);
    assert_eq!(suppresses[1].codes, vec!["SC2086"]);
    assert_eq!(suppresses[1].line, 3);
}

#[test]
fn indented_directive() {
    let content = "  # shellcheck disable=SC2034\n  UNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
}

#[test]
fn comment_not_found_when_code_above() {
    let content = "echo hello\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment, "code above should stop comment search");
}

#[test]
fn parse_code_with_inline_comment() {
    // Inline comment after the code should be stripped
    let content = "# shellcheck disable=SC2090  # explanation here\neval \"$cmd\"";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2090"]);
}

#[test]
fn parse_multiple_codes_with_inline_comment() {
    let content = "# shellcheck disable=SC2034,SC2086  # these are fine\necho $var";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034", "SC2086"]);
}
