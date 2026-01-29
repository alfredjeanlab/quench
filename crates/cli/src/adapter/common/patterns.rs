// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Common pattern utilities shared between language adapters.

/// Normalize exclude patterns to glob patterns.
///
/// Converts user-friendly directory patterns to proper glob patterns:
/// - `dir/` → `dir/**` (trailing slash means "everything in this directory")
/// - `dir` → `dir/**` (bare directory name without wildcards)
/// - `dir/**` → `dir/**` (already a glob pattern, kept as-is)
///
/// # Examples
///
/// ```ignore
/// let patterns = vec!["vendor/".to_string(), "build".to_string(), "**/*.pyc".to_string()];
/// let normalized = normalize_exclude_patterns(&patterns);
/// assert_eq!(normalized, vec!["vendor/**", "build/**", "**/*.pyc"]);
/// ```
pub fn normalize_exclude_patterns(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|p| {
            if p.ends_with('/') {
                format!("{}**", p)
            } else if !p.contains('*') {
                format!("{}/**", p.trim_end_matches('/'))
            } else {
                p.clone()
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
