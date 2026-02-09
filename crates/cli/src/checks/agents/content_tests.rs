// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

// =============================================================================
// TABLE DETECTION TESTS
// =============================================================================

#[test]
fn detect_tables_finds_basic_table() {
    let content = r#"# Title

| Command | Description |
|---------|-------------|
| build   | Build project |
"#;
    let issues = detect_tables(content);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 3); // 1-indexed
}

#[test]
fn detect_tables_finds_table_with_alignment() {
    let content = r#"
| Left | Center | Right |
|:-----|:------:|------:|
| a    | b      | c     |
"#;
    let issues = detect_tables(content);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 2);
}

#[test]
fn detect_tables_ignores_single_pipe() {
    let content = r#"
The command `foo | bar` uses a pipe.
"#;
    let issues = detect_tables(content);
    assert!(issues.is_empty());
}

#[test]
fn detect_tables_ignores_code_fence_pipe() {
    let content = r#"
```
echo "foo" | grep bar
```
"#;
    let issues = detect_tables(content);
    assert!(issues.is_empty());
}

#[test]
fn detect_tables_empty_content() {
    let issues = detect_tables("");
    assert!(issues.is_empty());
}

#[test]
fn detect_tables_no_separator_line() {
    // Not a valid table without separator
    let content = r#"
| Header | Header |
| Content | Content |
"#;
    let issues = detect_tables(content);
    assert!(issues.is_empty());
}

// =============================================================================
// BOX DIAGRAM DETECTION TESTS
// =============================================================================

#[test]
fn detect_box_diagrams_finds_simple_box() {
    let content = "┌─────────┐\n│  Main   │\n└─────────┘";
    let issues = detect_box_diagrams(content);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 1);
}

#[test]
fn detect_box_diagrams_finds_nested_boxes() {
    let content = r#"
┌──────────┐
│ ┌──────┐ │
│ │ Inner│ │
│ └──────┘ │
└──────────┘
"#;
    let issues = detect_box_diagrams(content);
    // Only reports first occurrence
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 2);
}

#[test]
fn detect_box_diagrams_ignores_single_char() {
    let content = "Use the │ character for separators.";
    let issues = detect_box_diagrams(content);
    assert!(issues.is_empty());
}

#[test]
fn detect_box_diagrams_empty_content() {
    let issues = detect_box_diagrams("");
    assert!(issues.is_empty());
}

#[test]
fn detect_box_diagrams_requires_two_chars() {
    // Single box char should not trigger
    let content = "└ alone";
    let issues = detect_box_diagrams(content);
    assert!(issues.is_empty());

    // Two box chars should trigger
    let content = "└─";
    let issues = detect_box_diagrams(content);
    assert_eq!(issues.len(), 1);
}

// =============================================================================
// MERMAID DETECTION TESTS
// =============================================================================

#[test]
fn detect_mermaid_finds_backtick_fence() {
    let content = r#"
# Diagram

```mermaid
graph TD
    A --> B
```
"#;
    let issues = detect_mermaid_blocks(content);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 4);
}

#[test]
fn detect_mermaid_finds_tilde_fence() {
    let content = r#"
~~~mermaid
graph LR
    A --> B
~~~
"#;
    let issues = detect_mermaid_blocks(content);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].line, 2);
}

#[test]
fn detect_mermaid_finds_multiple_blocks() {
    let content = r#"
```mermaid
graph TD
```

```mermaid
sequenceDiagram
```
"#;
    let issues = detect_mermaid_blocks(content);
    assert_eq!(issues.len(), 2);
}

#[test]
fn detect_mermaid_ignores_other_code_blocks() {
    let content = r#"
```rust
fn main() {}
```

```javascript
console.log("hi");
```
"#;
    let issues = detect_mermaid_blocks(content);
    assert!(issues.is_empty());
}

#[test]
fn detect_mermaid_empty_content() {
    let issues = detect_mermaid_blocks("");
    assert!(issues.is_empty());
}

#[test]
fn detect_mermaid_with_indentation() {
    let content = "   ```mermaid\n   graph TD\n   ```";
    let issues = detect_mermaid_blocks(content);
    assert_eq!(issues.len(), 1);
}

// =============================================================================
// CONTENT TYPE TESTS
// =============================================================================

#[test]
fn content_type_violation_type() {
    assert_eq!(ContentType::MarkdownTable.violation_type(), "forbidden_table");
    assert_eq!(ContentType::BoxDiagram.violation_type(), "forbidden_diagram");
    assert_eq!(ContentType::MermaidBlock.violation_type(), "forbidden_mermaid");
}

#[test]
fn content_type_advice_not_empty() {
    assert!(!ContentType::MarkdownTable.advice().is_empty());
    assert!(!ContentType::BoxDiagram.advice().is_empty());
    assert!(!ContentType::MermaidBlock.advice().is_empty());
}

// =============================================================================
// SIZE LIMIT TESTS
// =============================================================================

#[test]
fn check_line_count_under_limit() {
    let content = "line1\nline2\nline3";
    let result = check_line_count(content, 10);
    assert!(result.is_none());
}

#[test]
fn check_line_count_at_limit() {
    let content = "line1\nline2\nline3";
    let result = check_line_count(content, 3);
    assert!(result.is_none());
}

#[test]
fn check_line_count_over_limit() {
    let content = "line1\nline2\nline3\nline4";
    let result = check_line_count(content, 3);
    assert!(result.is_some());
    let violation = result.unwrap();
    assert_eq!(violation.value, 4);
    assert_eq!(violation.threshold, 3);
}

#[test]
fn check_line_count_empty() {
    let result = check_line_count("", 10);
    assert!(result.is_none());
}

#[test]
fn check_token_count_under_limit() {
    // 20 chars = ~5 tokens
    let content = "12345678901234567890";
    let result = check_token_count(content, 10);
    assert!(result.is_none());
}

#[test]
fn check_token_count_over_limit() {
    // 80 chars = ~20 tokens
    let content = "a".repeat(80);
    let result = check_token_count(&content, 10);
    assert!(result.is_some());
    let violation = result.unwrap();
    assert_eq!(violation.value, 20);
    assert_eq!(violation.threshold, 10);
}

#[test]
fn check_token_count_empty() {
    let result = check_token_count("", 10);
    assert!(result.is_none());
}

#[test]
fn size_limit_type_advice_lines() {
    let advice = SizeLimitType::Lines.advice(100, 50);
    assert!(advice.contains("100"));
    assert!(advice.contains("50"));
    assert!(advice.contains("lines"));
}

#[test]
fn size_limit_type_advice_tokens() {
    let advice = SizeLimitType::Tokens.advice(500, 200);
    assert!(advice.contains("500"));
    assert!(advice.contains("200"));
    assert!(advice.contains("tokens"));
}
