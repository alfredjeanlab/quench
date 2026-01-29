// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File change classification by glob patterns.
//!
//! Determines whether each changed file is a source file, test file,
//! or excluded, using configurable glob patterns.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use super::super::diff::{ChangeType, FileChange};
use super::CorrelationConfig;

#[cfg(test)]
#[path = "classify_tests.rs"]
mod tests;

/// Threshold for switching to parallel file classification.
/// Below this, sequential iteration is faster due to rayon overhead.
const PARALLEL_THRESHOLD: usize = 50;

/// Cached GlobSets for common pattern configurations.
#[derive(Clone)]
pub(super) struct CompiledPatterns {
    test_patterns: GlobSet,
    source_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl CompiledPatterns {
    pub(super) fn from_config(config: &CorrelationConfig) -> Result<Self, String> {
        Ok(Self {
            test_patterns: build_glob_set(&config.test_patterns)?,
            source_patterns: build_glob_set(&config.source_patterns)?,
            exclude_patterns: build_glob_set(&config.exclude_patterns)?,
        })
    }

    pub(super) fn empty() -> Self {
        Self {
            test_patterns: GlobSet::empty(),
            source_patterns: GlobSet::empty(),
            exclude_patterns: GlobSet::empty(),
        }
    }
}

/// Get cached patterns for the default configuration.
pub(super) fn default_patterns() -> &'static CompiledPatterns {
    static PATTERNS: OnceLock<CompiledPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        // Default patterns are hardcoded and known to be valid, but we handle
        // the error case defensively by returning empty patterns.
        CompiledPatterns::from_config(&CorrelationConfig::default())
            .unwrap_or_else(|_| CompiledPatterns::empty())
    })
}

/// Classify changes into source and test files.
///
/// Uses parallel processing for large change sets (>= PARALLEL_THRESHOLD files).
pub(super) fn classify_changes<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    if changes.len() >= PARALLEL_THRESHOLD {
        classify_changes_parallel(changes, patterns, root)
    } else {
        classify_changes_sequential(changes, patterns, root)
    }
}

/// Sequential classification for small change sets.
fn classify_changes_sequential<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    let mut source_changes: Vec<&FileChange> = Vec::new();
    let mut test_changes: Vec<PathBuf> = Vec::new();

    for change in changes {
        // Skip deleted files - they don't require tests
        if change.change_type == ChangeType::Deleted {
            continue;
        }

        // Get relative path for pattern matching
        let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);

        if patterns.test_patterns.is_match(rel_path) {
            test_changes.push(rel_path.to_path_buf());
        } else if patterns.source_patterns.is_match(rel_path)
            && !patterns.exclude_patterns.is_match(rel_path)
        {
            source_changes.push(change);
        }
    }

    (source_changes, test_changes)
}

/// Parallel classification for large change sets.
fn classify_changes_parallel<'a>(
    changes: &'a [FileChange],
    patterns: &CompiledPatterns,
    root: &Path,
) -> (Vec<&'a FileChange>, Vec<PathBuf>) {
    // Use rayon to classify in parallel
    let classified: Vec<_> = changes
        .par_iter()
        .filter(|c| c.change_type != ChangeType::Deleted)
        .filter_map(|change| {
            let rel_path = change.path.strip_prefix(root).unwrap_or(&change.path);

            if patterns.test_patterns.is_match(rel_path) {
                Some((None, Some(rel_path.to_path_buf())))
            } else if patterns.source_patterns.is_match(rel_path)
                && !patterns.exclude_patterns.is_match(rel_path)
            {
                Some((Some(change), None))
            } else {
                None
            }
        })
        .collect();

    // Separate into source and test changes
    let mut source_changes = Vec::new();
    let mut test_changes = Vec::new();

    for (source, test) in classified {
        if let Some(s) = source {
            source_changes.push(s);
        }
        if let Some(t) = test {
            test_changes.push(t);
        }
    }

    (source_changes, test_changes)
}

/// Build a GlobSet from pattern strings.
pub(super) fn build_glob_set(patterns: &[String]) -> Result<GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| e.to_string())?;
        builder.add(glob);
    }
    builder.build().map_err(|e| e.to_string())
}
