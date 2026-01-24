// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File-level caching for check results.
//!
//! Caches check violations per file using mtime+size as cache key.
//! Provides 10x speedup on iterative runs where few files change.

use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::check::Violation;

/// Cache version for invalidation on format changes.
/// Incremented when check logic changes (e.g., counting nonblank vs all lines).
/// v7: Migrated from bincode to postcard serialization.
/// v8: Fixed cfg(test) detection false positives on comments.
pub const CACHE_VERSION: u32 = 9;

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

/// Cached result for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFileResult {
    /// Cache key when this result was computed.
    pub key: FileCacheKey,
    /// Violations found in this file (across all checks).
    pub violations: Vec<CachedViolation>,
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
    /// Per-file cached results.
    pub files: HashMap<PathBuf, CachedFileResult>,
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

        Ok(Self {
            inner: cache.files.into_iter().collect(),
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
    pub fn lookup(&self, path: &Path, key: &FileCacheKey) -> Option<Vec<CachedViolation>> {
        if let Some(entry) = self.inner.get(path)
            && entry.key == *key
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(entry.violations.clone());
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Insert or update a file's cached result.
    pub fn insert(&self, path: PathBuf, key: FileCacheKey, violations: Vec<CachedViolation>) {
        self.inner
            .insert(path, CachedFileResult { key, violations });
    }

    /// Persist cache to disk.
    pub fn persist(&self, path: &Path) -> Result<(), CacheError> {
        let cache = PersistentCache {
            version: CACHE_VERSION,
            quench_version: self.quench_version.clone(),
            config_hash: self.config_hash,
            files: self
                .inner
                .iter()
                .map(|e| (e.key().clone(), e.value().clone()))
                .collect(),
        };

        // Write atomically via temp file
        let temp_path = path.with_extension("tmp");
        let bytes = postcard::to_allocvec(&cache)?;
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, path)?;
        Ok(())
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
    config.check.cloc.test_patterns.hash(&mut hasher);
    config.check.cloc.exclude.hash(&mut hasher);
    config.workspace.packages.hash(&mut hasher);

    hasher.finish()
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
