// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File-level caching for check results.
//!
//! Caches check violations per file using mtime+size as cache key.
//! Provides 10x speedup on iterative runs where few files change.

use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::SystemTime;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::check::Violation;

/// Cache version for invalidation on format changes.
/// Incremented when check logic changes (e.g., counting nonblank vs all lines).
/// v7: Migrated from bincode to postcard serialization.
/// v8: Fixed cfg(test) detection false positives on comments.
/// v11: TOC validator skips box diagrams (blocks with top corner characters).
/// v12: StripParentDirName resolves relative to parent dir, not root.
/// v13: Added content validation for spec files (sections, tables, size limits).
/// v14: Added source-based area detection for docs commit checking.
/// v17: Removed WorkspaceConfig, consolidated packages into ProjectConfig.
/// v18: Added target_path for docs cache.
/// v19: Added per-language cloc check levels (warn vs error).
/// v20: Added placeholders check for detecting placeholder tests.
/// v21: Added change_type and lines_changed fields to missing_tests violations.
/// v22: Added agent documentation check to git check.
/// v23: Added skip_merge option to git check (merge commits now skipped by default).
/// v24: Test pattern consolidation - hash language-specific patterns for file classification.
/// v25: Removed standalone placeholders check, integrated metrics into tests check.
/// v26: Added CI mode threshold checking (coverage and time violations).
/// v27: Added license check --fix functionality.
/// v28: Added Python coverage collection and parsing.
pub const CACHE_VERSION: u32 = 28;

/// Cache file name within .quench directory.
pub const CACHE_FILE_NAME: &str = "cache.bin";

/// Error type for cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Postcard(#[from] postcard::Error),

    /// Cache version mismatch.
    #[error("cache version mismatch")]
    VersionMismatch,

    /// Quench version mismatch.
    #[error("quench version changed")]
    QuenchVersionMismatch,

    /// Config hash changed.
    #[error("config changed")]
    ConfigChanged,
}

/// Metadata used as cache key for a single file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileCacheKey {
    /// Modification time seconds since epoch.
    pub mtime_secs: i64,
    /// Modification time nanoseconds.
    pub mtime_nanos: u32,
    /// File size in bytes.
    pub size: u64,
}

impl FileCacheKey {
    /// Create cache key from file metadata.
    pub fn from_metadata(meta: &Metadata) -> Self {
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let duration = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        Self {
            mtime_secs: duration.as_secs() as i64,
            mtime_nanos: duration.subsec_nanos(),
            size: meta.len(),
        }
    }

    /// Create cache key from a WalkedFile.
    pub fn from_walked_file(file: &crate::walker::WalkedFile) -> Self {
        Self {
            mtime_secs: file.mtime_secs,
            mtime_nanos: file.mtime_nanos,
            size: file.size,
        }
    }
}

/// Cached result for a single file (runtime representation with Arc).
#[derive(Debug, Clone)]
pub struct CachedFileResult {
    /// Cache key when this result was computed.
    pub key: FileCacheKey,
    /// Violations found in this file (across all checks).
    /// Uses Arc for O(1) clone on cache hits instead of O(n) deep clone.
    pub violations: Arc<Vec<CachedViolation>>,
}

/// Cached result for a single file (serialization format).
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SerializedFileResult {
    pub(crate) key: FileCacheKey,
    pub(crate) violations: Vec<CachedViolation>,
}

/// Minimal violation data for cache storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedViolation {
    /// Check that produced this violation.
    pub check: String,
    /// Line number (if applicable).
    pub line: Option<u32>,
    /// Violation type/category.
    pub violation_type: String,
    /// Advice message.
    pub advice: String,
    /// Current value (for threshold violations).
    pub value: Option<i64>,
    /// Threshold that was exceeded.
    pub threshold: Option<i64>,
    /// Pattern name that matched (for escape violations).
    pub pattern: Option<String>,
    /// Total line count (for cloc violations).
    pub lines: Option<i64>,
    /// Non-blank line count (for cloc violations).
    pub nonblank: Option<i64>,
    /// Path in TOC/link that was broken (for docs violations).
    pub target_path: Option<String>,
}

