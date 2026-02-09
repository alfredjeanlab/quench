// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
    let key = FileCacheKey { mtime_secs: 100, mtime_nanos: 0, size: 50 };

    let result = cache.lookup(Path::new("nonexistent.rs"), &key);
    assert!(result.is_none());
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn cache_insert_and_lookup_hit() {
    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let key = FileCacheKey { mtime_secs: 100, mtime_nanos: 0, size: 50 };

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
    let old_key = FileCacheKey { mtime_secs: 100, mtime_nanos: 0, size: 50 };
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
    let old_key = FileCacheKey { mtime_secs: 100, mtime_nanos: 0, size: 50 };
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
    let key = FileCacheKey { mtime_secs: 100, mtime_nanos: 500, size: 1000 };
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

#[test]
fn cache_persist_async_completes() {
    let dir = tempdir().unwrap();
    let cache_path = dir.path().join("cache.bin");
    let config_hash = 12345u64;

    // Create and populate cache
    let cache = FileCache::new(config_hash);
    let file_path = PathBuf::from("src/lib.rs");
    let key = FileCacheKey { mtime_secs: 100, mtime_nanos: 500, size: 1000 };
    cache.insert(file_path.clone(), key.clone(), vec![]);

    // Persist asynchronously and wait for completion
    let handle = cache.persist_async(cache_path.clone());
    handle.join().expect("thread panicked").expect("persist failed");

    // Verify file exists and can be restored
    assert!(cache_path.exists());
    let restored = FileCache::from_persistent(&cache_path, config_hash).unwrap();
    let result = restored.lookup(&file_path, &key);
    assert!(result.is_some());
}

#[test]
fn cache_lookup_returns_arc_for_efficient_cloning() {
    use std::sync::Arc;

    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let key = FileCacheKey { mtime_secs: 100, mtime_nanos: 0, size: 50 };

    // Insert violations
    let violations = vec![CachedViolation {
        check: "test".to_string(),
        line: Some(1),
        violation_type: "test".to_string(),
        advice: "test".to_string(),
        value: None,
        threshold: None,
        pattern: None,
        lines: None,
        nonblank: None,
        target_path: None,
    }];
    cache.insert(path.clone(), key.clone(), violations);

    // Get two references - should be the same Arc (same pointer)
    let arc1 = cache.lookup(&path, &key).unwrap();
    let arc2 = cache.lookup(&path, &key).unwrap();

    // Verify both point to same underlying data (Arc::ptr_eq)
    assert!(Arc::ptr_eq(&arc1, &arc2));
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn cache_handles_epoch_mtime() {
    // Test that cache works correctly with epoch mtime (1970-01-01)
    let cache = FileCache::new(0);
    let path = PathBuf::from("ancient.rs");
    let key = FileCacheKey {
        mtime_secs: 0, // Epoch
        mtime_nanos: 0,
        size: 100,
    };

    // Should not panic or cause issues
    cache.insert(path.clone(), key.clone(), vec![]);
    let result = cache.lookup(&path, &key);
    assert!(result.is_some());
    assert_eq!(cache.stats().hits, 1);
}

#[test]
fn cache_handles_pre_epoch_mtime_gracefully() {
    // Test that cache works with mtime that represents a time before epoch
    // (would have negative duration, but from_metadata handles this)
    let cache = FileCache::new(0);
    let path = PathBuf::from("very_old.rs");

    // Pre-epoch times get clamped to 0 by from_metadata (unwrap_or_default)
    // This simulates what would happen
    let key = FileCacheKey { mtime_secs: 0, mtime_nanos: 0, size: 50 };

    cache.insert(path.clone(), key.clone(), vec![]);
    let result = cache.lookup(&path, &key);
    assert!(result.is_some());
}

#[test]
fn cache_concurrent_insert_lookup() {
    use std::sync::Arc;
    use std::thread;

    let cache = Arc::new(FileCache::new(0));
    let num_threads = 10;
    let num_ops = 100;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || {
                for i in 0..num_ops {
                    let path = PathBuf::from(format!("file_{}.rs", i));
                    let key = FileCacheKey {
                        mtime_secs: i as i64,
                        mtime_nanos: 0,
                        size: (thread_id * num_ops + i) as u64,
                    };

                    // Insert
                    cache.insert(path.clone(), key.clone(), vec![]);

                    // Lookup (may hit or miss depending on race with other threads)
                    let _ = cache.lookup(&path, &key);
                }
            })
        })
        .collect();

    // All threads should complete without panic
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    // Cache should have entries (not necessarily all due to overwrites)
    assert!(cache.stats().entries > 0);
}

#[test]
fn file_cache_key_from_walked_file_epoch() {
    use crate::file_size::FileSizeClass;
    use crate::walker::WalkedFile;

    // Simulate a file with epoch mtime
    let walked = WalkedFile {
        path: PathBuf::from("test.rs"),
        mtime_secs: 0, // Epoch
        mtime_nanos: 0,
        size: 100,
        depth: 0,
        size_class: FileSizeClass::Small,
    };

    let key = FileCacheKey::from_walked_file(&walked);
    assert_eq!(key.mtime_secs, 0);
    assert_eq!(key.mtime_nanos, 0);
    assert_eq!(key.size, 100);
}

#[test]
fn hash_config_changes_when_escapes_check_off() {
    use crate::config::CheckLevel;

    let mut config = crate::config::Config::default();
    let hash_default = hash_config(&config);

    config.check.escapes.check = CheckLevel::Off;
    let hash_off = hash_config(&config);

    assert_ne!(hash_default, hash_off, "config hash must change when check.escapes.check changes");
}

#[test]
fn hash_config_changes_when_suppress_check_changes() {
    use crate::config::SuppressLevel;

    let mut config = crate::config::Config::default();
    let hash_default = hash_config(&config);

    config.javascript.suppress.check = SuppressLevel::Allow;
    let hash_changed = hash_config(&config);

    assert_ne!(
        hash_default, hash_changed,
        "config hash must change when javascript.suppress.check changes"
    );
}
