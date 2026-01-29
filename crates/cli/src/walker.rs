// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parallel file walking with gitignore support.
//!
//! Uses the `ignore` crate for efficient, parallel file discovery
//! that respects `.gitignore`, custom ignore patterns, and depth limits.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use crossbeam_channel::{Receiver, bounded};
use ignore::overrides::OverrideBuilder;
use ignore::{WalkBuilder, WalkState};

use crate::config::ExcludeConfig;
use crate::file_size::{self, FileSizeClass};

/// Helper to check if an ignore::Error is a symlink loop error.
fn is_loop_error(err: &ignore::Error) -> bool {
    match err {
        ignore::Error::Loop { .. } => true,
        ignore::Error::WithPath { err, .. } => is_loop_error(err),
        ignore::Error::WithDepth { err, .. } => is_loop_error(err),
        ignore::Error::WithLineNumber { err, .. } => is_loop_error(err),
        _ => false,
    }
}

/// Build a WalkedFile from a directory entry and metadata.
fn build_walked_file(
    entry: ignore::DirEntry,
    size: u64,
    meta: &Result<std::fs::Metadata, ignore::Error>,
) -> WalkedFile {
    let (mtime_secs, mtime_nanos) = meta
        .as_ref()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dur = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
            (dur.as_secs() as i64, dur.subsec_nanos())
        })
        .unwrap_or((0, 0));

    WalkedFile {
        depth: entry.depth(),
        path: entry.into_path(),
        size,
        mtime_secs,
        mtime_nanos,
        size_class: FileSizeClass::from_size(size),
    }
}

/// Default maximum directory depth.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Directories to skip entirely during walking.
/// These are filtered during traversal, not after discovery.
/// Skipping at the walker level prevents any I/O on these subtrees.
pub(crate) const SKIP_DIRECTORIES: &[&str] = &["node_modules", ".git"];

/// Walker configuration.
#[derive(Debug, Clone)]
pub struct WalkerConfig {
    /// Maximum directory depth (default: 100).
    pub max_depth: Option<usize>,

    /// Custom exclude patterns from config (walker-level: prevents I/O on subtrees).
    pub exclude_patterns: Vec<String>,

    /// Whether to respect gitignore files.
    pub git_ignore: bool,

    /// Whether to skip hidden files.
    pub hidden: bool,

    /// Number of threads (0 = auto).
    pub threads: usize,

    /// Minimum file count estimate for parallel walking (default: 1000).
    /// Below this threshold, single-threaded walking is used to avoid thread overhead.
    pub parallel_threshold: usize,

    /// Force parallel mode regardless of heuristic.
    pub force_parallel: bool,

    /// Force sequential mode regardless of heuristic.
    pub force_sequential: bool,
}

/// Default threshold for switching from sequential to parallel walking.
/// Based on benchmarks: parallel overhead exceeds benefits for <1000 files.
pub const DEFAULT_PARALLEL_THRESHOLD: usize = 1000;

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(DEFAULT_MAX_DEPTH),
            exclude_patterns: Vec::new(),
            git_ignore: true,
            hidden: true, // Skip hidden files by default
            threads: 0,   // Auto-detect
            parallel_threshold: DEFAULT_PARALLEL_THRESHOLD,
            force_parallel: false,
            force_sequential: false,
        }
    }
}

/// File discovered by the walker.
#[derive(Debug)]
pub struct WalkedFile {
    /// Path to the file.
    pub path: PathBuf,

    /// File size in bytes.
    pub size: u64,

    /// Modification time seconds since epoch.
    pub mtime_secs: i64,

    /// Modification time nanoseconds.
    pub mtime_nanos: u32,

    /// Directory depth from root.
    pub depth: usize,

    /// File size classification for processing hints.
    pub size_class: FileSizeClass,
}

/// Statistics from a walk operation.
#[derive(Debug, Default)]
pub struct WalkStats {
    /// Total files discovered.
    pub files_found: usize,

    /// Files skipped due to ignore patterns.
    pub files_ignored: usize,

    /// Files skipped due to size limit (>10MB).
    pub files_skipped_size: usize,

    /// Directories skipped due to depth limit.
    pub depth_limited: usize,

    /// Symlink loops detected.
    pub symlink_loops: usize,

    /// Errors encountered.
    pub errors: usize,
}

