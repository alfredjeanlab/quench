//! Cache behavioral specifications.
//!
//! Tests file-level caching behavior for faster iterative runs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::prelude::*;
use std::fs;
use std::thread;
use std::time::Duration;

/// Spec: docs/specs/performance.md#file-caching
///
/// > Cache file is created in .quench/cache.bin
#[test]
fn cache_file_created_after_check() {
    let dir = temp_project();
    fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path().join(".quench/cache.bin").exists(),
        "cache file should be created"
    );
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > --no-cache bypasses cache (no .quench directory created)
#[test]
fn no_cache_flag_skips_cache() {
    let dir = temp_project();
    fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    quench_cmd()
        .args(["check", "--no-cache"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        !dir.path().join(".quench").exists(),
        ".quench directory should not exist with --no-cache"
    );
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Cache reports hits/misses in verbose mode
#[test]
fn verbose_shows_cache_stats() {
    let dir = temp_project();
    fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    // First run: cache miss
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("Cache:"))
        .stderr(predicates::str::contains("miss"));

    // Second run: cache hit
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("Cache:"))
        .stderr(predicates::str::contains("hit"));
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Modifying a file causes cache miss for that file
#[test]
fn modified_file_causes_cache_miss() {
    let dir = temp_project();
    let test_file = dir.path().join("test.rs");
    fs::write(&test_file, "fn main() {}\n").unwrap();

    // First run: build cache (all misses)
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("0 hits"))
        .stderr(predicates::str::contains("miss"));

    // Second run: should hit cache (all hits, no misses)
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("hits"))
        .stderr(predicates::str::contains("0 misses"));

    // Touch file (change mtime)
    thread::sleep(Duration::from_millis(10));
    fs::write(&test_file, "fn main() {}\n").unwrap();

    // Third run: should have at least one miss for the touched file
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("miss").and(predicates::str::contains("0 misses").not()));
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Config changes invalidate entire cache
#[test]
fn config_change_invalidates_cache() {
    let dir = temp_project();
    fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    // First run: build cache with default config
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("0 hits"));

    // Second run: should hit cache
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("0 misses"));

    // Change config (this changes config hash)
    fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.cloc]
max_lines = 500
"#,
    )
    .unwrap();

    // Third run: should miss due to config change (cache invalidated)
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("0 hits"));
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Cache persists across sessions (not just in-memory)
#[test]
fn cache_persists_across_invocations() {
    let dir = temp_project();
    fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    // First run: build cache
    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify cache file exists
    let cache_path = dir.path().join(".quench/cache.bin");
    assert!(cache_path.exists());

    // Get initial cache file size
    let cache_size = fs::metadata(&cache_path).unwrap().len();
    assert!(cache_size > 0, "cache should not be empty");

    // Second run: should use persisted cache (all hits, no misses)
    quench_cmd()
        .args(["check", "-v"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("hits"))
        .stderr(predicates::str::contains("0 misses"));
}
