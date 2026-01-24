// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for placeholder detection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn find_js_placeholders_detects_test_todo() {
    let content = r#"
test.todo('parser should handle edge case');
test.todo('validates input correctly');
"#;
    let placeholders = find_js_placeholders(content);
    assert_eq!(placeholders.len(), 2);
    assert!(placeholders.contains(&"parser should handle edge case".to_string()));
}

#[test]
fn find_js_placeholders_detects_it_todo() {
    let content = "it.todo('should parse empty input');";
    let placeholders = find_js_placeholders(content);
    assert_eq!(placeholders.len(), 1);
}

#[test]
fn find_js_placeholders_detects_test_skip() {
    let content = r#"test.skip('temporarily disabled', () => {});"#;
    let placeholders = find_js_placeholders(content);
    assert_eq!(placeholders.len(), 1);
    assert!(placeholders.contains(&"temporarily disabled".to_string()));
}

#[test]
fn find_js_placeholders_empty_content() {
    assert!(find_js_placeholders("").is_empty());
}

#[test]
fn find_js_placeholders_no_placeholders() {
    let content = r#"test('parser works', () => { expect(true).toBe(true); });"#;
    assert!(find_js_placeholders(content).is_empty());
}

#[test]
fn find_js_placeholders_mixed_content() {
    let content = r#"
test('parser works', () => {});
test.todo('parser edge case');
it.skip('broken test', () => {});
"#;
    let placeholders = find_js_placeholders(content);
    assert_eq!(placeholders.len(), 2);
}

#[test]
fn find_js_placeholders_handles_different_quotes() {
    let content = r#"
test.todo('single quotes');
test.todo("double quotes");
test.todo(`backticks`);
"#;
    let placeholders = find_js_placeholders(content);
    assert_eq!(placeholders.len(), 3);
}

#[test]
fn find_rust_placeholders_detects_ignored_tests() {
    let content = r#"
#[test]
#[ignore = "TODO: implement parser"]
fn test_parser() { todo!() }

#[test]
fn test_other() { }
"#;
    let placeholders = find_rust_placeholders(content);
    assert_eq!(placeholders.len(), 1);
    assert_eq!(placeholders[0], "test_parser");
}

#[test]
fn find_rust_placeholders_empty_content() {
    assert!(find_rust_placeholders("").is_empty());
}

#[test]
fn find_rust_placeholders_no_ignored() {
    let content = r#"
#[test]
fn test_parser() { assert!(true); }
"#;
    assert!(find_rust_placeholders(content).is_empty());
}

#[test]
fn find_rust_placeholders_multiple() {
    let content = r#"
#[test]
#[ignore = "TODO"]
fn test_one() { todo!() }

#[test]
#[ignore]
fn test_two() { todo!() }
"#;
    let placeholders = find_rust_placeholders(content);
    assert_eq!(placeholders.len(), 2);
}

// =============================================================================
// EDGE CASE TESTS FOR RUST PLACEHOLDER DETECTION
// =============================================================================

#[test]
fn find_rust_placeholders_with_whitespace_in_attribute() {
    let content = r#"
#[ test ]
#[ignore = "TODO"]
fn test_parser() { todo!() }
"#;
    let result = find_rust_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(result.contains(&"test_parser".to_string()));
}

#[test]
fn find_rust_placeholders_with_reversed_attribute_order() {
    let content = r#"
#[ignore = "TODO: implement"]
#[test]
fn test_parser() { todo!() }
"#;
    let result = find_rust_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(result.contains(&"test_parser".to_string()));
}

#[test]
fn find_rust_placeholders_with_whitespace_in_ignore() {
    let content = r#"
#[test]
#[  ignore  ]
fn test_lexer() { todo!() }
"#;
    let result = find_rust_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(result.contains(&"test_lexer".to_string()));
}

#[test]
fn find_rust_placeholders_mixed_normal_and_reversed() {
    let content = r#"
#[test]
#[ignore = "TODO: normal order"]
fn test_normal() { todo!() }

#[ignore = "TODO: reversed order"]
#[test]
fn test_reversed() { todo!() }

#[test]
fn test_regular() { assert!(true); }
"#;
    let result = find_rust_placeholders(content);
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"test_normal".to_string()));
    assert!(result.contains(&"test_reversed".to_string()));
}

#[test]
fn is_test_attribute_handles_whitespace() {
    assert!(is_test_attribute("#[test]"));
    assert!(is_test_attribute("#[ test ]"));
    assert!(is_test_attribute("#[  test  ]"));
    assert!(!is_test_attribute("#[cfg(test)]"));
    assert!(!is_test_attribute("#[ignore]"));
}

#[test]
fn is_ignore_attribute_handles_variations() {
    assert!(is_ignore_attribute("#[ignore]"));
    assert!(is_ignore_attribute("#[ignore = \"reason\"]"));
    assert!(is_ignore_attribute("#[ ignore ]"));
    assert!(is_ignore_attribute("#[  ignore  =  \"reason\"  ]"));
    assert!(!is_ignore_attribute("#[test]"));
    assert!(!is_ignore_attribute("#[cfg(test)]"));
}

// =============================================================================
// EDGE CASE TESTS FOR JAVASCRIPT PLACEHOLDER DETECTION
// =============================================================================

#[test]
fn find_js_placeholders_with_escaped_single_quotes() {
    let content = r#"test.todo('doesn\'t break on escaped quotes');"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].contains("doesn't"),
        "Should unescape single quote"
    );
}

#[test]
fn find_js_placeholders_with_escaped_double_quotes() {
    let content = r#"test.todo("parser \"quoted\" test");"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].contains("\"quoted\""),
        "Should unescape double quotes"
    );
}

#[test]
fn find_js_placeholders_with_escaped_backticks() {
    let content = r#"test.todo(`template \`backtick\` test`);"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(
        result[0].contains("`backtick`"),
        "Should unescape backticks"
    );
}

#[test]
fn find_js_placeholders_multiple_with_escapes() {
    let content = r#"
test.todo('first test');
it.skip('second test with \'quotes\'', () => {});
describe.todo("third \"group\"");
"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 3);
    assert!(result[1].contains("'quotes'"));
    assert!(result[2].contains("\"group\""));
}

#[test]
fn find_js_placeholders_mixed_quote_types() {
    // Each placeholder uses different quote types
    let content = r#"
test.todo('single quotes');
test.todo("double quotes");
test.todo(`backticks`);
"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 3);
    assert!(result.contains(&"single quotes".to_string()));
    assert!(result.contains(&"double quotes".to_string()));
    assert!(result.contains(&"backticks".to_string()));
}
