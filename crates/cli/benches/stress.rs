// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Stress benchmarks for Rust adapter edge cases.
//!
//! Tests performance under pathological conditions:
//! - Large files (10K-50K lines)
//! - Many #[cfg(test)] blocks (50+ per file)
//! - Large workspaces (50 packages, 1000 files)
//! - Deep module nesting (20 levels)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;

use quench::adapter::Adapter;
use quench::adapter::rust::{CargoWorkspace, CfgTestInfo, RustAdapter};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/stress-rust")
        .join(name)
}

/// Benchmark large file parsing.
fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_large_files");
    group.sample_size(20); // Fewer samples for slow benchmarks

    let cfg_test_attr = "#[cfg(test)]";

    // Generate content inline for predictable sizing
    for lines in [10_000, 50_000] {
        let content: String = (0..lines)
            .map(|i| {
                if i % 100 == 50 {
                    format!(
                        "{}\nmod tests_{} {{ #[test] fn t() {{}} }}\n",
                        cfg_test_attr, i
                    )
                } else {
                    format!("pub fn func_{}() {{ }}\n", i)
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("cfg_test_parse", format!("{}_lines", lines)),
            &content,
            |b, content| b.iter(|| black_box(CfgTestInfo::parse(content))),
        );
    }

    group.finish();
}

/// Benchmark many #[cfg(test)] blocks in single file.
fn bench_many_cfg_test_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_many_cfg_test");

    let cfg_test_attr = "#[cfg(test)]";

    // 50 separate #[cfg(test)] blocks
    let content: String = (0..50)
        .map(|i| {
            format!(
                "pub fn func_{}() {{}}\n\n{}\nmod tests_{} {{\n    #[test]\n    fn test() {{}}\n}}\n\n",
                i, cfg_test_attr, i
            )
        })
        .collect();

    group.bench_function("50_blocks", |b| {
        b.iter(|| black_box(CfgTestInfo::parse(&content)))
    });

    group.finish();
}

/// Benchmark large workspace detection.
fn bench_large_workspace(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_workspace");
    group.sample_size(20);

    let fixture = fixture_path("many-packages");
    if fixture.exists() {
        group.bench_function("50_packages", |b| {
            b.iter(|| black_box(CargoWorkspace::from_root(&fixture)))
        });
    }

    group.finish();
}

/// Benchmark file classification on large workspace.
fn bench_large_workspace_classify(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_classify");

    let adapter = RustAdapter::new();

    // 1000 paths (50 packages × 20 files)
    let paths: Vec<PathBuf> = (1..=50)
        .flat_map(|pkg| {
            (1..=20).map(move |f| PathBuf::from(format!("crates/pkg_{}/src/mod_{}.rs", pkg, f)))
        })
        .collect();

    group.bench_function("1000_files", |b| {
        b.iter(|| {
            for path in &paths {
                black_box(adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark deep module nesting path classification.
fn bench_deep_nesting(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_deep_nesting");

    let adapter = RustAdapter::new();

    // 20 levels of nesting
    let deep_paths: Vec<PathBuf> = (1..=20)
        .map(|level| {
            let mut path = PathBuf::from("src");
            for l in 1..=level {
                path.push(format!("level_{}", l));
            }
            path.push("mod.rs");
            path
        })
        .collect();

    group.bench_function("20_levels", |b| {
        b.iter(|| {
            for path in &deep_paths {
                black_box(adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// End-to-end stress benchmarks using disk fixtures.
///
/// These benchmarks run quench against generated stress fixtures.
/// Generate fixtures first with: ./scripts/fixtures/generate-stress-fixtures
fn bench_stress_e2e(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_e2e");
    group.sample_size(10); // Fewer samples for slow benchmarks

    let quench_bin = env!("CARGO_BIN_EXE_quench");

    // Define fixtures with their expected performance characteristics
    let fixtures = [
        ("stress-huge-files", "50K files traversal"),
        ("stress-monorepo", "5K files workspace"),
        ("stress-large-file", "1-5MB files"),
    ];

    for (fixture, description) in fixtures {
        let path = e2e_fixture_path(fixture);
        if !path.exists() {
            eprintln!(
                "Skipping {fixture} ({description}): run ./scripts/fixtures/generate-stress-fixtures"
            );
            continue;
        }

        group.bench_function(fixture, |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .arg("check")
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    }

    group.finish();
}

/// Path to E2E stress fixtures (tests/fixtures/stress-*)
fn e2e_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

criterion_group!(
    benches,
    bench_large_files,
    bench_many_cfg_test_blocks,
    bench_large_workspace,
    bench_large_workspace_classify,
    bench_deep_nesting,
    bench_stress_e2e,
);
criterion_main!(benches);
