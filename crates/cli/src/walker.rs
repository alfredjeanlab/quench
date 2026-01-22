//! Parallel file walking with gitignore support.
//!
//! Uses the `ignore` crate for efficient, parallel file discovery
//! that respects `.gitignore`, custom ignore patterns, and depth limits.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_channel::{Receiver, bounded};
use ignore::overrides::OverrideBuilder;
use ignore::{WalkBuilder, WalkState};

use crate::config::IgnoreConfig;

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

/// Default maximum directory depth.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Walker configuration.
#[derive(Debug, Clone)]
pub struct WalkerConfig {
    /// Maximum directory depth (default: 100).
    pub max_depth: Option<usize>,

    /// Custom ignore patterns from config.
    pub ignore_patterns: Vec<String>,

    /// Whether to respect gitignore files.
    pub git_ignore: bool,

    /// Whether to skip hidden files.
    pub hidden: bool,

    /// Number of threads (0 = auto).
    pub threads: usize,
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(DEFAULT_MAX_DEPTH),
            ignore_patterns: Vec::new(),
            git_ignore: true,
            hidden: true, // Skip hidden files by default
            threads: 0,   // Auto-detect
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

    /// Directory depth from root.
    pub depth: usize,
}

/// Statistics from a walk operation.
#[derive(Debug, Default)]
pub struct WalkStats {
    /// Total files discovered.
    pub files_found: usize,

    /// Files skipped due to ignore patterns.
    pub files_ignored: usize,

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
    /// Create a new walker with the given configuration.
    pub fn new(config: WalkerConfig) -> Self {
        Self { config }
    }

    /// Create a walker from project ignore config.
    pub fn from_ignore_config(ignore: &IgnoreConfig) -> Self {
        Self::new(WalkerConfig {
            ignore_patterns: ignore.patterns.clone(),
            ..Default::default()
        })
    }

    /// Walk the given root directory, returning a receiver of discovered files.
    ///
    /// Files are streamed through the channel as they're discovered.
    /// Returns (receiver, handle) where the handle can be joined to get stats.
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

        // Add custom ignore patterns using overrides
        // In ignore crate's override system:
        // - Without `!`: INCLUDE matching files (whitelist)
        // - With `!`: EXCLUDE matching files (blacklist)
        // To ignore files matching our patterns, we need `!` prefix
        if !self.config.ignore_patterns.is_empty() {
            let mut override_builder = OverrideBuilder::new(root);
            for pattern in &self.config.ignore_patterns {
                let _ = override_builder.add(&format!("!{}", pattern));
            }
            if let Ok(overrides) = override_builder.build() {
                builder.overrides(overrides);
            }
        }

        let walker = builder.build_parallel();

        // Track stats atomically
        let files_found = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let symlink_loops = Arc::new(AtomicUsize::new(0));

        let stats_files = Arc::clone(&files_found);
        let stats_errors = Arc::clone(&errors);
        let stats_loops = Arc::clone(&symlink_loops);

        // Run walker in background
        let handle = std::thread::spawn(move || {
            walker.run(|| {
                let tx = tx.clone();
                let files_found = Arc::clone(&stats_files);
                let errors = Arc::clone(&stats_errors);
                let symlink_loops = Arc::clone(&stats_loops);

                Box::new(move |entry| {
                    match entry {
                        Ok(entry) => {
                            // Skip directories
                            let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or(false);

                            if !is_file {
                                return WalkState::Continue;
                            }

                            // Get metadata for size
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                            let depth = entry.depth();
                            let walked = WalkedFile {
                                path: entry.into_path(),
                                size,
                                depth,
                            };

                            files_found.fetch_add(1, Ordering::Relaxed);

                            // Send through channel (blocking if full)
                            if tx.send(walked).is_err() {
                                return WalkState::Quit;
                            }

                            WalkState::Continue
                        }
                        Err(err) => {
                            // Check for symlink loop
                            if is_loop_error(&err) {
                                tracing::warn!("Symlink loop detected: {}", err);
                                symlink_loops.fetch_add(1, Ordering::Relaxed);
                            } else {
                                tracing::warn!("Walk error: {}", err);
                                errors.fetch_add(1, Ordering::Relaxed);
                            }
                            WalkState::Continue
                        }
                    }
                })
            });

            WalkStats {
                files_found: stats_files.load(Ordering::Relaxed),
                errors: stats_errors.load(Ordering::Relaxed),
                symlink_loops: stats_loops.load(Ordering::Relaxed),
                ..Default::default()
            }
        });

        (rx, WalkHandle { handle })
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
