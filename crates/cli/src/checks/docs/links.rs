// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Markdown link validation.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::check::{CheckContext, Violation};

/// Regex pattern string for markdown links: [text](url)
/// Handles nested brackets in link text like `[[text]](url)`.
const LINK_PATTERN: &str = r"\[(?:[^\[\]]|\[[^\]]*\])*\]\(([^)]+)\)";

/// A markdown link extracted from content.
#[derive(Debug)]
pub(super) struct ExtractedLink {
    /// Line number (1-indexed) where the link appears.
    pub(super) line: u32,
    /// The URL/path from the link.
    pub(super) target: String,
}

/// Extract all markdown links from content, skipping links inside fenced code blocks.
pub(super) fn extract_links(content: &str) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    let Ok(pattern) = Regex::new(LINK_PATTERN) else {
        return links;
    };

    let mut in_fenced_block = false;

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx as u32 + 1;
        let trimmed = line.trim();

        // Check for fenced code block boundaries
        if trimmed.starts_with("```") {
            in_fenced_block = !in_fenced_block;
            continue;
        }

        // Skip lines inside fenced code blocks
        if in_fenced_block {
            continue;
        }

        for cap in pattern.captures_iter(line) {
            if let Some(target) = cap.get(1) {
                links.push(ExtractedLink {
                    line: line_num,
                    target: target.as_str().to_string(),
                });
            }
        }
    }
    links
}

/// Check if a link target is a local file path (not external URL).
pub(super) fn is_local_link(target: &str) -> bool {
    // Skip external URLs
    if target.starts_with("http://") || target.starts_with("https://") {
        return false;
    }
    // Skip mailto: links
    if target.starts_with("mailto:") {
        return false;
    }
    // Skip other protocols with ://
    if target.contains("://") {
        return false;
    }
    // Skip protocol-relative URLs (//example.com/)
    if target.starts_with("//") {
        return false;
    }
    // Skip fragment-only links (#section)
    if target.starts_with('#') {
        return false;
    }
    true
}

/// Strip fragment from link target.
pub(super) fn strip_fragment(target: &str) -> &str {
    target.split('#').next().unwrap_or(target)
}

/// Resolve a link target relative to the markdown file.
fn resolve_link(md_file: &Path, target: &str) -> std::path::PathBuf {
    let target = strip_fragment(target);

    // Normalize `.`/`./` prefix
    let normalized = if let Some(stripped) = target.strip_prefix("./") {
        stripped
    } else if target == "." {
        ""
    } else {
        target
    };

    // Resolve relative to markdown file's directory
    if let Some(parent) = md_file.parent() {
        parent.join(normalized)
    } else {
        std::path::PathBuf::from(normalized)
    }
}

/// Build a GlobSet from patterns.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Validate markdown links in all markdown files.
pub fn validate_links(ctx: &CheckContext, violations: &mut Vec<Violation>) {
    let config = &ctx.config.check.docs.links;

    // Check if link validation is disabled
    let check_level = config
        .check
        .as_deref()
        .or(ctx.config.check.docs.check.as_deref())
        .unwrap_or("error");
    if check_level == "off" {
        return;
    }

    // Build include/exclude matchers
    let include_set = build_glob_set(&config.include);
    let exclude_set = build_glob_set(&config.exclude);

    // Process each markdown file
    for walked in ctx.files {
        let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
        let path_str = relative_path.to_string_lossy();

        // Check include patterns
        if !include_set.is_match(&*path_str) {
            continue;
        }

        // Check exclude patterns
        if exclude_set.is_match(&*path_str) {
            continue;
        }

        // Read file content
        let content = match std::fs::read_to_string(&walked.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Extract and validate links
        validate_file_links(ctx, relative_path, &content, violations);
    }
}

/// Validate links within a single file.
fn validate_file_links(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    violations: &mut Vec<Violation>,
) {
    let links = extract_links(content);
    let abs_file = ctx.root.join(relative_path);

    for link in links {
        // Skip external links
        if !is_local_link(&link.target) {
            continue;
        }

        // Resolve and check existence
        let resolved = resolve_link(&abs_file, &link.target);
        if !resolved.exists() {
            violations.push(
                Violation::file(
                    relative_path,
                    link.line,
                    "broken_link",
                    "Linked file does not exist. Update the link or create the file.",
                )
                .with_pattern(strip_fragment(&link.target)),
            );
        }
    }
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
