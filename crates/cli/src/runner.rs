// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parallel check runner with error recovery and caching.
//!
//! Runs checks in parallel using rayon, isolating errors so one
//! check failure doesn't prevent other checks from running.
//! Supports file-level caching for faster iterative runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

use rayon::prelude::*;

use crate::cache::{CachedViolation, FileCache, FileCacheKey};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::Config;
use crate::walker::WalkedFile;

/// Cached violations for a file (Arc for O(1) clone).
type CachedViolationsArc = Arc<Vec<CachedViolation>>;

/// Configuration for the check runner.
pub struct RunnerConfig {
    /// Maximum violations before early termination (None = unlimited).
    pub limit: Option<usize>,
    /// Files changed since base ref (for --base flag).
    pub changed_files: Option<Vec<PathBuf>>,
    /// Whether to automatically fix violations when possible.
    pub fix: bool,
    /// Show what --fix would change without modifying files.
    pub dry_run: bool,
    /// Whether running in CI mode (enables slow checks like commit validation).
    pub ci_mode: bool,
    /// Base branch for commit comparison in CI mode.
    pub base_branch: Option<String>,
    /// Whether checking only staged changes (--staged flag).
    pub staged: bool,
}

/// The check runner executes multiple checks in parallel.
pub struct CheckRunner {
    config: RunnerConfig,
    cache: Option<Arc<FileCache>>,
}

