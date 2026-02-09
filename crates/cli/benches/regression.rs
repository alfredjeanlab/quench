// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Performance regression tests.
//!
//! These tests have hard time limits and fail if exceeded.
//! Unlike benchmarks (which compare to baselines), these are absolute limits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

fn quench_bin() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().join("target/release/quench")
}

/// Cold run must complete within 2 seconds (unacceptable threshold).
#[test]
fn cold_run_under_2s() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        eprintln!("Run: ./scripts/fixtures/generate-bench-fixtures");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    let cache_dir = path.join(".quench");
    let _ = std::fs::remove_dir_all(&cache_dir);

    let start = Instant::now();
    let output = Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    // Log actual time for debugging
    eprintln!("Cold run time: {:?}", elapsed);

    assert!(elapsed < Duration::from_secs(2), "Cold run took {:?}, exceeds 2s limit", elapsed);

    // Also verify it ran successfully (exit 0 or 1 for violations)
    assert!(output.status.code().unwrap_or(-1) <= 1, "Unexpected exit code: {:?}", output.status);
}

/// Warm run must complete within 500ms (unacceptable threshold).
#[test]
fn warm_run_under_500ms() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        eprintln!("Run: ./scripts/fixtures/generate-bench-fixtures");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    // Warm the cache
    let cache_dir = path.join(".quench");
    let _ = std::fs::remove_dir_all(&cache_dir);
    Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warmup should run");

    // Verify cache exists
    assert!(cache_dir.join("cache.bin").exists(), "Cache not created during warmup");

    let start = Instant::now();
    let output = Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    eprintln!("Warm run time: {:?}", elapsed);

    assert!(
        elapsed < Duration::from_millis(500),
        "Warm run took {:?}, exceeds 500ms limit",
        elapsed
    );

    assert!(output.status.code().unwrap_or(-1) <= 1, "Unexpected exit code: {:?}", output.status);
}

/// Cache speedup should be at least 2x.
///
/// Note: For small fixtures, speedup may be lower due to fixed overhead.
/// The 3x target applies to larger projects; we use 2x here for CI reliability.
#[test]
fn cache_provides_speedup() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        eprintln!("Run: ./scripts/fixtures/generate-bench-fixtures");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    let cache_dir = path.join(".quench");

    // Cold run - do multiple to avoid OS file cache effects
    let _ = std::fs::remove_dir_all(&cache_dir);

    // First run may be affected by OS file cache, so we measure it but
    // use subsequent runs for comparison
    Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warmup should complete");

    // Now do actual cold measurement
    let _ = std::fs::remove_dir_all(&cache_dir);
    let cold_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("cold run should complete");
    let cold_time = cold_start.elapsed();

    // Warm run
    let warm_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warm run should complete");
    let warm_time = warm_start.elapsed();

    eprintln!("Cold: {:?}, Warm: {:?}", cold_time, warm_time);

    // Skip speedup check if cold run is too fast (< 50ms) - fixture is too small
    if cold_time.as_millis() < 50 {
        eprintln!("Skipping speedup check: cold run too fast ({}ms < 50ms)", cold_time.as_millis());
        eprintln!("Fixture is too small to measure meaningful cache speedup");
        return;
    }

    let speedup = cold_time.as_millis() as f64 / warm_time.as_millis().max(1) as f64;
    eprintln!("Speedup: {:.1}x", speedup);

    // Use 2x threshold for reliability in CI (3x is target for large projects)
    assert!(
        speedup >= 2.0,
        "Cache speedup is only {:.1}x, expected at least 2x (cold: {:?}, warm: {:?})",
        speedup,
        cold_time,
        warm_time
    );
}

/// CI mode on tests-ci fixture must complete within 30s.
///
/// This is a conservative limit since it runs `cargo test` which
/// compiles and runs tests. The 30s limit matches the "unacceptable"
/// threshold from docs/specs/20-performance.md.
#[test]
fn tests_ci_mode_under_30s() {
    let path = fixture_path("tests-ci");
    if !path.exists() {
        eprintln!("Skipping: tests-ci fixture not found");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    let start = Instant::now();
    let output = Command::new(&bin)
        .args(["check", "--tests", "--ci"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    eprintln!("Tests CI mode time: {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(30),
        "Tests CI mode took {:?}, exceeds 30s limit",
        elapsed
    );

    // Should complete successfully (tests pass)
    assert!(output.status.code().unwrap_or(-1) <= 1, "Unexpected exit code: {:?}", output.status);
}

/// CI mode overhead should be bounded relative to fast mode.
///
/// The overhead comes from:
/// - Running actual tests (vs just correlation checking)
/// - Coverage collection (if configured)
///
/// This test ensures the overhead stays bounded (CI < 3x fast mode).
#[test]
fn tests_ci_mode_overhead_bounded() {
    let path = fixture_path("tests-ci");
    if !path.exists() {
        eprintln!("Skipping: tests-ci fixture not found");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    // Fast mode time (correlation only)
    let fast_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents"])
        .current_dir(&path)
        .output()
        .expect("fast mode should run");
    let fast_time = fast_start.elapsed();

    // CI mode time (run tests + metrics)
    let ci_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents", "--ci"])
        .current_dir(&path)
        .output()
        .expect("CI mode should run");
    let ci_time = ci_start.elapsed();

    eprintln!("Fast: {:?}, CI: {:?}", fast_time, ci_time);

    // CI overhead should be less than 200% of fast mode
    // (CI runs actual tests, so some overhead is expected)
    let overhead_pct = (ci_time.as_millis() as f64 / fast_time.as_millis().max(1) as f64) * 100.0;
    eprintln!("CI overhead: {:.1}%", overhead_pct - 100.0);

    assert!(
        ci_time < fast_time * 3,
        "CI mode overhead too high: {:?} vs {:?} ({:.1}%)",
        ci_time,
        fast_time,
        overhead_pct - 100.0
    );
}

fn main() {
    // This is a test harness, tests are run via cargo test
    println!("Run with: cargo test --bench regression -- --nocapture");
}