impl CachedViolation {
    /// Create from a Violation and check name.
    pub fn from_violation(v: &Violation, check: &str) -> Self {
        Self {
            check: check.to_string(),
            line: v.line,
            violation_type: v.violation_type.clone(),
            advice: v.advice.clone(),
            value: v.value,
            threshold: v.threshold,
            pattern: v.pattern.clone(),
            lines: v.lines,
            nonblank: v.nonblank,
            target_path: v.target.clone().or_else(|| v.path.clone()),
        }
    }

    /// Convert back to Violation with file path.
    pub fn to_violation(&self, file: PathBuf) -> Violation {
        Violation {
            file: Some(file),
            line: self.line,
            violation_type: self.violation_type.clone(),
            advice: self.advice.clone(),
            value: self.value,
            threshold: self.threshold,
            pattern: self.pattern.clone(),
            lines: self.lines,
            nonblank: self.nonblank,
            other_file: None,
            section: None,
            commit: None,
            message: None,
            expected_docs: None,
            area: None,
            area_match: None,
            path: None,
            target: self.target_path.clone(),
            change_type: None,
            lines_changed: None,
            scope: None,
            expected: None,
            found: None,
        }
    }
}

/// Persistent cache structure for serialization.
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentCache {
    /// Cache format version.
    pub version: u32,
    /// Quench version that created this cache.
    pub quench_version: String,
    /// Hash of config that affects check results.
    pub config_hash: u64,
    /// Per-file cached results (serialized without Arc).
    pub(crate) files: HashMap<PathBuf, SerializedFileResult>,
}

/// Runtime cache wrapper with thread-safe access.
pub struct FileCache {
    /// Concurrent hashmap of cached results.
    inner: DashMap<PathBuf, CachedFileResult>,
    /// Config hash for invalidation.
    config_hash: u64,
    /// Quench version.
    quench_version: String,
    /// Cache hit count.
    hits: AtomicUsize,
    /// Cache miss count.
    misses: AtomicUsize,
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: usize,
    /// Number of cache misses.
    pub misses: usize,
    /// Number of entries in cache.
    pub entries: usize,
}

