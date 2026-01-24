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
