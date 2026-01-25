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
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/release/quench")
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

    assert!(
        elapsed < Duration::from_secs(2),
        "Cold run took {:?}, exceeds 2s limit",
        elapsed
    );

    // Also verify it ran successfully (exit 0 or 1 for violations)
    assert!(
        output.status.code().unwrap_or(-1) <= 1,
        "Unexpected exit code: {:?}",
        output.status
    );
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
    assert!(
        cache_dir.join("cache.bin").exists(),
        "Cache not created during warmup"
    );

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

    assert!(
        output.status.code().unwrap_or(-1) <= 1,
        "Unexpected exit code: {:?}",
        output.status
    );
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
        eprintln!(
            "Skipping speedup check: cold run too fast ({}ms < 50ms)",
            cold_time.as_millis()
        );
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

fn main() {
    // This is a test harness, tests are run via cargo test
    println!("Run with: cargo test --bench regression -- --nocapture");
}