/// Parallel file walker with gitignore support.
pub struct FileWalker {
    config: WalkerConfig,
}

impl FileWalker {
    /// Check if a directory entry should be skipped entirely.
    /// Skipping prevents traversal of the entire subtree.
    #[inline]
    fn should_skip_dir(entry: &ignore::DirEntry) -> bool {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            return false;
        }

        entry
            .file_name()
            .to_str()
            .map(|name| SKIP_DIRECTORIES.contains(&name))
            .unwrap_or(false)
    }

    /// Create a new walker with the given configuration.
    pub fn new(config: WalkerConfig) -> Self {
        Self { config }
    }

    /// Create a walker from project exclude config.
    pub fn from_exclude_config(exclude: &ExcludeConfig) -> Self {
        Self::new(WalkerConfig {
            exclude_patterns: exclude.patterns.clone(),
            ..Default::default()
        })
    }

    /// Determine whether to use parallel walking based on heuristics.
    ///
    /// Uses a quick count of top-level directory entries as a proxy for total file count.
    /// For small codebases, single-threaded walking avoids thread pool overhead.
    fn should_use_parallel(&self, root: &Path) -> bool {
        // Force flags take precedence
        if self.config.force_parallel {
            return true;
        }
        if self.config.force_sequential {
            return false;
        }

        // Quick heuristic: count top-level entries.
        // If > threshold / 10, likely a large codebase worth parallelizing.
        // This avoids scanning the entire tree just to decide the walking strategy.
        let entry_count = std::fs::read_dir(root)
            .map(|entries| entries.count())
            .unwrap_or(0);

        entry_count >= self.config.parallel_threshold / 10
    }

    /// Walk the given root directory, returning a receiver of discovered files.
    ///
    /// Files are streamed through the channel as they're discovered.
    /// Returns (receiver, handle) where the handle can be joined to get stats.
    ///
    /// Automatically selects parallel or sequential walking based on heuristics:
    /// - For small directories (<1000 files estimated), uses sequential walking
    ///   to avoid thread pool initialization overhead.
    /// - For large directories, uses parallel walking for better throughput.
    pub fn walk(&self, root: &Path) -> (Receiver<WalkedFile>, WalkHandle) {
        let (tx, rx) = bounded(1000);

        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(self.config.hidden)
            .git_ignore(self.config.git_ignore)
            .git_exclude(true)
            .git_global(true)
            .follow_links(true); // Follow symlinks (ignore crate detects loops)

        if let Some(depth) = self.config.max_depth {
            builder.max_depth(Some(depth));
        }

        if self.config.threads > 0 {
            builder.threads(self.config.threads);
        }

        // Add custom exclude patterns using overrides
        // In ignore crate's override system:
        // - Without `!`: INCLUDE matching files (whitelist)
        // - With `!`: EXCLUDE matching files (blacklist)
        // To exclude files matching our patterns, we need `!` prefix
        if !self.config.exclude_patterns.is_empty() {
            let mut override_builder = OverrideBuilder::new(root);
            for pattern in &self.config.exclude_patterns {
                let _ = override_builder.add(&format!("!{}", pattern));
            }
            if let Ok(overrides) = override_builder.build() {
                builder.overrides(overrides);
            }
        }

        // Filter out common skip directories at the walker level.
        // This prevents any I/O on these subtrees for both parallel and sequential modes.
        builder.filter_entry(|entry| {
            !entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || !entry
                    .file_name()
                    .to_str()
                    .map(|name| SKIP_DIRECTORIES.contains(&name))
                    .unwrap_or(false)
        });

        let use_parallel = self.should_use_parallel(root);

        let handle = if use_parallel {
            Self::walk_parallel(builder, tx)
        } else {
            Self::walk_sequential(builder, tx)
        };

        (rx, handle)
    }

    /// Run parallel walker in a background thread.
    fn walk_parallel(
        builder: WalkBuilder,
        tx: crossbeam_channel::Sender<WalkedFile>,
    ) -> WalkHandle {
        let walker = builder.build_parallel();

        // Track stats atomically for parallel access
        let files_found = Arc::new(AtomicUsize::new(0));
        let files_skipped_size = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let symlink_loops = Arc::new(AtomicUsize::new(0));

        let stats_files = Arc::clone(&files_found);
        let stats_skipped = Arc::clone(&files_skipped_size);
        let stats_errors = Arc::clone(&errors);
        let stats_loops = Arc::clone(&symlink_loops);

        let handle = std::thread::spawn(move || {
            walker.run(|| {
                let tx = tx.clone();
                let files_found = Arc::clone(&stats_files);
                let files_skipped_size = Arc::clone(&stats_skipped);
                let errors = Arc::clone(&stats_errors);
                let symlink_loops = Arc::clone(&stats_loops);

                Box::new(move |entry| match entry {
                    Ok(entry) => {
                        // Skip configured directories entirely (e.g., node_modules, .git)
                        if Self::should_skip_dir(&entry) {
                            return WalkState::Skip;
                        }

                        let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);

                        if !is_file {
                            return WalkState::Continue;
                        }

                        let meta = entry.metadata();
                        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

                        // Skip files exceeding size limit (>10MB)
                        if size > file_size::MAX_FILE_SIZE {
                            tracing::warn!(
                                "skipping {} ({} > 10MB limit)",
                                entry.path().display(),
                                file_size::human_size(size, false)
                            );
                            files_skipped_size.fetch_add(1, Ordering::Relaxed);
                            return WalkState::Continue;
                        }

                        let walked = build_walked_file(entry, size, &meta);

                        files_found.fetch_add(1, Ordering::Relaxed);

                        if tx.send(walked).is_err() {
                            return WalkState::Quit;
                        }

                        WalkState::Continue
                    }
                    Err(err) => {
                        if is_loop_error(&err) {
                            tracing::warn!("Symlink loop detected: {}", err);
                            symlink_loops.fetch_add(1, Ordering::Relaxed);
                        } else {
                            tracing::warn!("Walk error: {}", err);
                            errors.fetch_add(1, Ordering::Relaxed);
                        }
                        WalkState::Continue
                    }
                })
            });

            WalkStats {
                files_found: stats_files.load(Ordering::Relaxed),
                files_skipped_size: stats_skipped.load(Ordering::Relaxed),
                errors: stats_errors.load(Ordering::Relaxed),
                symlink_loops: stats_loops.load(Ordering::Relaxed),
                ..Default::default()
            }
        });

        WalkHandle { handle }
    }

    /// Run sequential walker in a background thread.
    /// Avoids thread pool overhead for small directories.
    fn walk_sequential(
        builder: WalkBuilder,
        tx: crossbeam_channel::Sender<WalkedFile>,
    ) -> WalkHandle {
        let walker = builder.build();

        let handle = std::thread::spawn(move || {
            let mut files_found = 0usize;
            let mut files_skipped_size = 0usize;
            let mut errors = 0usize;
            let mut symlink_loops = 0usize;

            for entry in walker {
                match entry {
                    Ok(entry) => {
                        let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);

                        if !is_file {
                            continue;
                        }

                        let meta = entry.metadata();
                        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

                        // Skip files exceeding size limit (>10MB)
                        if size > file_size::MAX_FILE_SIZE {
                            tracing::warn!(
                                "skipping {} ({} > 10MB limit)",
                                entry.path().display(),
                                file_size::human_size(size, false)
                            );
                            files_skipped_size += 1;
                            continue;
                        }

                        let walked = build_walked_file(entry, size, &meta);

                        files_found += 1;

                        if tx.send(walked).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        if is_loop_error(&err) {
                            tracing::warn!("Symlink loop detected: {}", err);
                            symlink_loops += 1;
                        } else {
                            tracing::warn!("Walk error: {}", err);
                            errors += 1;
                        }
                    }
                }
            }

            WalkStats {
                files_found,
                files_skipped_size,
                errors,
                symlink_loops,
                ..Default::default()
            }
        });

        WalkHandle { handle }
    }

    /// Walk and collect all files (convenience method for small directories).
    pub fn walk_collect(&self, root: &Path) -> (Vec<WalkedFile>, WalkStats) {
        let (rx, handle) = self.walk(root);
        let files: Vec<_> = rx.iter().collect();
        let stats = handle.join();
        (files, stats)
    }
}

/// Handle to a running walk operation.
pub struct WalkHandle {
    handle: std::thread::JoinHandle<WalkStats>,
}

impl WalkHandle {
    /// Wait for the walk to complete and return stats.
    pub fn join(self) -> WalkStats {
        self.handle.join().unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "walker_tests.rs"]
mod tests;
