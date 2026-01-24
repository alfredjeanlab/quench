// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn patterns(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn detects_ignore_attribute() {
    let content = r#"
#[test]
#[ignore]
fn test_parser() {
    assert!(true);
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["ignore"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].test_name, "test_parser");
    assert_eq!(results[0].kind, RustPlaceholderKind::Ignore);
    assert_eq!(results[0].line, 3); // Line of #[ignore]
}

#[test]
fn detects_ignore_with_message() {
    let content = r#"
#[test]
#[ignore = "TODO: implement parser"]
fn test_parser() {
    // Not implemented
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["ignore"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].test_name, "test_parser");
    assert_eq!(results[0].kind, RustPlaceholderKind::Ignore);
}

#[test]
fn detects_todo_macro() {
    let content = r#"
#[test]
fn test_lexer() {
    todo!()
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].test_name, "test_lexer");
    assert_eq!(results[0].kind, RustPlaceholderKind::Todo);
    assert_eq!(results[0].line, 4); // Line of todo!()
}

#[test]
fn detects_todo_with_message() {
    let content = r#"
#[test]
fn test_lexer() {
    todo!("implement lexer test")
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].test_name, "test_lexer");
    assert_eq!(results[0].kind, RustPlaceholderKind::Todo);
}

#[test]
fn detects_both_ignore_and_todo() {
    let content = r#"
#[test]
#[ignore]
fn test_parser() {
    todo!()
}

#[test]
fn test_lexer() {
    todo!()
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["ignore", "todo"]));
    assert_eq!(results.len(), 3);
    // First: ignore on test_parser
    assert_eq!(results[0].test_name, "test_parser");
    assert_eq!(results[0].kind, RustPlaceholderKind::Ignore);
    // Second: todo in test_parser
    assert_eq!(results[1].test_name, "test_parser");
    assert_eq!(results[1].kind, RustPlaceholderKind::Todo);
    // Third: todo in test_lexer
    assert_eq!(results[2].test_name, "test_lexer");
    assert_eq!(results[2].kind, RustPlaceholderKind::Todo);
}

#[test]
fn ignores_non_test_functions() {
    let content = r#"
fn helper_function() {
    todo!()
}

#[ignore]
fn not_a_test() {
    todo!()
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["ignore", "todo"]));
    assert!(results.is_empty());
}

#[test]
fn skips_when_pattern_not_enabled() {
    let content = r#"
#[test]
#[ignore]
fn test_parser() {
    todo!()
}
"#;

    // Only enable todo detection
    let results = find_rust_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, RustPlaceholderKind::Todo);

    // Only enable ignore detection
    let results = find_rust_placeholders(content, &patterns(&["ignore"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, RustPlaceholderKind::Ignore);

    // Enable neither
    let results = find_rust_placeholders(content, &patterns(&[]));
    assert!(results.is_empty());
}

#[test]
fn handles_nested_braces() {
    let content = r#"
#[test]
fn test_with_blocks() {
    {
        {
            todo!()
        }
    }
}
"#;

    let results = find_rust_placeholders(content, &patterns(&["todo"]));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].test_name, "test_with_blocks");
}

#[test]
fn extracts_function_names_correctly() {
    assert_eq!(extract_fn_name("fn test_foo() {"), Some("test_foo"));
    assert_eq!(extract_fn_name("fn test_bar()"), Some("test_bar"));
    assert_eq!(extract_fn_name("fn test_baz(x: i32) {"), Some("test_baz"));
    // Generics are included in the name (acceptable for display purposes)
    assert_eq!(extract_fn_name("fn helper<T>() {"), Some("helper<T>"));
    assert_eq!(extract_fn_name("not a function"), None);
}

#[test]
fn kind_as_str() {
    assert_eq!(RustPlaceholderKind::Ignore.as_str(), "ignore");
    assert_eq!(RustPlaceholderKind::Todo.as_str(), "todo");
}
