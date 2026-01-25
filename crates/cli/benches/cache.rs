// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Warm cache benchmarks - the primary use case.
//!
//! Agents iterate repeatedly on a codebase. Most runs have a warm cache
//! where 95%+ of files are unchanged. This must be < 100ms.
//!
//! Performance targets from docs/specs/20-performance.md:
//! - Cold run: < 500ms target, < 1s acceptable, > 2s unacceptable
//! - Warm run: < 100ms target, < 200ms acceptable, > 500ms unacceptable

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
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

/// Check if the quench binary has a `check` command.
fn has_check_command() -> bool {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let output = Command::new(quench_bin)
        .arg("check")
        .arg("--help")
        .output()
        .expect("quench should run");

    output.status.success()
}

/// Warm the cache by running quench once, then benchmark subsequent runs.
fn bench_warm_cache(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    if !has_check_command() {
        eprintln!("Skipping warm cache benchmarks: 'check' command not available");
        // Run a minimal benchmark to keep criterion happy
        let mut group = c.benchmark_group("warm_cache");
        group.bench_function("placeholder", |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .arg("--help")
                    .output()
                    .expect("quench should run")
            })
        });
        group.finish();
        return;
    }

    let mut group = c.benchmark_group("warm_cache");

    for fixture in ["bench-small", "bench-medium"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: run scripts/fixtures/generate-bench-fixtures");
            continue;
        }

        // Warm the cache (setup, not measured)
        // Note: quench may return non-zero exit if violations are found, but
        // for benchmarking we only care that it runs to completion
        let cache_dir = path.join(".quench");
        let _ = std::fs::remove_dir_all(&cache_dir);
        Command::new(quench_bin)
            .args(["check", "--no-limit"])
            .current_dir(&path)
            .output()
            .expect("warmup should run");

        // Verify cache was created
        if !cache_dir.join("cache.bin").exists() {
            eprintln!("Skipping {fixture}: cache not created after warmup");
            continue;
        }

        // Benchmark warm runs
        group.bench_with_input(BenchmarkId::new("check_warm", fixture), &path, |b, path| {
            b.iter(|| {
                Command::new(quench_bin)
                    .args(["check", "--no-limit"])
                    .current_dir(path)
                    .output()
                    .expect("quench should run")
            })
        });
    }

    group.finish();
}

/// Measure cache speedup ratio (cold time / warm time).
fn bench_cache_speedup(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    if !has_check_command() {
        return;
    }

    let mut group = c.benchmark_group("cache_speedup");

    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping cache_speedup: bench-medium fixture not found");
        return;
    }

    let cache_dir = path.join(".quench");

    // Cold run (clear cache first)
    group.bench_function("cold", |b| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let _ = std::fs::remove_dir_all(&cache_dir);
                let start = std::time::Instant::now();
                Command::new(quench_bin)
                    .args(["check", "--no-limit"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                total += start.elapsed();
            }
            total
        })
    });

    // Ensure cache is warm for next benchmark
    let _ = std::fs::remove_dir_all(&cache_dir);
    Command::new(quench_bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warmup should succeed");

    // Warm run
    group.bench_function("warm", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--no-limit"])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

criterion_group!(benches, bench_warm_cache, bench_cache_speedup);
criterion_main!(benches);
