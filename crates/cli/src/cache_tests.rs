// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::tempdir;

#[test]
fn file_cache_key_from_metadata() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "hello").unwrap();

    let meta = std::fs::metadata(&path).unwrap();
    let key = FileCacheKey::from_metadata(&meta);

    assert_eq!(key.size, 5);
    assert!(key.mtime_secs > 0);
}

#[test]
fn cache_lookup_miss_on_empty() {
    let cache = FileCache::new(0);
    let key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 50,
    };

    let result = cache.lookup(Path::new("nonexistent.rs"), &key);
    assert!(result.is_none());
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn cache_insert_and_lookup_hit() {
    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 50,
    };

    let violations = vec![CachedViolation {
        check: "cloc".to_string(),
        line: Some(10),
        violation_type: "file_too_large".to_string(),
        advice: "Split the file".to_string(),
        value: None,
        threshold: None,
        pattern: None,
        lines: None,
        nonblank: None,
        target_path: None,
    }];

    cache.insert(path.clone(), key.clone(), violations.clone());

    let result = cache.lookup(&path, &key);
    assert!(result.is_some());
    let cached = result.unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].check, "cloc");
    assert_eq!(cache.stats().hits, 1);
}

#[test]
fn cache_miss_on_mtime_change() {
    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let old_key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 50,
    };
    let new_key = FileCacheKey {
        mtime_secs: 200, // Changed
        mtime_nanos: 0,
        size: 50,
    };

    cache.insert(path.clone(), old_key, vec![]);

    let result = cache.lookup(&path, &new_key);
    assert!(result.is_none());
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn cache_miss_on_size_change() {
    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let old_key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 50,
    };
    let new_key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 100, // Changed
    };

    cache.insert(path.clone(), old_key, vec![]);

    let result = cache.lookup(&path, &new_key);
    assert!(result.is_none());
}

#[test]
fn cache_persist_and_restore() {
    let dir = tempdir().unwrap();
    let cache_path = dir.path().join("cache.bin");
    let config_hash = 12345u64;

    // Create and populate cache
    let cache = FileCache::new(config_hash);
    let file_path = PathBuf::from("src/lib.rs");
    let key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 500,
        size: 1000,
    };
    cache.insert(
        file_path.clone(),
        key.clone(),
        vec![CachedViolation {
            check: "cloc".to_string(),
            line: Some(42),
            violation_type: "file_too_large".to_string(),
            advice: "Refactor".to_string(),
            value: Some(100),
            threshold: Some(50),
            pattern: None,
            lines: None,
            nonblank: None,
            target_path: None,
        }],
    );

    // Persist
    cache.persist(&cache_path).unwrap();

    // Restore
    let restored = FileCache::from_persistent(&cache_path, config_hash).unwrap();

    // Verify
    let result = restored.lookup(&file_path, &key);
    assert!(result.is_some());
    let violations = result.unwrap();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, Some(42));
}

#[test]
fn cache_rejects_version_mismatch() {
    let dir = tempdir().unwrap();
    let cache_path = dir.path().join("cache.bin");

    // Create cache with wrong version
    let bad_cache = PersistentCache {
        version: CACHE_VERSION + 1, // Wrong version
        quench_version: env!("CARGO_PKG_VERSION").to_string(),
        config_hash: 0,
        files: HashMap::new(),
    };

    let bytes = postcard::to_allocvec(&bad_cache).unwrap();
    std::fs::write(&cache_path, &bytes).unwrap();

    let result = FileCache::from_persistent(&cache_path, 0);
    assert!(matches!(result, Err(CacheError::VersionMismatch)));
}

#[test]
fn cache_rejects_config_change() {
    let dir = tempdir().unwrap();
    let cache_path = dir.path().join("cache.bin");

    // Create cache with different config hash
    let cache = FileCache::new(111);
    cache.persist(&cache_path).unwrap();

    let result = FileCache::from_persistent(&cache_path, 222);
    assert!(matches!(result, Err(CacheError::ConfigChanged)));
}

#[test]
fn cached_violation_roundtrip() {
    let violation = Violation::file("src/main.rs", 10, "test_type", "test advice");
    let cached = CachedViolation::from_violation(&violation, "test_check");

    assert_eq!(cached.check, "test_check");
    assert_eq!(cached.line, Some(10));
    assert_eq!(cached.violation_type, "test_type");
    assert_eq!(cached.advice, "test advice");

    let restored = cached.to_violation(PathBuf::from("src/main.rs"));
    assert_eq!(restored.line, Some(10));
    assert_eq!(restored.violation_type, "test_type");
}

#[test]
fn hash_config_deterministic() {
    let config = crate::config::Config::default();
    let hash1 = hash_config(&config);
    let hash2 = hash_config(&config);
    assert_eq!(hash1, hash2);
}
