// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Markdown link validation.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::check::{CheckContext, Violation};

/// Regex pattern string for markdown links: [text](url)
/// Handles nested brackets in link text like `[[text]](url)`.
const LINK_PATTERN: &str = r"\[(?:[^\[\]]|\[[^\]]*\])*\]\(([^)]+)\)";

/// Pre-compiled regex for markdown link extraction.
#[allow(clippy::expect_used)]
static LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(LINK_PATTERN).expect("valid regex pattern"));

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

        // Use pre-compiled regex
        for cap in LINK_REGEX.captures_iter(line) {
            if let Some(target) = cap.get(1) {
                links.push(ExtractedLink { line: line_num, target: target.as_str().to_string() });
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
pub(super) fn resolve_link(md_file: &Path, target: &str) -> std::path::PathBuf {
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

/// Validate markdown links in all markdown files (parallel version).
pub fn validate_links_parallel(
    ctx: &CheckContext,
    path_cache: &super::PathCache,
) -> Vec<Violation> {
    let config = &ctx.config.check.docs.links;

    // Check if link validation is disabled
    if !super::is_check_enabled(config.check.as_deref(), ctx.config.check.docs.check.as_deref()) {
        return Vec::new();
    }

    super::process_markdown_files_parallel(
        ctx,
        &config.include,
        &config.exclude,
        path_cache,
        validate_file_links_cached,
    )
}

/// Validate links within a single file (uses path cache).
fn validate_file_links_cached(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    path_cache: &super::PathCache,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let links = extract_links(content);
    let abs_file = ctx.root.join(relative_path);

    for link in links {
        // Skip external links
        if !is_local_link(&link.target) {
            continue;
        }

        // Resolve and check existence using cache
        let resolved = resolve_link(&abs_file, &link.target);
        if !path_cache.exists(&resolved) {
            violations.push(
                Violation::file(
                    relative_path,
                    link.line,
                    "broken_link",
                    "Linked file does not exist. Update the link or create the file.",
                )
                .with_target(strip_fragment(&link.target)),
            );
        }
    }
    violations
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
