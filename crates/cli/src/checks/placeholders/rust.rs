// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust placeholder test detection.
//!
//! Detects `#[ignore]` attribute on tests and `todo!()` macro in test bodies.

/// Placeholder test detected in Rust code.
#[derive(Debug)]
pub struct RustPlaceholder {
    /// Line number where the placeholder was found.
    pub line: u32,
    /// Name of the test function.
    pub test_name: String,
    /// Kind of placeholder detected.
    pub kind: RustPlaceholderKind,
}

/// Kind of Rust placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustPlaceholderKind {
    /// `#[ignore]` or `#[ignore = "..."]` attribute.
    Ignore,
    /// `todo!()` macro in test body.
    Todo,
}

impl RustPlaceholderKind {
    /// Get string representation for violation type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Todo => "todo",
        }
    }
}

/// Find placeholder tests in Rust content.
///
/// Detects:
/// - `#[ignore]` or `#[ignore = "..."]` on test functions
/// - `todo!()` or `todo!("...")` in test function bodies
pub fn find_rust_placeholders(content: &str, patterns: &[String]) -> Vec<RustPlaceholder> {
    let detect_ignore = patterns.iter().any(|p| p == "ignore");
    let detect_todo = patterns.iter().any(|p| p == "todo");

    if !detect_ignore && !detect_todo {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut state = ParseState::default();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = (line_idx + 1) as u32;
        let trimmed = line.trim();

        // Track #[test] attribute
        if trimmed == "#[test]" {
            state.saw_test_attr = true;
            state.test_line = line_num;
            continue;
        }

        // Track #[ignore] attribute (only valid after #[test])
        if state.saw_test_attr
            && (trimmed.starts_with("#[ignore]") || trimmed.starts_with("#[ignore ="))
            && detect_ignore
        {
            state.saw_ignore_attr = true;
            state.ignore_line = line_num;
            continue;
        }

        // Match function name after #[test]
        if state.saw_test_attr && trimmed.starts_with("fn ") {
            if let Some(name) = extract_fn_name(trimmed) {
                // Report #[ignore] placeholder
                if state.saw_ignore_attr && detect_ignore {
                    results.push(RustPlaceholder {
                        line: state.ignore_line,
                        test_name: name.to_string(),
                        kind: RustPlaceholderKind::Ignore,
                    });
                }

                // Start scanning function body for todo!()
                if detect_todo {
                    state.current_test_name = Some(name.to_string());
                    state.in_test_body = true;
                    state.brace_depth = 0;
                }
            }
            state.saw_test_attr = false;
            state.saw_ignore_attr = false;
            continue;
        }

        // Track brace depth in test body
        if state.in_test_body {
            for ch in line.chars() {
                match ch {
                    '{' => state.brace_depth += 1,
                    '}' => {
                        state.brace_depth -= 1;
                        if state.brace_depth <= 0 {
                            state.in_test_body = false;
                            state.current_test_name = None;
                        }
                    }
                    _ => {}
                }
            }

            // Check for todo!() in body
            if (trimmed.contains("todo!()") || trimmed.contains("todo!("))
                && let Some(ref name) = state.current_test_name
            {
                results.push(RustPlaceholder {
                    line: line_num,
                    test_name: name.clone(),
                    kind: RustPlaceholderKind::Todo,
                });
            }
        }

        // Reset state on non-attribute lines (unless in test body)
        if !trimmed.starts_with('#') && !trimmed.is_empty() && !state.in_test_body {
            state.saw_test_attr = false;
            state.saw_ignore_attr = false;
        }
    }

    results
}

#[derive(Default)]
struct ParseState {
    saw_test_attr: bool,
    saw_ignore_attr: bool,
    test_line: u32,
    ignore_line: u32,
    in_test_body: bool,
    brace_depth: i32,
    current_test_name: Option<String>,
}

/// Extract function name from a line like "fn test_foo() {"
fn extract_fn_name(line: &str) -> Option<&str> {
    line.strip_prefix("fn ")?
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod tests;
