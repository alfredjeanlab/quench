// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File walking benchmarks.
//!
//! Measures file discovery performance using the `ignore` crate's
//! parallel walker. This isolates walker performance from file reading
//! and checking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Walk a fixture directory and count files (single-threaded).
fn walk_fixture_single(path: &Path) -> usize {
    let mut count = 0;
    for entry in WalkBuilder::new(path).hidden(true).git_ignore(true).threads(1).build().flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            count += 1;
        }
    }
    count
}

/// Walk a fixture directory and count files (parallel).
fn walk_fixture_parallel(path: &Path) -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let count = AtomicUsize::new(0);

    WalkBuilder::new(path).hidden(true).git_ignore(true).build_parallel().run(|| {
        Box::new(|entry| {
            if let Ok(entry) = entry
                && entry.file_type().map(|t| t.is_file()).unwrap_or(false)
            {
                count.fetch_add(1, Ordering::Relaxed);
            }
            ignore::WalkState::Continue
        })
    });

    count.load(Ordering::Relaxed)
}

fn bench_file_walking_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_walking/single");

    for fixture in ["bench-small", "bench-medium", "bench-large", "bench-deep", "bench-large-files"]
    {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: fixture not found at {path:?}");
            continue;
        }

        group.bench_with_input(BenchmarkId::new("walk", fixture), &path, |b, path| {
            b.iter(|| walk_fixture_single(path))
        });
    }

    group.finish();
}

fn bench_file_walking_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_walking/parallel");

    for fixture in ["bench-small", "bench-medium", "bench-large", "bench-deep", "bench-large-files"]
    {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: fixture not found at {path:?}");
            continue;
        }

        group.bench_with_input(BenchmarkId::new("walk", fixture), &path, |b, path| {
            b.iter(|| walk_fixture_parallel(path))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_file_walking_single, bench_file_walking_parallel);
criterion_main!(benches);
