// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Documentation validation check.
//!
//! Validates:
//! - TOC entries reference existing files
//! - Markdown links point to existing files
//! - Specs have required sections
//! - Feature commits have documentation (CI mode)

mod commit;
mod content;
mod links;
mod specs;
mod toc;

use std::path::{Path, PathBuf};

use dashmap::DashMap;
use rayon::prelude::*;

use crate::adapter::build_glob_set;
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::file_reader::FileContent;

/// Per-run cache for path existence checks.
///
/// Shared across all docs sub-checks to avoid redundant filesystem calls.
pub(super) struct PathCache {
    /// Maps paths to existence result.
    exists: DashMap<PathBuf, bool>,
}

impl PathCache {
    pub fn new() -> Self {
        Self { exists: DashMap::new() }
    }

    /// Check if a path exists, using cache.
    pub fn exists(&self, path: &Path) -> bool {
        // Use path as-is for cache key (canonicalization is expensive)
        if let Some(result) = self.exists.get(path) {
            return *result;
        }
        let result = path.exists();
        self.exists.insert(path.to_path_buf(), result);
        result
    }

    /// Pre-populate cache with known existing files.
    pub fn populate(&self, files: &[&crate::walker::WalkedFile]) {
        for file in files {
            self.exists.insert(file.path.clone(), true);
        }
    }
}

/// Check if a docs subcheck is enabled.
///
/// Returns `true` if the check should run, `false` if disabled.
/// Checks subcheck config first, then falls back to parent config.
pub(super) fn is_check_enabled(subcheck: Option<&str>, parent: Option<&str>) -> bool {
    matches!(subcheck.or(parent).unwrap_or("error"), "error" | "warn")
}

/// Process markdown files matching include/exclude patterns in parallel.
pub(super) fn process_markdown_files_parallel<F>(
    ctx: &CheckContext,
    include: &[String],
    exclude: &[String],
    path_cache: &PathCache,
    validator: F,
) -> Vec<Violation>
where
    F: Fn(&CheckContext, &Path, &str, &PathCache) -> Vec<Violation> + Sync,
{
    let include_set = build_glob_set(include);
    let exclude_set = build_glob_set(exclude);

    // Collect matching files first (fast filter pass)
    let matching_files: Vec<_> = ctx
        .files
        .iter()
        .filter(|walked| {
            let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
            let path_str = relative_path.to_string_lossy();
            include_set.is_match(&*path_str) && !exclude_set.is_match(&*path_str)
        })
        .collect();

    // Process in parallel
    matching_files
        .par_iter()
        .flat_map(|walked| {
            let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);

            // Read file content (uses mmap for large files per performance spec)
            let file_content = match FileContent::read(&walked.path) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            let Some(content) = file_content.as_str() else {
                return Vec::new(); // Skip non-UTF-8 files
            };

            validator(ctx, relative_path, content, path_cache)
        })
        .collect()
}

pub struct DocsCheck;

impl Check for DocsCheck {
    fn name(&self) -> &'static str {
        "docs"
    }

    fn description(&self) -> &'static str {
        "Documentation validation"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let mut violations = Vec::new();

        // Check if docs check is disabled
        if !is_check_enabled(None, ctx.config.check.docs.check.as_deref()) {
            return CheckResult::passed("docs");
        }

        // Create shared path cache and pre-populate with known files
        let path_cache = PathCache::new();
        let file_refs: Vec<_> = ctx.files.iter().collect();
        path_cache.populate(&file_refs);

        // Run TOC validation (parallel)
        violations.extend(toc::validate_toc_parallel(ctx, &path_cache));

        // Run link validation (parallel)
        violations.extend(links::validate_links_parallel(ctx, &path_cache));

        // Run specs validation (uses path cache internally)
        specs::validate_specs(ctx, &mut violations, &path_cache);

        // Run commit validation (CI mode only)
        if ctx.ci_mode {
            commit::validate_commit_docs(ctx, &mut violations);
        }

        // Respect violation limit
        if let Some(limit) = ctx.limit {
            violations.truncate(limit);
        }

        // Collect metrics for JSON output
        let metrics =
            specs::collect_metrics(ctx).map(|m| serde_json::to_value(m).unwrap_or_default());

        let result = if violations.is_empty() {
            CheckResult::passed("docs")
        } else {
            CheckResult::failed("docs", violations)
        };

        if let Some(m) = metrics { result.with_metrics(m) } else { result }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
