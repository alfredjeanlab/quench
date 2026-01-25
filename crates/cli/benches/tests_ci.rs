// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Benchmarks for tests check CI mode metrics.
//!
//! Tests performance of:
//! - Test runner execution and output parsing
//! - Metrics aggregation (timing, coverage)
//! - CI mode overhead vs fast mode

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use quench::checks::tests::runners::parse_cargo_output;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

fn quench_bin() -> &'static str {
    env!("CARGO_BIN_EXE_quench")
}

fn quench_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

//=============================================================================
// Phase 2: tests-ci Fixture Benchmarks
//=============================================================================

/// Benchmark the tests check on the tests-ci fixture in fast and CI modes.
fn bench_tests_ci_fixture(c: &mut Criterion) {
    let quench_bin = quench_bin();
    let path = fixture_path("tests-ci");

    if !path.exists() {
        eprintln!("Skipping: tests-ci fixture not found at {path:?}");
        return;
    }

    let mut group = c.benchmark_group("tests_ci");
    group.sample_size(20); // Fewer samples - runs actual cargo test

    // Fast mode (no metrics collection)
    group.bench_function("fast", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args([
                    "check",
                    "--tests",
                    "--no-cloc",
                    "--no-escapes",
                    "--no-agents",
                ])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    // CI mode (full metrics collection)
    group.bench_function("ci", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args([
                    "check",
                    "--tests",
                    "--no-cloc",
                    "--no-escapes",
                    "--no-agents",
                    "--ci",
                ])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    // CI mode JSON output
    group.bench_function("ci_json", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args([
                    "check",
                    "--tests",
                    "--no-cloc",
                    "--no-escapes",
                    "--no-agents",
                    "--ci",
                    "-o",
                    "json",
                ])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

//=============================================================================
// Phase 3: CI Mode Overhead Measurement
//=============================================================================

/// Measure overhead of --ci flag on dogfood (quench repo).
fn bench_ci_mode_overhead(c: &mut Criterion) {
    let quench_bin = quench_bin();
    let root = quench_root();

    let mut group = c.benchmark_group("ci_overhead");
    group.sample_size(10); // Fewer samples - runs actual cargo test

    // Tests check fast mode
    group.bench_function("tests_fast", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args([
                    "check",
                    "--tests",
                    "--no-cloc",
                    "--no-escapes",
                    "--no-agents",
                ])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    // Tests check CI mode
    group.bench_function("tests_ci", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args([
                    "check",
                    "--tests",
                    "--no-cloc",
                    "--no-escapes",
                    "--no-agents",
                    "--ci",
                ])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

//=============================================================================
// Phase 4: Metrics Parsing Benchmarks
//=============================================================================

/// Generate synthetic cargo test output for benchmarking.
fn generate_cargo_test_output(test_count: usize) -> String {
    let mut output = format!("running {} tests\n", test_count);
    for i in 0..test_count {
        output.push_str(&format!("test test_{} ... ok\n", i));
    }
    output.push_str(&format!(
        "\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s\n",
        test_count
    ));
    output
}

/// Benchmark cargo test output parsing.
fn bench_metrics_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests_metrics_parsing");

    // Small test output (10 tests)
    let small_output = generate_cargo_test_output(10);
    group.bench_function("small_10_tests", |b| {
        b.iter(|| parse_cargo_output(&small_output, Duration::from_secs(1)))
    });

    // Medium test output (100 tests)
    let medium_output = generate_cargo_test_output(100);
    group.bench_function("medium_100_tests", |b| {
        b.iter(|| parse_cargo_output(&medium_output, Duration::from_secs(1)))
    });

    // Large test output (1000 tests)
    let large_output = generate_cargo_test_output(1000);
    group.bench_function("large_1000_tests", |b| {
        b.iter(|| parse_cargo_output(&large_output, Duration::from_secs(1)))
    });

    group.finish();
}

//=============================================================================
// Criterion Configuration
//=============================================================================

criterion_group!(
    benches,
    // Phase 2: Fixture benchmarks
    bench_tests_ci_fixture,
    // Phase 3: CI overhead
    bench_ci_mode_overhead,
    // Phase 4: Metrics parsing
    bench_metrics_parsing,
);
criterion_main!(benches);
