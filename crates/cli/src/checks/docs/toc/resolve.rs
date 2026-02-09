// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! TOC entry resolution.
//!
//! Resolves TOC paths using multiple strategies.

use std::path::Path;

use globset::Glob;

use super::parse::{TreeEntry, normalize_dot_prefix};

/// Resolution strategy for TOC entries.
#[derive(Debug, Clone, Copy)]
pub(super) enum ResolutionStrategy {
    /// Relative to the markdown file's directory (`.`/`./` treated as current directory)
    RelativeToFile,
    /// Relative to project root
    RelativeToRoot,
    /// Strip markdown file's parent directory name prefix
    StripParentDirName,
}

impl ResolutionStrategy {
    pub(super) fn description(&self) -> &'static str {
        match self {
            Self::RelativeToFile => "relative to markdown file",
            Self::RelativeToRoot => "relative to project root",
            Self::StripParentDirName => "stripping parent directory prefix",
        }
    }
}

/// Check if a path contains glob wildcards.
pub(super) fn is_glob_pattern(path: &str) -> bool {
    path.contains('*')
}

/// Try to resolve a glob pattern by finding any matching file.
/// Uses the `ignore` crate for fast parallel directory walking.
fn try_resolve_glob(base: &Path, pattern: &str) -> bool {
    let Ok(glob) = Glob::new(pattern) else {
        return false;
    };
    let matcher = glob.compile_matcher();

    // Use ignore crate's WalkBuilder for fast traversal
    let walker = ignore::WalkBuilder::new(base).max_depth(Some(10)).build();

    for entry in walker.flatten() {
        let path = entry.path();
        // Get path relative to base for matching
        if let Ok(relative) = path.strip_prefix(base)
            && matcher.is_match(relative)
        {
            return true;
        }
    }
    false
}

/// Normalize a TOC path for cross-platform compatibility.
///
/// - Converts Windows path separators (backslash to forward slash)
/// - Strips trailing slashes
/// - Decodes URL-encoded characters (e.g., %20 to space)
fn normalize_toc_path(path: &str) -> String {
    // Convert Windows separators to Unix
    let path = path.replace('\\', "/");

    // Strip trailing slash
    let path = path.trim_end_matches('/');

    // Decode URL-encoded characters
    percent_encoding::percent_decode_str(path).decode_utf8_lossy().into_owned()
}

/// Try to resolve a path using a specific strategy.
pub(super) fn try_resolve(
    root: &Path,
    md_file: &Path,
    entry_path: &str,
    strategy: ResolutionStrategy,
) -> bool {
    // Normalize the path (separators, trailing slashes, URL encoding)
    let normalized_owned = normalize_toc_path(entry_path);
    // Then normalize `.`/`./` prefix for all strategies
    let normalized = normalize_dot_prefix(&normalized_owned);

    // Handle glob patterns
    if is_glob_pattern(normalized) {
        return match strategy {
            ResolutionStrategy::RelativeToFile => {
                if let Some(parent) = md_file.parent() {
                    try_resolve_glob(parent, normalized)
                } else {
                    false
                }
            }
            ResolutionStrategy::RelativeToRoot => try_resolve_glob(root, normalized),
            ResolutionStrategy::StripParentDirName => {
                if let Some(parent) = md_file.parent()
                    && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
                {
                    let prefix = format!("{}/", parent_name);
                    if let Some(stripped) = normalized.strip_prefix(&prefix) {
                        return try_resolve_glob(parent, stripped);
                    }
                }
                false
            }
        };
    }

    match strategy {
        ResolutionStrategy::RelativeToFile => {
            if let Some(parent) = md_file.parent() {
                parent.join(normalized).exists()
            } else {
                false
            }
        }
        ResolutionStrategy::RelativeToRoot => root.join(normalized).exists(),
        ResolutionStrategy::StripParentDirName => {
            // Get the parent directory name of the markdown file
            if let Some(parent) = md_file.parent()
                && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
            {
                // Try stripping the parent dir name prefix
                // e.g., "quality/foo.sh" in checks/quality/README.md → checks/quality/foo.sh
                let prefix = format!("{}/", parent_name);
                if let Some(stripped) = normalized.strip_prefix(&prefix) {
                    return parent.join(stripped).exists();
                }
            }
            false
        }
    }
}

/// Try all resolution strategies for a block of entries.
/// Returns None if a strategy resolves all entries, or Some with unresolved entries.
pub(super) fn try_resolve_block<'a>(
    root: &Path,
    md_file: &Path,
    entries: &'a [TreeEntry],
    strategy: ResolutionStrategy,
) -> Option<Vec<&'a TreeEntry>> {
    let unresolved: Vec<_> = entries
        .iter()
        .filter(|e| !e.is_dir && !try_resolve(root, md_file, &e.path, strategy))
        .collect();

    if unresolved.is_empty() { None } else { Some(unresolved) }
}
