// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Full check pipeline benchmarks.
//!
//! Measures end-to-end quench performance including:
//! - File walking
//! - File reading
//! - Pattern matching
//! - Output generation
//!
//! Note: These benchmarks require the `check` command to be implemented.
//! Until then, they measure CLI invocation overhead with --help.

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

fn bench_check_cold(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    // Skip if check command doesn't exist yet
    if !has_check_command() {
        eprintln!("Skipping check benchmarks: 'check' command not yet implemented");

        // Run a minimal benchmark to keep criterion happy
        let mut group = c.benchmark_group("check_cold");
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

    let mut group = c.benchmark_group("check_cold");

    for fixture in ["bench-small", "bench-medium", "bench-large"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: fixture not found at {path:?}");
            continue;
        }

        group.bench_with_input(BenchmarkId::new("check", fixture), &path, |b, path| {
            b.iter(|| {
                Command::new(quench_bin)
                    .arg("check")
                    .current_dir(path)
                    .output()
                    .expect("quench check should run")
            })
        });
    }

    group.finish();
}

fn bench_check_deep(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    if !has_check_command() {
        return;
    }

    let mut group = c.benchmark_group("check_deep");

    let path = fixture_path("bench-deep");
    if !path.exists() {
        eprintln!("Skipping bench-deep: fixture not found");
        return;
    }

    group.bench_with_input(BenchmarkId::new("check", "bench-deep"), &path, |b, path| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("check")
                .current_dir(path)
                .output()
                .expect("quench check should run")
        })
    });

    group.finish();
}

fn bench_check_large_files(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    if !has_check_command() {
        return;
    }

    let mut group = c.benchmark_group("check_large_files");

    let path = fixture_path("bench-large-files");
    if !path.exists() {
        eprintln!("Skipping bench-large-files: fixture not found");
        return;
    }

    group.bench_with_input(
        BenchmarkId::new("check", "bench-large-files"),
        &path,
        |b, path| {
            b.iter(|| {
                Command::new(quench_bin)
                    .arg("check")
                    .current_dir(path)
                    .output()
                    .expect("quench check should run")
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_check_cold,
    bench_check_deep,
    bench_check_large_files
);
criterion_main!(benches);
