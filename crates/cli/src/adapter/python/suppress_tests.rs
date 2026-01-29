// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for Python suppress directive parsing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// NOQA TESTS
// =============================================================================

#[test]
fn parse_noqa_blanket() {
    let content = "x = 1  # noqa";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::Noqa);
    assert!(suppresses[0].codes.is_empty(), "blanket noqa has no codes");
    assert_eq!(suppresses[0].line, 0);
}

#[test]
fn parse_noqa_single_code() {
    let content = "x = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::Noqa);
    assert_eq!(suppresses[0].codes, vec!["E501"]);
}

#[test]
fn parse_noqa_multiple_codes() {
    let content = "x = 1  # noqa: E501, W503";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["E501", "W503"]);
}

#[test]
fn parse_noqa_multiple_codes_with_spaces() {
    let content = "x = 1  # noqa: E501 , W503 , E302";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["E501", "W503", "E302"]);
}

#[test]
fn parse_noqa_case_insensitive() {
    let content = "x = 1  # NOQA: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["E501"]);
}

#[test]
fn noqa_as_full_line_comment() {
    let content = "# noqa: E501\nx = 1";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["E501"]);
}

// =============================================================================
// TYPE: IGNORE TESTS
// =============================================================================

#[test]
fn parse_type_ignore_blanket() {
    let content = "x: int = \"not int\"  # type: ignore";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::TypeIgnore);
    assert!(suppresses[0].codes.is_empty());
}

#[test]
fn parse_type_ignore_single_code() {
    let content = "x: int = \"not int\"  # type: ignore[assignment]";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::TypeIgnore);
    assert_eq!(suppresses[0].codes, vec!["assignment"]);
}

#[test]
fn parse_type_ignore_multiple_codes() {
    let content = "x = f(y)  # type: ignore[arg-type, return-value]";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["arg-type", "return-value"]);
}

#[test]
fn parse_type_ignore_no_space() {
    let content = "x = 1  # type:ignore[assignment]";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::TypeIgnore);
    assert_eq!(suppresses[0].codes, vec!["assignment"]);
}

// =============================================================================
// PYLINT: DISABLE TESTS
// =============================================================================

#[test]
fn parse_pylint_disable_single() {
    let content = "# pylint: disable=line-too-long\nx = 1";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::PylintDisable);
    assert_eq!(suppresses[0].codes, vec!["line-too-long"]);
}

#[test]
fn parse_pylint_disable_multiple() {
    let content = "# pylint: disable=line-too-long,unused-variable\nx = 1";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(
        suppresses[0].codes,
        vec!["line-too-long", "unused-variable"]
    );
}

#[test]
fn parse_pylint_disable_all() {
    let content = "# pylint: disable=all\nx = 1";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["all"]);
}

#[test]
fn parse_pylint_disable_no_space() {
    let content = "# pylint:disable=line-too-long\nx = 1";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["line-too-long"]);
}

#[test]
fn parse_pylint_inline() {
    let content = "x = 1  # pylint: disable=invalid-name";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["invalid-name"]);
}

// =============================================================================
// PRAGMA: NO COVER TESTS
// =============================================================================

#[test]
fn parse_pragma_no_cover() {
    let content = "if DEBUG:  # pragma: no cover\n    pass";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::PragmaNoCover);
    assert_eq!(suppresses[0].codes, vec!["coverage"]);
}

#[test]
fn parse_pragma_no_cover_full_line() {
    let content = "# pragma: no cover\nif DEBUG:\n    pass";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::PragmaNoCover);
}

#[test]
fn parse_pragma_no_cover_no_space() {
    let content = "if DEBUG:  # pragma:no cover\n    pass";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::PragmaNoCover);
}

// =============================================================================
// JUSTIFICATION COMMENT TESTS
// =============================================================================

#[test]
fn detects_justification_comment() {
    let content = "# Legacy API compatibility\nx = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
    assert_eq!(
        suppresses[0].comment_text.as_deref(),
        Some("Legacy API compatibility")
    );
}

#[test]
fn no_comment_when_blank_line_separates() {
    let content = "# Comment\n\nx = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment);
}

#[test]
fn no_comment_when_code_above() {
    let content = "y = 2\nx = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(
        !suppresses[0].has_comment,
        "code above should stop comment search"
    );
}

#[test]
fn requires_specific_pattern_when_configured() {
    let content = "# Random comment\nx = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, Some("# LEGACY:"));

    assert_eq!(suppresses.len(), 1);
    assert!(
        !suppresses[0].has_comment,
        "should require # LEGACY: pattern"
    );
}

#[test]
fn matches_specific_pattern() {
    let content = "# LEGACY: old code\nx = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, Some("# LEGACY:"));

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
}

// =============================================================================
// MULTIPLE SUPPRESSES TESTS
// =============================================================================

#[test]
fn parse_multiple_suppress_directives() {
    let content = "x = 1  # noqa: E501\ny = 2  # type: ignore";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 2);
    assert_eq!(suppresses[0].kind, PythonSuppressKind::Noqa);
    assert_eq!(suppresses[0].line, 0);
    assert_eq!(suppresses[1].kind, PythonSuppressKind::TypeIgnore);
    assert_eq!(suppresses[1].line, 1);
}

#[test]
fn parse_indented_directive() {
    let content = "    x = 1  # noqa: E501";
    let suppresses = parse_python_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["E501"]);
}

// =============================================================================
// DISPLAY TESTS
// =============================================================================

#[test]
fn suppress_kind_display() {
    assert_eq!(format!("{}", PythonSuppressKind::Noqa), "noqa");
    assert_eq!(
        format!("{}", PythonSuppressKind::TypeIgnore),
        "type: ignore"
    );
    assert_eq!(
        format!("{}", PythonSuppressKind::PylintDisable),
        "pylint: disable"
    );
    assert_eq!(
        format!("{}", PythonSuppressKind::PragmaNoCover),
        "pragma: no cover"
    );
}
