// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript placeholder test detection.
//!
//! Detects `test.todo()`, `it.todo()`, `describe.todo()` and similar patterns.

use regex::Regex;
use std::sync::LazyLock;

/// Placeholder test detected in JavaScript/TypeScript.
#[derive(Debug)]
pub struct JsPlaceholder {
    /// Line number where the placeholder was found.
    pub line: u32,
    /// Test description from the placeholder call.
    pub description: String,
    /// Kind of placeholder detected.
    pub kind: JsPlaceholderKind,
}

/// Kind of JavaScript placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsPlaceholderKind {
    /// `test.todo()`, `it.todo()`, `describe.todo()`.
    Todo,
    /// `test.fixme()`, `it.fixme()` (Playwright pattern).
    Fixme,
    /// `test.skip()`, `it.skip()`, `describe.skip()` (optional).
    Skip,
}

impl JsPlaceholderKind {
    /// Get string representation for violation type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Fixme => "fixme",
            Self::Skip => "skip",
        }
    }
}

/// Regex for `test.todo('...')`, `it.todo('...')`, `describe.todo('...')`.
#[allow(clippy::expect_used)]
static TODO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:test|it|describe)\.todo\s*\(\s*['"`]([^'"`]+)['"`]"#)
        .expect("valid regex pattern")
});

/// Regex for `test.fixme('...')`, `it.fixme('...')` (Playwright).
#[allow(clippy::expect_used)]
static FIXME_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:test|it|describe)\.fixme\s*\(\s*['"`]([^'"`]+)['"`]"#)
        .expect("valid regex pattern")
});

/// Regex for `test.skip('...')`, `it.skip('...')`, `describe.skip('...')`.
#[allow(clippy::expect_used)]
static SKIP_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?:test|it|describe)\.skip\s*\(\s*['"`]([^'"`]+)['"`]"#)
        .expect("valid regex pattern")
});

/// Find placeholder tests in JavaScript/TypeScript content.
///
/// Detects:
/// - `test.todo('description')`, `it.todo('description')`, `describe.todo('description')`
/// - `test.fixme('description')`, `it.fixme('description')` (Playwright)
/// - `test.skip('description')`, `it.skip('description')` (optional)
pub fn find_js_placeholders(content: &str, patterns: &[String]) -> Vec<JsPlaceholder> {
    let detect_todo = patterns.iter().any(|p| p == "todo");
    let detect_fixme = patterns.iter().any(|p| p == "fixme");
    let detect_skip = patterns.iter().any(|p| p == "skip");

    if !detect_todo && !detect_fixme && !detect_skip {
        return Vec::new();
    }

    let mut results = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = (line_idx + 1) as u32;

        if detect_todo {
            for cap in TODO_PATTERN.captures_iter(line) {
                if let Some(desc) = cap.get(1) {
                    results.push(JsPlaceholder {
                        line: line_num,
                        description: desc.as_str().to_string(),
                        kind: JsPlaceholderKind::Todo,
                    });
                }
            }
        }

        if detect_fixme {
            for cap in FIXME_PATTERN.captures_iter(line) {
                if let Some(desc) = cap.get(1) {
                    results.push(JsPlaceholder {
                        line: line_num,
                        description: desc.as_str().to_string(),
                        kind: JsPlaceholderKind::Fixme,
                    });
                }
            }
        }

        if detect_skip {
            for cap in SKIP_PATTERN.captures_iter(line) {
                if let Some(desc) = cap.get(1) {
                    results.push(JsPlaceholder {
                        line: line_num,
                        description: desc.as_str().to_string(),
                        kind: JsPlaceholderKind::Skip,
                    });
                }
            }
        }
    }

    results
}

#[cfg(test)]
#[path = "javascript_tests.rs"]
mod tests;
