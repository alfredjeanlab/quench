//! Cache behavioral specifications.
//!
//! Tests file-level caching behavior for faster iterative runs.
//!
//! NOTE: These tests use quench_cmd() directly because the CheckBuilder
//! always adds --no-cache, but these tests specifically test cache behavior.

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
    let temp = default_project();
    fs::write(temp.path().join("test.rs"), "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - cache tests need cache enabled
    quench_cmd()
        .args(["check"])
        .current_dir(temp.path())
        .assert()
        .success();

    assert!(
        temp.path().join(".quench/cache.bin").exists(),
        "cache file should be created"
    );
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > --no-cache bypasses cache (no cache.bin file created)
///
/// Note: latest.json is still written even with --no-cache
#[test]
fn no_cache_flag_skips_cache() {
    let temp = default_project();
    fs::write(temp.path().join("test.rs"), "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - testing --no-cache flag behavior
    quench_cmd()
        .args(["check", "--no-cache"])
        .current_dir(temp.path())
        .assert()
        .success();

    assert!(
        !temp.path().join(".quench/cache.bin").exists(),
        "cache.bin should not exist with --no-cache"
    );
    // latest.json is still written for local metrics viewing
    assert!(
        temp.path().join(".quench/latest.json").exists(),
        "latest.json should still be written even with --no-cache"
    );
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Cache reports hits/misses in verbose mode
/// > Format: "Cache: N hits, M misses"
#[test]
fn verbose_shows_cache_stats() {
    let temp = default_project();
    fs::write(temp.path().join("test.rs"), "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - cache tests need cache enabled
    // First run: cache miss
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, \d+ misses?").unwrap());

    // Second run: cache hit
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, 0 misses?").unwrap());
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Modifying a file causes cache miss for that file
/// > Format: "Cache: N hits, M misses"
#[test]
fn modified_file_causes_cache_miss() {
    let temp = default_project();
    let test_file = temp.path().join("test.rs");
    fs::write(&test_file, "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - cache tests need cache enabled
    // First run: build cache (all misses)
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: 0 hits?, \d+ misses?").unwrap());

    // Second run: should hit cache (all hits)
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, 0 misses?").unwrap());

    // Touch file (change mtime)
    thread::sleep(Duration::from_millis(10));
    fs::write(&test_file, "fn main() {}\n").unwrap();

    // Third run: should have at least one miss for the touched file
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, [1-9]\d* misses?").unwrap());
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Config changes invalidate entire cache
/// > Format: "Cache: N hits, M misses"
#[test]
fn config_change_invalidates_cache() {
    let temp = default_project();
    fs::write(temp.path().join("test.rs"), "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - cache tests need cache enabled
    // First run: build cache with default config
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: 0 hits?, \d+ misses?").unwrap());

    // Second run: should hit cache
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, 0 misses?").unwrap());

    // Change config (this changes config hash)
    fs::write(
        temp.path().join("quench.toml"),
        r#"version = 1
[check.cloc]
max_lines = 500
"#,
    )
    .unwrap();

    // Third run: should miss due to config change (cache invalidated)
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: 0 hits?, \d+ misses?").unwrap());
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Docs violations with target paths (broken_link, broken_toc) are invalidated
/// > when the target file is created, even if the source file is cached.
#[test]
fn docs_cache_invalidated_when_target_created() {
    let temp = default_project();

    // Create a markdown file with a broken link
    fs::write(
        temp.path().join("README.md"),
        "# Project\n\nSee [missing](missing.md) for info.\n",
    )
    .unwrap();

    // First run: should report broken_link violation
    quench_cmd()
        .args(["check", "--docs", "-o", "json"])
        .current_dir(temp.path())
        .assert()
        .code(1)
        .stdout(predicates::str::contains("broken_link"));

    // Second run: should still report violation (from cache)
    quench_cmd()
        .args(["check", "--docs", "-o", "json", "--timing"])
        .current_dir(temp.path())
        .assert()
        .code(1)
        .stdout(predicates::str::contains("broken_link"))
        .stdout(predicates::str::contains(r#""cache_hits""#)); // Verify cache is being used

    // Create the missing file (simulating --fix behavior)
    fs::write(temp.path().join("missing.md"), "# Missing\n").unwrap();

    // Third run: should PASS because target now exists (cache violation invalidated)
    quench_cmd()
        .args(["check", "--docs", "-o", "json"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""passed": true"#));
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Docs violations are invalidated when target symlinks are created.
#[test]
#[cfg(unix)]
fn docs_cache_invalidated_when_target_symlink_created() {
    use std::os::unix::fs::symlink;

    let temp = default_project();

    // Create a markdown file with a broken link
    fs::write(
        temp.path().join("README.md"),
        "# Project\n\nSee [missing](missing.md) for info.\n",
    )
    .unwrap();

    // First run: should report broken_link violation
    quench_cmd()
        .args(["check", "--docs", "-o", "json"])
        .current_dir(temp.path())
        .assert()
        .code(1)
        .stdout(predicates::str::contains("broken_link"));

    // Create target file and symlink (simulating --fix that creates symlinks)
    fs::write(temp.path().join("target.md"), "# Target\n").unwrap();
    symlink("target.md", temp.path().join("missing.md")).unwrap();

    // Second run: should PASS because symlink target exists
    quench_cmd()
        .args(["check", "--docs", "-o", "json"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""passed": true"#));
}

/// Spec: docs/specs/performance.md#file-caching
///
/// > Cache persists across sessions (not just in-memory)
/// > Format: "Cache: N hits, M misses"
#[test]
fn cache_persists_across_invocations() {
    let temp = default_project();
    fs::write(temp.path().join("test.rs"), "fn main() {}\n").unwrap();

    // Uses quench_cmd() directly - cache tests need cache enabled
    // First run: build cache
    quench_cmd()
        .args(["check"])
        .current_dir(temp.path())
        .assert()
        .success();

    // Verify cache file exists
    let cache_path = temp.path().join(".quench/cache.bin");
    assert!(cache_path.exists());

    // Get initial cache file size
    let cache_size = fs::metadata(&cache_path).unwrap().len();
    assert!(cache_size > 0, "cache should not be empty");

    // Second run: should use persisted cache (all hits)
    quench_cmd()
        .args(["check"])
        .env("QUENCH_DEBUG", "1")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_match(r"Cache: \d+ hits?, 0 misses?").unwrap());
}
