// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Common pattern utilities shared between language adapters.

use std::path::Path;

use globset::GlobSet;

/// Standard exclude pattern checking with optional fast-path prefix optimization.
///
/// This provides a consistent implementation across all adapters:
/// 1. Fast-path check: If prefixes are provided, check first path component
/// 2. Fallback: Use GlobSet for full pattern matching
///
/// # Arguments
/// * `path` - The path to check
/// * `patterns` - Compiled GlobSet of exclude patterns
/// * `fast_prefixes` - Optional array of directory names for fast checking
pub fn check_exclude_patterns(
    path: &Path,
    patterns: &GlobSet,
    fast_prefixes: Option<&[&str]>,
) -> bool {
    // Fast path: check common directory prefixes
    if let Some(prefixes) = fast_prefixes
        && let Some(std::path::Component::Normal(name)) = path.components().next()
        && let Some(name_str) = name.to_str()
    {
        for prefix in prefixes {
            if name_str == *prefix {
                return true;
            }
        }
    }

    // Standard GlobSet matching
    patterns.is_match(path)
}

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
