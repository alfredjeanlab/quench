// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use yare::parameterized;

#[parameterized(
    nolint_all = { "//nolint\nfoo()", 0, &[] },
    nolint_single = { "//nolint:errcheck\nfoo()", 0, &["errcheck"] },
    nolint_multiple = { "//nolint:errcheck,gosec,staticcheck\nfoo()", 0, &["errcheck", "gosec", "staticcheck"] },
    nolint_end_of_line = { "foo() //nolint:errcheck", 0, &["errcheck"] },
)]
fn parse_nolint_directive(content: &str, expected_line: usize, expected_codes: &[&str]) {
    let directives = parse_nolint_directives(content, None);
    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].line, expected_line);
    assert_eq!(
        directives[0].codes,
        expected_codes.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
}

#[parameterized(
    inline_comment = { "//nolint:errcheck // This error is intentionally ignored", None, true, Some("This error is intentionally ignored") },
    previous_line = { "// OK: This is justified\n//nolint:errcheck", None, true, None },
    blank_line_before = { "// Comment\n\n//nolint:errcheck", None, false, None },
    required_pattern_miss = { "// Some comment\n//nolint:errcheck", Some("// OK:"), false, None },
    required_pattern_match = { "// OK: Justified reason\n//nolint:errcheck", Some("// OK:"), true, None },
)]
fn parse_justification_comment(
    content: &str,
    required_pattern: Option<&str>,
    expected_has_comment: bool,
    expected_comment_text: Option<&str>,
) {
    let directives = parse_nolint_directives(content, required_pattern);
    assert_eq!(directives.len(), 1);
    assert_eq!(directives[0].has_comment, expected_has_comment);
    if let Some(text) = expected_comment_text {
        assert_eq!(directives[0].comment_text.as_deref(), Some(text));
    }
}

#[test]
fn multiple_directives_in_file() {
    let content = "//nolint:errcheck\nfoo()\n//nolint:gosec\nbar()";
    let directives = parse_nolint_directives(content, None);

    assert_eq!(directives.len(), 2);
    assert_eq!(directives[0].line, 0);
    assert_eq!(directives[1].line, 2);
}
