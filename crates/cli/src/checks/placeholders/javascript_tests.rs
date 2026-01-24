// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn patterns(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn detects_test_todo() {
    let content = r#"
test.todo('should handle edge case');
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "should handle edge case");
    assert_eq!(results[0].kind, JsPlaceholderKind::Todo);
    assert_eq!(results[0].line, 2);
}

#[test]
fn detects_it_todo() {
    let content = r#"
it.todo('validates input');
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "validates input");
    assert_eq!(results[0].kind, JsPlaceholderKind::Todo);
}

#[test]
fn detects_describe_todo() {
    let content = r#"
describe.todo('Parser module');
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "Parser module");
}

#[test]
fn detects_fixme() {
    let content = r#"
test.fixme('broken on empty input');
it.fixme('needs investigation');
"#;

    let results = find_js_placeholders(content, &patterns(&["fixme"]));
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].description, "broken on empty input");
    assert_eq!(results[0].kind, JsPlaceholderKind::Fixme);
    assert_eq!(results[1].description, "needs investigation");
}

#[test]
fn detects_skip() {
    let content = r#"
test.skip('slow test');
it.skip('flaky on CI');
describe.skip('deprecated API');
"#;

    let results = find_js_placeholders(content, &patterns(&["skip"]));
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].kind, JsPlaceholderKind::Skip);
    assert_eq!(results[1].kind, JsPlaceholderKind::Skip);
    assert_eq!(results[2].kind, JsPlaceholderKind::Skip);
}

#[test]
fn handles_double_quotes() {
    let content = r#"
test.todo("should handle edge case");
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "should handle edge case");
}

#[test]
fn handles_backticks() {
    let content = r#"
test.todo(`should handle edge case`);
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "should handle edge case");
}

#[test]
fn handles_whitespace_variations() {
    let content = r#"
test.todo( 'with space' );
it.todo(  "extra spaces"  );
"#;

    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].description, "with space");
    assert_eq!(results[1].description, "extra spaces");
}

#[test]
fn detects_multiple_patterns() {
    let content = r#"
test.todo('needs implementation');
test.fixme('broken test');
test.skip('slow test');
"#;

    let results = find_js_placeholders(content, &patterns(&["todo", "fixme", "skip"]));
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].kind, JsPlaceholderKind::Todo);
    assert_eq!(results[1].kind, JsPlaceholderKind::Fixme);
    assert_eq!(results[2].kind, JsPlaceholderKind::Skip);
}

#[test]
fn skips_when_pattern_not_enabled() {
    let content = r#"
test.todo('needs implementation');
test.fixme('broken test');
"#;

    // Only enable todo
    let results = find_js_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, JsPlaceholderKind::Todo);

    // Only enable fixme
    let results = find_js_placeholders(content, &patterns(&["fixme"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, JsPlaceholderKind::Fixme);

    // Enable neither
    let results = find_js_placeholders(content, &patterns(&[]));
    assert!(results.is_empty());
}

#[test]
fn ignores_regular_tests() {
    let content = r#"
test('regular test', () => {
    expect(true).toBe(true);
});

it('another regular test', () => {
    expect(1 + 1).toBe(2);
});
"#;

    let results = find_js_placeholders(content, &patterns(&["todo", "fixme", "skip"]));
    assert!(results.is_empty());
}

#[test]
fn kind_as_str() {
    assert_eq!(JsPlaceholderKind::Todo.as_str(), "todo");
    assert_eq!(JsPlaceholderKind::Fixme.as_str(), "fixme");
    assert_eq!(JsPlaceholderKind::Skip.as_str(), "skip");
}
