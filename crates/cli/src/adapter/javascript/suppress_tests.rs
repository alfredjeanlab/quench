// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// ESLint Parsing Tests
// =============================================================================

#[test]
fn eslint_next_line_no_rules() {
    let content = "// eslint-disable-next-line\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].line, 0);
    assert_eq!(result[0].kind, EslintSuppressKind::DisableNextLine);
    assert!(result[0].codes.is_empty());
    assert!(!result[0].has_comment);
}

#[test]
fn eslint_next_line_single_rule() {
    let content = "// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["no-console"]);
    assert!(!result[0].has_comment);
}

#[test]
fn eslint_next_line_multiple_rules() {
    let content = "// eslint-disable-next-line no-console, no-debugger\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["no-console", "no-debugger"]);
}

#[test]
fn eslint_next_line_with_inline_reason() {
    let content =
        "// eslint-disable-next-line no-console -- debugging legacy code\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["no-console"]);
    assert!(result[0].has_comment);
    assert_eq!(
        result[0].comment_text.as_deref(),
        Some("debugging legacy code")
    );
}

#[test]
fn eslint_next_line_with_comment_above() {
    let content = "// Legacy API requires this pattern\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
    assert_eq!(
        result[0].comment_text.as_deref(),
        Some("Legacy API requires this pattern")
    );
}

#[test]
fn eslint_next_line_with_required_pattern_matching() {
    let content = "// JUSTIFIED: needed for test\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, Some("// JUSTIFIED:"));

    assert_eq!(result.len(), 1);
    assert!(result[0].has_comment);
    assert!(
        result[0]
            .comment_text
            .as_ref()
            .unwrap()
            .starts_with("JUSTIFIED:")
    );
}

#[test]
fn eslint_next_line_with_required_pattern_not_matching() {
    let content =
        "// Some random comment\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, Some("// JUSTIFIED:"));

    assert_eq!(result.len(), 1);
    // Pattern doesn't match, so no valid comment found
    assert!(!result[0].has_comment);
}

#[test]
fn eslint_block_disable() {
    let content = "/* eslint-disable */\nconst x = 1;\n/* eslint-enable */";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, EslintSuppressKind::DisableBlock);
    assert!(result[0].codes.is_empty());
}

#[test]
fn eslint_block_with_rules() {
    let content = "/* eslint-disable no-console, no-alert */\nconst x = 1;\n/* eslint-enable */";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["no-console", "no-alert"]);
}

#[test]
fn eslint_file_level_disable_at_top() {
    let content = "/* eslint-disable */\nconst x = 1;";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].kind, EslintSuppressKind::DisableFile);
}

#[test]
fn eslint_typescript_scoped_rule() {
    let content =
        "// eslint-disable-next-line @typescript-eslint/no-explicit-any\nconst x: any = {};";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["@typescript-eslint/no-explicit-any"]);
}

// =============================================================================
// Biome Parsing Tests
// =============================================================================

#[test]
fn biome_ignore_single_rule() {
    let content = "// biome-ignore lint/suspicious/noExplicitAny\nconst x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].line, 0);
    assert_eq!(result[0].codes, vec!["lint/suspicious/noExplicitAny"]);
    assert!(!result[0].has_explanation);
}

#[test]
fn biome_ignore_multiple_rules() {
    let content =
        "// biome-ignore lint/suspicious/noExplicitAny lint/style/noVar\nvar x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].codes,
        vec!["lint/suspicious/noExplicitAny", "lint/style/noVar"]
    );
}

#[test]
fn biome_ignore_with_explanation() {
    let content = "// biome-ignore lint/suspicious/noExplicitAny: legacy API requires any\nconst x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].has_explanation);
    assert_eq!(
        result[0].explanation_text.as_deref(),
        Some("legacy API requires any")
    );
}

#[test]
fn biome_ignore_with_empty_explanation() {
    let content = "// biome-ignore lint/suspicious/noExplicitAny:\nconst x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    // Empty after colon doesn't count as explanation
    assert!(!result[0].has_explanation);
}

#[test]
fn biome_ignore_with_comment_above() {
    let content = "// Legacy code needs this\n// biome-ignore lint/suspicious/noExplicitAny\nconst x: any = {};";
    let result = parse_biome_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(!result[0].has_explanation);
    assert!(result[0].has_comment);
    assert_eq!(
        result[0].comment_text.as_deref(),
        Some("Legacy code needs this")
    );
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

    // ESLint directive
    assert_eq!(result[0].tool, SuppressTool::Eslint);
    assert_eq!(result[0].codes, vec!["no-console"]);

    // Biome directive
    assert_eq!(result[1].tool, SuppressTool::Biome);
    assert_eq!(result[1].codes, vec!["lint/suspicious/noExplicitAny"]);
    assert!(result[1].has_comment); // Has explanation
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
fn eslint_inline_reason_only_no_rules() {
    let content = "// eslint-disable-next-line -- just a reason\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    assert!(result[0].codes.is_empty());
    assert!(result[0].has_comment);
    assert_eq!(result[0].comment_text.as_deref(), Some("just a reason"));
}

#[test]
fn handles_blank_lines_before_directive() {
    let content = "// Comment\n\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    // Blank line stops the search, so no comment found
    assert!(!result[0].has_comment);
}

#[test]
fn skips_directive_lines_when_looking_for_comments() {
    // @ts-ignore is a directive, not a justification
    let content = "// @ts-ignore\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    // @ts-ignore is skipped as it's a directive pattern
    assert!(!result[0].has_comment);
}

#[test]
fn finds_comment_above_directive() {
    let content = "// This is needed because...\n// @ts-ignore\n// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);

    assert_eq!(result.len(), 1);
    // Skips @ts-ignore and finds the comment above it
    assert!(result[0].has_comment);
    assert_eq!(
        result[0].comment_text.as_deref(),
        Some("This is needed because...")
    );
}
