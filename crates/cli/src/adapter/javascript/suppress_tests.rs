// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use yare::parameterized;

use super::*;

// =============================================================================
// ESLint Parsing Tests
// =============================================================================

#[parameterized(
    no_rules = { "// eslint-disable-next-line\nconsole.log('test');", &[] },
    single_rule = { "// eslint-disable-next-line no-console\nconsole.log('test');", &["no-console"] },
    multiple_rules = { "// eslint-disable-next-line no-console, no-debugger\nconsole.log('test');", &["no-console", "no-debugger"] },
    typescript_scoped = { "// eslint-disable-next-line @typescript-eslint/no-explicit-any\nconst x: any = {};", &["@typescript-eslint/no-explicit-any"] },
)]
fn eslint_next_line_codes(content: &str, expected_codes: &[&str]) {
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, EslintSuppressKind::DisableNextLine);
    assert_eq!(result[0].codes, expected_codes);
}

#[parameterized(
    with_inline_reason = {
        "// eslint-disable-next-line no-console -- debugging legacy code\nconsole.log('test');",
        &["no-console"],
        true,
        Some("debugging legacy code")
    },
    with_comment_above = {
        "// Legacy API requires this pattern\n// eslint-disable-next-line no-console\nconsole.log('test');",
        &["no-console"],
        true,
        Some("Legacy API requires this pattern")
    },
    inline_reason_no_rules = {
        "// eslint-disable-next-line -- just a reason\nconsole.log('test');",
        &[],
        true,
        Some("just a reason")
    },
)]
fn eslint_next_line_with_comment(
    content: &str,
    expected_codes: &[&str],
    has_comment: bool,
    comment_text: Option<&str>,
) {
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, expected_codes);
    assert_eq!(result[0].has_comment, has_comment);
    assert_eq!(result[0].comment_text.as_deref(), comment_text);
}

#[test]
fn eslint_next_line_with_required_pattern_matching() {
    let content = "// JUSTIFIED: needed for test\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, Some("// JUSTIFIED:"));

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
    assert!(result[0].comment_text.as_ref().unwrap().starts_with("JUSTIFIED:"));
}

#[test]
fn eslint_next_line_with_required_pattern_not_matching() {
    let content =
        "// Some random comment\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, Some("// JUSTIFIED:"));

    assert_eq!(result.len(), 1);
    assert!(!result[0].has_comment);
}

#[parameterized(
    block_no_rules = {
        "/* eslint-disable */\nconst x = 1;\n/* eslint-enable */",
        EslintSuppressKind::DisableBlock,
        &[]
    },
    block_with_rules = {
        "/* eslint-disable no-console, no-alert */\nconst x = 1;\n/* eslint-enable */",
        EslintSuppressKind::DisableBlock,
        &["no-console", "no-alert"]
    },
    file_level = {
        "/* eslint-disable */\nconst x = 1;",
        EslintSuppressKind::DisableFile,
        &[]
    },
)]
fn eslint_block_and_file(
    content: &str,
    expected_kind: EslintSuppressKind,
    expected_codes: &[&str],
) {
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, expected_kind);
    assert_eq!(result[0].codes, expected_codes);
}

// =============================================================================
// Biome Parsing Tests
// =============================================================================

#[parameterized(
    single_rule = {
        "// biome-ignore lint/suspicious/noExplicitAny\nconst x: any = {};",
        &["lint/suspicious/noExplicitAny"]
    },
    multiple_rules = {
        "// biome-ignore lint/suspicious/noExplicitAny lint/style/noVar\nvar x: any = {};",
        &["lint/suspicious/noExplicitAny", "lint/style/noVar"]
    },
)]
fn biome_ignore_codes(content: &str, expected_codes: &[&str]) {
    let result = parse_biome_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, expected_codes);
    assert!(!result[0].has_explanation);
}

#[parameterized(
    with_explanation = {
        "// biome-ignore lint/suspicious/noExplicitAny: legacy API requires any\nconst x: any = {};",
        true,
        Some("legacy API requires any")
    },
    empty_explanation = {
        "// biome-ignore lint/suspicious/noExplicitAny:\nconst x: any = {};",
        false,
        None
    },
)]
fn biome_ignore_explanation(content: &str, has_explanation: bool, explanation_text: Option<&str>) {
    let result = parse_biome_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].has_explanation, has_explanation);
    assert_eq!(result[0].explanation_text.as_deref(), explanation_text);
}

#[test]
fn biome_ignore_with_comment_above() {
    let content = "// Legacy code needs this\n// biome-ignore lint/suspicious/noExplicitAny\nconst x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(!result[0].has_explanation);
    assert!(result[0].has_comment);
    assert_eq!(result[0].comment_text.as_deref(), Some("Legacy code needs this"));
}

// =============================================================================
// Unified Parsing Tests
// =============================================================================

#[test]
fn unified_parses_both_eslint_and_biome() {
    let content = r#"// eslint-disable-next-line no-console
console.log('debug');

// biome-ignore lint/suspicious/noExplicitAny: needed
const x: any = {};
"#;
    let result = parse_javascript_suppresses(content, None);

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].tool, SuppressTool::Eslint);
    assert_eq!(result[0].codes, vec!["no-console"]);
    assert_eq!(result[1].tool, SuppressTool::Biome);
    assert_eq!(result[1].codes, vec!["lint/suspicious/noExplicitAny"]);
    assert!(result[1].has_comment);
}

#[test]
fn biome_explanation_counts_as_comment() {
    let content = "// biome-ignore lint/suspicious/noExplicitAny: API boundary\nconst x: any = {};";
    let result = parse_javascript_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
    assert_eq!(result[0].comment_text.as_deref(), Some("API boundary"));
}

#[test]
fn biome_comment_above_counts_as_comment() {
    let content = "// JUSTIFIED: needed for interop\n// biome-ignore lint/suspicious/noExplicitAny\nconst x: any = {};";
    let result = parse_javascript_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
}

#[test]
fn unified_sorted_by_line() {
    let content = r#"// biome-ignore lint/a: x
const a = 1;
// eslint-disable-next-line b
const b = 2;
// biome-ignore lint/c: y
const c = 3;
"#;
    let result = parse_javascript_suppresses(content, None);

    assert_eq!(result.len(), 3);
    assert!(result[0].line < result[1].line);
    assert!(result[1].line < result[2].line);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn handles_blank_lines_before_directive() {
    let content = "// Comment\n\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(!result[0].has_comment);
}

#[test]
fn skips_directive_lines_when_looking_for_comments() {
    let content = "// @ts-ignore\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(!result[0].has_comment);
}

#[test]
fn finds_comment_above_directive() {
    let content = "// This is needed because...\n// @ts-ignore\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
    assert_eq!(result[0].comment_text.as_deref(), Some("This is needed because..."));
}