impl FileCache {
    /// Create a new empty cache.
    pub fn new(config_hash: u64) -> Self {
        Self {
            inner: DashMap::new(),
            config_hash,
            quench_version: env!("CARGO_PKG_VERSION").to_string(),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    /// Load cache from disk.
    pub fn from_persistent(path: &Path, config_hash: u64) -> Result<Self, CacheError> {
        let bytes = std::fs::read(path)?;
        let cache: PersistentCache = postcard::from_bytes(&bytes)?;

        // Validate version
        if cache.version != CACHE_VERSION {
            return Err(CacheError::VersionMismatch);
        }
        if cache.quench_version != env!("CARGO_PKG_VERSION") {
            return Err(CacheError::QuenchVersionMismatch);
        }
        if cache.config_hash != config_hash {
            return Err(CacheError::ConfigChanged);
        }

        // Convert serialized format to runtime format (wrap violations in Arc)
        let inner: DashMap<PathBuf, CachedFileResult> = cache
            .files
            .into_iter()
            .map(|(path, result)| {
                (
                    path,
                    CachedFileResult {
                        key: result.key,
                        violations: Arc::new(result.violations),
                    },
                )
            })
            .collect();

        Ok(Self {
            inner,
            config_hash,
            quench_version: cache.quench_version,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        })
    }

    /// Look up cached violations for a file.
    ///
    /// Returns Some if the file has a valid cache entry (matching mtime+size).
    /// Returns None on cache miss.
    ///
    /// The returned Arc allows O(1) clone instead of O(n) deep clone of violations.
    pub fn lookup(&self, path: &Path, key: &FileCacheKey) -> Option<Arc<Vec<CachedViolation>>> {
        if let Some(entry) = self.inner.get(path)
            && entry.key == *key
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(Arc::clone(&entry.violations));
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert or update a file's cached result.
    pub fn insert(&self, path: PathBuf, key: FileCacheKey, violations: Vec<CachedViolation>) {
        self.inner.insert(
            path,
            CachedFileResult {
                key,
                violations: Arc::new(violations),
            },
        );
    }

    /// Persist cache to disk.
    pub fn persist(&self, path: &Path) -> Result<(), CacheError> {
        let cache = PersistentCache {
            version: CACHE_VERSION,
            quench_version: self.quench_version.clone(),
            config_hash: self.config_hash,
            // Convert runtime format to serialized format (extract from Arc)
            files: self
                .inner
                .iter()
                .map(|e| {
                    (
                        e.key().clone(),
                        SerializedFileResult {
                            key: e.value().key.clone(),
                            violations: (*e.value().violations).clone(),
                        },
                    )
                })
                .collect(),
        };

        // Write atomically via temp file
        let temp_path = path.with_extension("tmp");
        let bytes = postcard::to_allocvec(&cache)?;
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
    }

    /// Persist cache to disk asynchronously.
    ///
    /// Returns a join handle that can be waited on, or ignored if caller
    /// doesn't care about completion. The cache data is cloned before
    /// spawning the thread to avoid holding locks.
    ///
    /// # Example
    /// ```
    /// # use std::path::PathBuf;
    /// # use tempfile::tempdir;
    /// # use quench::cache::FileCache;
    /// # let dir = tempdir().unwrap();
    /// # let cache_path = dir.path().join("cache.bin");
    /// # let cache = FileCache::new(0);
    /// // Fire and forget - cache write happens in background
    /// let handle = cache.persist_async(cache_path);
    /// // Wait for completion (optional - process can exit without waiting)
    /// handle.join().unwrap().unwrap();
    /// ```
    pub fn persist_async(&self, path: PathBuf) -> JoinHandle<Result<(), CacheError>> {
        // Clone data for the background thread
        let cache = PersistentCache {
            version: CACHE_VERSION,
            quench_version: self.quench_version.clone(),
            config_hash: self.config_hash,
            files: self
                .inner
                .iter()
                .map(|e| {
                    (
                        e.key().clone(),
                        SerializedFileResult {
                            key: e.value().key.clone(),
                            violations: (*e.value().violations).clone(),
                        },
                    )
                })
                .collect(),
        };

        std::thread::spawn(move || {
            let temp_path = path.with_extension("tmp");
            let bytes = postcard::to_allocvec(&cache)?;

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&temp_path, &bytes)?;
            std::fs::rename(&temp_path, &path)?;
            Ok(())
        })
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            entries: self.inner.len(),
        }
    }
}

/// Compute a hash of config fields that affect check results.
pub fn hash_config(config: &crate::config::Config) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash check config fields that affect results
    config.check.cloc.max_lines.hash(&mut hasher);
    config.check.cloc.max_lines_test.hash(&mut hasher);
    config.check.cloc.exclude.hash(&mut hasher);
    config.project.packages.hash(&mut hasher);

    // Hash test/source patterns from resolution hierarchy:
    // 1. Language-specific patterns (most specific)
    // 2. Project-level patterns
    // Changes to any of these affect file classification.
    config.project.tests.hash(&mut hasher);
    config.project.source.hash(&mut hasher);
    config.rust.tests.hash(&mut hasher);
    config.rust.source.hash(&mut hasher);
    config.rust.ignore.hash(&mut hasher);
    config.golang.tests.hash(&mut hasher);
    config.golang.source.hash(&mut hasher);
    config.javascript.tests.hash(&mut hasher);
    config.javascript.source.hash(&mut hasher);
    config.shell.tests.hash(&mut hasher);
    config.shell.source.hash(&mut hasher);

    hasher.finish()
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
