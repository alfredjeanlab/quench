// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Placeholder test detection for Rust and JavaScript/TypeScript.

use std::path::Path;

/// Check if a Rust test file contains placeholder tests (#[test] #[ignore]).
pub fn has_placeholder_test(
    test_path: &Path,
    source_base: &str,
    root: &Path,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(root.join(test_path)).map_err(|e| e.to_string())?;
    let prefixes =
        [source_base.to_string(), format!("test_{source_base}"), format!("{source_base}_test")];
    Ok(find_rust_placeholders(&content).iter().any(|n| prefixes.iter().any(|p| n.contains(p))))
}

/// Check if a JS/TS test file contains placeholder tests for a source file.
pub fn has_js_placeholder_test(
    test_path: &Path,
    source_base: &str,
    root: &Path,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(root.join(test_path)).map_err(|e| e.to_string())?;
    let base_lower = source_base.to_lowercase();
    Ok(find_js_placeholders(&content).iter().any(|n| n.to_lowercase().contains(&base_lower)))
}

/// Parse JS/TS test file for test.todo(), it.todo(), test.skip(), etc.
///
/// Handles:
/// - Single, double, and backtick quotes
/// - Escaped quotes within strings (e.g., `test.todo('doesn\'t work')`)
pub fn find_js_placeholders(content: &str) -> Vec<String> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Rust's regex crate doesn't support backreferences, so we use separate patterns
    // for each quote type. Each pattern handles escaped quotes within the string.
    static PAT_SINGLE: OnceLock<Option<Regex>> = OnceLock::new();
    static PAT_DOUBLE: OnceLock<Option<Regex>> = OnceLock::new();
    static PAT_BACKTICK: OnceLock<Option<Regex>> = OnceLock::new();

    let pat_single = PAT_SINGLE.get_or_init(|| {
        // Single quotes: match content that doesn't contain unescaped single quotes
        Regex::new(r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*'((?:[^'\\]|\\.)*)'"#).ok()
    });

    let pat_double = PAT_DOUBLE.get_or_init(|| {
        // Double quotes: match content that doesn't contain unescaped double quotes
        Regex::new(r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*"((?:[^"\\]|\\.)*)""#).ok()
    });

    let pat_backtick = PAT_BACKTICK.get_or_init(|| {
        // Backticks: match content that doesn't contain unescaped backticks
        Regex::new(r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*`((?:[^`\\]|\\.)*)`"#).ok()
    });

    let mut results = Vec::new();

    // Helper to unescape captured content
    let unescape = |s: &str| s.replace("\\'", "'").replace("\\\"", "\"").replace("\\`", "`");

    // Collect from each pattern
    if let Some(re) = pat_single.as_ref() {
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                results.push(unescape(m.as_str()));
            }
        }
    }

    if let Some(re) = pat_double.as_ref() {
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                results.push(unescape(m.as_str()));
            }
        }
    }

    if let Some(re) = pat_backtick.as_ref() {
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                results.push(unescape(m.as_str()));
            }
        }
    }

    results
}

/// Check if a line is a #[test] attribute (with whitespace tolerance).
fn is_test_attribute(trimmed: &str) -> bool {
    // Remove all whitespace for comparison
    let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    normalized == "#[test]"
}

/// Check if a line starts an #[ignore...] attribute.
fn is_ignore_attribute(trimmed: &str) -> bool {
    let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    normalized.starts_with("#[ignore")
}

/// Parse Rust test file for placeholder tests (#[test] #[ignore]).
///
/// Handles:
/// - Whitespace variations: `#[ test ]`, `#[  ignore  ]`
/// - Reversed attribute order: `#[ignore]` before `#[test]`
fn find_rust_placeholders(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut saw_test_attr = false;
    let mut saw_ignore_attr = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Handle #[test] after #[ignore] (reversed order)
        if is_test_attribute(trimmed) && saw_ignore_attr {
            saw_test_attr = true;
            continue;
        }

        // Handle #[test] (normal order)
        if is_test_attribute(trimmed) {
            saw_test_attr = true;
            saw_ignore_attr = false;
            continue;
        }

        // Handle #[ignore] after #[test]
        if saw_test_attr && is_ignore_attribute(trimmed) {
            saw_ignore_attr = true;
            continue;
        }

        // Handle #[ignore] before #[test] (reversed order)
        if is_ignore_attribute(trimmed) && !saw_test_attr {
            saw_ignore_attr = true;
            continue;
        }

        if saw_test_attr
            && saw_ignore_attr
            && trimmed.starts_with("fn ")
            && let Some(name_part) = trimmed.strip_prefix("fn ")
            && let Some(name) = name_part.split('(').next()
        {
            result.push(name.to_string());
            saw_test_attr = false;
            saw_ignore_attr = false;
            continue;
        }

        // Reset if we see something else
        if !trimmed.starts_with('#') && !trimmed.is_empty() {
            saw_test_attr = false;
            saw_ignore_attr = false;
        }
    }

    result
}

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod tests;
