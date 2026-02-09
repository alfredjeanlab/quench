// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Docs check benchmarks.
//!
//! Measures performance of:
//! - TOC parsing and validation
//! - Link extraction and resolution
//! - Specs validation (TOC and Linked modes)
//! - End-to-end docs check on various sizes

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Path to small docs fixtures (tests/fixtures/docs/*)
fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/docs")
        .join(name)
}

/// Path to stress test fixtures (tests/fixtures/stress-docs/*)
fn stress_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/stress-docs")
        .join(name)
}

/// Generate a nested directory tree of specified depth using box-drawing format.
fn generate_nested_tree(depth: usize) -> String {
    let mut lines = vec!["```".to_string(), "src/".to_string()];
    for level in 1..=depth {
        let prefix = if level == depth { "└── " } else { "├── " };
        let indent = "│   ".repeat(level - 1);
        lines.push(format!("{}{}level_{}/", indent, prefix, level));
    }
    // Add a file at the deepest level
    let deepest_indent = "│   ".repeat(depth - 1) + "    ";
    lines.push(format!("{}└── mod.rs", deepest_indent));
    lines.push("```".to_string());
    lines.join("\n")
}

/// Generate a wide directory tree with specified number of entries.
fn generate_wide_tree(width: usize) -> String {
    let mut lines = vec!["```".to_string(), "src/".to_string()];
    for i in 1..width {
        lines.push(format!("├── file_{}.rs", i));
    }
    lines.push(format!("└── file_{}.rs", width));
    lines.push("```".to_string());
    lines.join("\n")
}

/// Generate markdown content with N links.
fn generate_content_with_links(count: usize) -> String {
    (0..count).map(|i| format!("See [file {}](path/to/file_{}.md) for details.\n", i, i)).collect()
}

/// Benchmark TOC tree generation and parsing helpers.
///
/// Tests the helper functions that generate tree content.
fn bench_tree_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_tree_gen");

    // Benchmark tree generation at various depths
    for depth in [5, 10, 20] {
        group.bench_with_input(BenchmarkId::new("nested_depth", depth), &depth, |b, &depth| {
            b.iter(|| black_box(generate_nested_tree(depth)))
        });
    }

    // Benchmark wide tree generation
    for width in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("wide_width", width), &width, |b, &width| {
            b.iter(|| black_box(generate_wide_tree(width)))
        });
    }

    group.finish();
}

/// Benchmark link content generation.
fn bench_link_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("docs_link_gen");

    for count in [10, 50, 200] {
        group.bench_with_input(BenchmarkId::new("count", count), &count, |b, &count| {
            b.iter(|| black_box(generate_content_with_links(count)))
        });
    }

    group.finish();
}

/// End-to-end benchmarks on small fixtures.
///
/// Tests full CLI invocation on the standard test fixtures.
fn bench_docs_e2e_small(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("docs_e2e_small");
    group.sample_size(20);

    // Test various small fixtures that exist
    let fixtures = [
        ("toc-ok", "Valid TOC validation"),
        ("toc-broken", "Broken TOC detection"),
        ("link-ok", "Valid link validation"),
        ("link-broken", "Broken link detection"),
        ("index-toc", "Index with TOC mode"),
        ("index-linked", "Index with linked mode"),
    ];

    for (name, description) in fixtures {
        let path = fixture_path(name);
        if !path.exists() {
            eprintln!("Skipping {name} ({description}): fixture not found");
            continue;
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .args(["check", "--docs"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    }

    group.finish();
}

/// End-to-end benchmarks on stress fixtures.
///
/// Tests full CLI invocation on large generated fixtures.
/// Generate fixtures first with: ./scripts/fixtures/generate-docs-stress
fn bench_docs_e2e_stress(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("docs_e2e_stress");
    group.sample_size(10); // Fewer samples for slower benchmarks

    let fixtures = [
        ("many-files", "500 spec files"),
        ("deep-links", "50-level link chain"),
        ("large-toc", "100-entry TOC"),
        ("many-links", "Single file with 500+ links"),
    ];

    for (name, description) in fixtures {
        let path = stress_fixture_path(name);
        if !path.exists() {
            eprintln!(
                "Skipping {name} ({description}): run ./scripts/fixtures/generate-docs-stress"
            );
            continue;
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .args(["check", "--docs"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    }

    group.finish();
}

/// Benchmark docs check in CI mode (includes commit checking).
fn bench_docs_ci_mode(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("docs_ci");
    group.sample_size(10);

    // Use the docs-project fixture which has git setup
    let project = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/docs-project");

    if project.exists() {
        group.bench_function("with_commit_check", |b| {
            b.iter(|| {
                let output = Command::new(quench_bin)
                    .args(["check", "--ci", "--docs"])
                    .current_dir(&project)
                    .output()
                    .expect("quench should run");
                black_box(output)
            })
        });
    } else {
        eprintln!("Skipping CI mode benchmark: docs-project fixture not found");
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_tree_generation,
    bench_link_generation,
    bench_docs_e2e_small,
    bench_docs_e2e_stress,
    bench_docs_ci_mode,
);
criterion_main!(benches);
