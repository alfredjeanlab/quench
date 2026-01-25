// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Memory usage benchmarks.
//!
//! Validates memory targets from docs/specs/20-performance.md:
//! - Fast checks: < 100MB target, 500MB hard limit
//! - CI checks: < 500MB target, 2GB hard limit
//!
//! Uses `/usr/bin/time` to measure peak RSS since this provides the most
//! accurate measurement of actual memory consumption during execution.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Parse peak memory from /usr/bin/time output (macOS/Linux).
fn measure_peak_memory(fixture: &str) -> Option<u64> {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let path = fixture_path(fixture);

    if !path.exists() {
        eprintln!("Fixture {fixture} not found at {path:?}");
        return None;
    }

    // Use /usr/bin/time to measure peak RSS
    #[cfg(target_os = "macos")]
    let output = Command::new("/usr/bin/time")
        .args(["-l", quench_bin, "check", "--no-limit"])
        .current_dir(&path)
        .output()
        .ok()?;

    #[cfg(target_os = "linux")]
    let output = Command::new("/usr/bin/time")
        .args(["-v", quench_bin, "check", "--no-limit"])
        .current_dir(&path)
        .output()
        .ok()?;

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        eprintln!("Memory measurement not supported on this platform");
        return None;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse memory from time output
    #[cfg(target_os = "macos")]
    {
        // macOS: "  12345678  peak memory footprint"
        for line in stderr.lines() {
            if line.contains("peak memory footprint") {
                let bytes: u64 = line.split_whitespace().next()?.parse().ok()?;
                return Some(bytes);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: "Maximum resident set size (kbytes): 12345"
        for line in stderr.lines() {
            if line.contains("Maximum resident set size") {
                let kb: u64 = line.split(':').nth(1)?.trim().parse().ok()?;
                return Some(kb * 1024);
            }
        }
    }

    eprintln!("Could not parse memory from time output:\n{stderr}");
    None
}

#[test]
fn test_memory_bench_small() {
    if let Some(peak_bytes) = measure_peak_memory("bench-small") {
        let peak_mb = peak_bytes / (1024 * 1024);
        println!("bench-small peak memory: {}MB", peak_mb);

        // Target: < 100MB, Hard limit: 500MB
        assert!(
            peak_mb < 500,
            "Memory exceeded hard limit: {}MB > 500MB",
            peak_mb
        );

        if peak_mb > 100 {
            eprintln!("WARNING: Memory above target: {}MB > 100MB", peak_mb);
        }
    } else {
        eprintln!("Skipping memory test: could not measure memory");
    }
}

#[test]
fn test_memory_bench_medium() {
    if let Some(peak_bytes) = measure_peak_memory("bench-medium") {
        let peak_mb = peak_bytes / (1024 * 1024);
        println!("bench-medium peak memory: {}MB", peak_mb);

        // Target: < 100MB, Hard limit: 500MB
        assert!(
            peak_mb < 500,
            "Memory exceeded hard limit: {}MB > 500MB",
            peak_mb
        );

        if peak_mb > 100 {
            eprintln!("WARNING: Memory above target: {}MB > 100MB", peak_mb);
        }
    } else {
        eprintln!("Skipping memory test: could not measure memory");
    }
}

#[test]
fn test_memory_bench_large() {
    if let Some(peak_bytes) = measure_peak_memory("bench-large") {
        let peak_mb = peak_bytes / (1024 * 1024);
        println!("bench-large peak memory: {}MB", peak_mb);

        // CI target: < 500MB, Hard limit: 2GB
        assert!(
            peak_mb < 2048,
            "Memory exceeded hard limit: {}MB > 2GB",
            peak_mb
        );

        if peak_mb > 500 {
            eprintln!("WARNING: Memory above target: {}MB > 500MB", peak_mb);
        }
    } else {
        eprintln!("Skipping memory test: could not measure memory");
    }
}

// Note: This file uses harness = true in Cargo.toml to use the standard
// test harness. Run with: cargo test --bench memory -- --nocapture
