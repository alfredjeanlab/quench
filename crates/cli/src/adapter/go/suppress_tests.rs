#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_nolint_all() {
    let content = "//nolint\nfoo()";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].line, 0);
    assert!(directives[0].codes.is_empty());
}

#[test]
fn parses_nolint_single_code() {
    let content = "//nolint:errcheck\nfoo()";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].codes, vec!["errcheck"]);
}

#[test]
fn parses_nolint_multiple_codes() {
    let content = "//nolint:errcheck,gosec,staticcheck\nfoo()";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert_eq!(
        directives[0].codes,
        vec!["errcheck", "gosec", "staticcheck"]
    );
}

#[test]
fn parses_inline_comment_as_justification() {
    let content = "//nolint:errcheck // This error is intentionally ignored";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert!(directives[0].has_comment);
    assert_eq!(
        directives[0].comment_text.as_deref(),
        Some("This error is intentionally ignored")
    );
}

#[test]
fn parses_comment_on_previous_line() {
    let content = "// OK: This is justified\n//nolint:errcheck";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert!(directives[0].has_comment);
}

#[test]
fn no_comment_when_blank_line_before() {
    let content = "// Comment\n\n//nolint:errcheck";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert!(!directives[0].has_comment);
}

#[test]
fn respects_required_comment_pattern() {
    let content = "// Some comment\n//nolint:errcheck";
    let directives = parse_nolint_directives(content, Some("// OK:"));

    assert_eq!(directives.len(), 1);
    assert!(!directives[0].has_comment); // "Some comment" doesn't match "OK:"
}

#[test]
fn finds_required_comment_pattern() {
    let content = "// OK: Justified reason\n//nolint:errcheck";
    let directives = parse_nolint_directives(content, Some("// OK:"));

    assert_eq!(directives.len(), 1);
    assert!(directives[0].has_comment);
}

#[test]
fn parses_nolint_at_end_of_line() {
    let content = "foo() //nolint:errcheck";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].codes, vec!["errcheck"]);
}

#[test]
fn multiple_directives_in_file() {
    let content = "//nolint:errcheck\nfoo()\n//nolint:gosec\nbar()";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 2);
    assert_eq!(directives[0].line, 0);
    assert_eq!(directives[1].line, 2);
}