impl CheckRunner {
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            config,
            cache: None,
        }
    }

    /// Enable caching for this runner.
    pub fn with_cache(mut self, cache: Arc<FileCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Run all provided checks and return results.
    ///
    /// Checks run in parallel. Errors are isolated - one check failing
    /// doesn't prevent other checks from running.
    ///
    /// If a cache is configured, files with valid cache entries will
    /// use cached violations instead of re-running checks.
    pub fn run(
        &self,
        checks: Vec<Arc<dyn Check>>,
        files: &[WalkedFile],
        config: &Config,
        root: &Path,
    ) -> Vec<CheckResult> {
        // If no cache, run checks normally
        let Some(cache) = &self.cache else {
            return self.run_uncached(checks, files, config, root);
        };

        // Separate files into cached and uncached
        // Pre-size for expected distribution (optimized for warm cache case).
        // Cold runs will reallocate, but that's acceptable as they're infrequent
        // (~4 reallocations worst case, negligible vs check work).
        let file_count = files.len();
        let mut cached_violations: HashMap<PathBuf, CachedViolationsArc> =
            HashMap::with_capacity(file_count);
        // Expect ~10% cache miss on warm runs. Cold runs will reallocate.
        let mut uncached_files: Vec<&WalkedFile> = Vec::with_capacity(file_count / 10 + 1);

        for file in files {
            let key = FileCacheKey::from_walked_file(file);
            if let Some(violations) = cache.lookup(&file.path, &key) {
                // Arc clone is O(1) - just increments refcount
                cached_violations.insert(file.path.clone(), violations);
            } else {
                uncached_files.push(file);
            }
        }

        // Build owned files for uncached (needed for CheckContext)
        // Note: We need owned WalkedFiles for the context, so we clone
        let uncached_owned: Vec<WalkedFile> = uncached_files
            .iter()
            .map(|f| WalkedFile {
                path: f.path.clone(),
                size: f.size,
                mtime_secs: f.mtime_secs,
                mtime_nanos: f.mtime_nanos,
                depth: f.depth,
                size_class: f.size_class,
            })
            .collect();

        let violation_count = AtomicUsize::new(0);

        // Run checks on uncached files
        let results: Vec<CheckResult> = checks
            .into_par_iter()
            .map(|check| {
                let check_name = check.name();

                // Get cached violations for this check
                let cached_for_check: Vec<Violation> = cached_violations
                    .iter()
                    .flat_map(|(path, violations)| {
                        violations
                            .iter()
                            .filter(|v| v.check == check_name)
                            .map(|v| {
                                // Convert to relative path for display
                                let display_path = path.strip_prefix(root).unwrap_or(path);
                                v.to_violation(display_path.to_path_buf())
                            })
                    })
                    .collect();

                let ctx = CheckContext {
                    root,
                    files: &uncached_owned,
                    config,
                    limit: self.config.limit,
                    violation_count: &violation_count,
                    changed_files: self.config.changed_files.as_deref(),
                    fix: self.config.fix,
                    dry_run: self.config.dry_run,
                    ci_mode: self.config.ci_mode,
                    base_branch: self.config.base_branch.as_deref(),
                    staged: self.config.staged,
                };

                // Run check on uncached files with timing
                let check_start = Instant::now();
                let mut result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || check.run(&ctx),
                )) {
                    Ok(result) => result,
                    Err(_) => CheckResult::skipped(
                        check_name,
                        "Internal error: check panicked".to_string(),
                    ),
                };
                result.duration_ms = Some(check_start.elapsed().as_millis() as u64);

                // Merge cached violations into result
                if cached_for_check.is_empty() {
                    result
                } else {
                    let mut all_violations = cached_for_check;
                    all_violations.extend(result.violations);

                    // Sort violations by file path for consistent output
                    all_violations.sort_by(|a, b| {
                        a.file
                            .as_deref()
                            .cmp(&b.file.as_deref())
                            .then_with(|| a.line.cmp(&b.line))
                    });

                    let passed = all_violations.is_empty() && !result.skipped;
                    CheckResult {
                        name: result.name,
                        passed,
                        skipped: result.skipped,
                        stub: result.stub,
                        fixed: result.fixed,
                        error: result.error,
                        violations: all_violations,
                        fix_summary: result.fix_summary,
                        metrics: result.metrics,
                        by_package: result.by_package,
                        duration_ms: result.duration_ms,
                    }
                }
            })
            .collect();

        // Update cache with violations from newly processed files
        // Group violations by file path
        // Pre-size for uncached file count (Phase 3 optimization)
        let mut violations_by_file: HashMap<PathBuf, Vec<CachedViolation>> =
            HashMap::with_capacity(uncached_files.len());

        // Build a set for O(1) lookup instead of O(n) linear search
        let processed_paths: std::collections::HashSet<&Path> =
            uncached_files.iter().map(|f| f.path.as_path()).collect();

        for result in &results {
            for violation in &result.violations {
                if let Some(file_path) = &violation.file {
                    // Only cache violations from files we just processed
                    let abs_path = if file_path.is_absolute() {
                        file_path.clone()
                    } else {
                        root.join(file_path)
                    };

                    // Check if this file was in uncached_files (O(1) lookup)
                    if processed_paths.contains(abs_path.as_path()) {
                        violations_by_file
                            .entry(abs_path)
                            .or_default()
                            .push(CachedViolation::from_violation(violation, &result.name));
                    }
                }
            }
        }

        // Insert all processed files into cache (including those with no violations)
        for file in &uncached_files {
            let key = FileCacheKey::from_walked_file(file);
            let violations = violations_by_file.remove(&file.path).unwrap_or_default();
            cache.insert(file.path.clone(), key, violations);
        }

        // Sort results by canonical check order for consistent output
        let mut sorted = results;
        sorted.sort_by_key(|r| {
            crate::checks::CHECK_NAMES
                .iter()
                .position(|&n| n == r.name)
                .unwrap_or(usize::MAX)
        });

        sorted
    }

    /// Run checks without caching.
    fn run_uncached(
        &self,
        checks: Vec<Arc<dyn Check>>,
        files: &[WalkedFile],
        config: &Config,
        root: &Path,
    ) -> Vec<CheckResult> {
        let violation_count = AtomicUsize::new(0);

        // Run checks in parallel
        let results: Vec<CheckResult> = checks
            .into_par_iter()
            .map(|check| {
                let ctx = CheckContext {
                    root,
                    files,
                    config,
                    limit: self.config.limit,
                    violation_count: &violation_count,
                    changed_files: self.config.changed_files.as_deref(),
                    fix: self.config.fix,
                    dry_run: self.config.dry_run,
                    ci_mode: self.config.ci_mode,
                    base_branch: self.config.base_branch.as_deref(),
                    staged: self.config.staged,
                };

                // Catch panics to ensure error isolation, with timing
                let check_start = Instant::now();
                let mut result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || check.run(&ctx),
                )) {
                    Ok(result) => result,
                    Err(_) => {
                        // Check panicked - return skipped result
                        CheckResult::skipped(
                            check.name(),
                            "Internal error: check panicked".to_string(),
                        )
                    }
                };
                result.duration_ms = Some(check_start.elapsed().as_millis() as u64);
                result
            })
            .collect();

        // Sort results by canonical check order for consistent output
        let mut sorted = results;
        sorted.sort_by_key(|r| {
            crate::checks::CHECK_NAMES
                .iter()
                .position(|&n| n == r.name)
                .unwrap_or(usize::MAX)
        });

        sorted
    }

    /// Check if early termination is needed based on violation count.
    pub fn should_terminate(&self, violation_count: usize) -> bool {
        if let Some(limit) = self.config.limit {
            violation_count >= limit
        } else {
            false
        }
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
