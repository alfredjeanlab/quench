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
    let prefixes = [
        source_base.to_string(),
        format!("test_{source_base}"),
        format!("{source_base}_test"),
    ];
    Ok(find_rust_placeholders(&content)
        .iter()
        .any(|n| prefixes.iter().any(|p| n.contains(p))))
}

/// Check if a JS/TS test file contains placeholder tests for a source file.
pub fn has_js_placeholder_test(
    test_path: &Path,
    source_base: &str,
    root: &Path,
) -> Result<bool, String> {
    let content = std::fs::read_to_string(root.join(test_path)).map_err(|e| e.to_string())?;
    let base_lower = source_base.to_lowercase();
    Ok(find_js_placeholders(&content)
        .iter()
        .any(|n| n.to_lowercase().contains(&base_lower)))
}

/// Parse JS/TS test file for test.todo(), it.todo(), test.skip(), etc.
pub fn find_js_placeholders(content: &str) -> Vec<String> {
    use regex::Regex;
    use std::sync::OnceLock;
    static PAT: OnceLock<Option<Regex>> = OnceLock::new();
    let pat = PAT.get_or_init(|| {
        Regex::new(r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*['"`]([^'"`]+)['"`]"#).ok()
    });
    pat.as_ref().map_or_else(Vec::new, |re| {
        re.captures_iter(content)
            .filter_map(|c| c.get(2).map(|m| m.as_str().to_string()))
            .collect()
    })
}

/// Parse Rust test file for placeholder tests (#[test] #[ignore]).
fn find_rust_placeholders(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut saw_test_attr = false;
    let mut saw_ignore_attr = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "#[test]" {
            saw_test_attr = true;
            saw_ignore_attr = false;
            continue;
        }

        if saw_test_attr && (trimmed.starts_with("#[ignore") || trimmed.starts_with("#[ignore =")) {
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
