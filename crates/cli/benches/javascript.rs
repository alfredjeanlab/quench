// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript adapter benchmarks.
//!
//! Measures performance of JavaScript/TypeScript-specific operations:
//! - Adapter creation (GlobSet compilation for 22 patterns)
//! - File classification with ignore pattern checking
//! - Workspace detection (pnpm/npm)
//! - ESLint/Biome suppress directive parsing

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::{Path, PathBuf};

use quench::adapter::Adapter;
use quench::adapter::javascript::{JavaScriptAdapter, JsWorkspace, parse_javascript_suppresses};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Benchmark JavaScriptAdapter creation (GlobSet compilation).
fn bench_js_adapter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("js_adapter_creation");

    group.bench_function("JavaScriptAdapter::new", |b| {
        b.iter(|| black_box(JavaScriptAdapter::new()))
    });

    group.finish();
}

/// Benchmark JavaScript file classification.
fn bench_js_classify(c: &mut Criterion) {
    let js_adapter = JavaScriptAdapter::new();

    // Generate test paths
    let source_paths: Vec<PathBuf> =
        (0..1000).map(|i| PathBuf::from(format!("src/components/component_{}.tsx", i))).collect();
    let test_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("src/components/component_{}.test.tsx", i)))
        .collect();
    let node_modules_paths: Vec<PathBuf> =
        (0..1000).map(|i| PathBuf::from(format!("node_modules/pkg_{}/index.js", i))).collect();

    let mut group = c.benchmark_group("js_classify");

    group.bench_function("js_1k_source", |b| {
        b.iter(|| {
            for path in &source_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.bench_function("js_1k_test", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.bench_function("js_1k_node_modules_ignored", |b| {
        b.iter(|| {
            for path in &node_modules_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark fast-path extension checking for source files.
fn bench_js_fast_path(c: &mut Criterion) {
    let js_adapter = JavaScriptAdapter::new();

    // Mixed extensions: some JS/TS (fast path hit), some other (fast path miss)
    let mixed_paths: Vec<PathBuf> = ["tsx", "ts", "js", "jsx", "mjs", "mts", "css", "json", "md"]
        .iter()
        .cycle()
        .take(1000)
        .enumerate()
        .map(|(i, ext)| PathBuf::from(format!("src/file_{}.{}", i, ext)))
        .collect();

    // All JS/TS extensions (100% fast path hit)
    let js_only_paths: Vec<PathBuf> = ["tsx", "ts", "js", "jsx", "mjs", "mts"]
        .iter()
        .cycle()
        .take(1000)
        .enumerate()
        .map(|(i, ext)| PathBuf::from(format!("src/file_{}.{}", i, ext)))
        .collect();

    // Mixed ignore prefix paths
    let ignore_paths: Vec<PathBuf> = [
        "node_modules/pkg/index.js",
        "dist/bundle.js",
        "src/app.ts",
        "build/output.js",
        ".next/cache/data.json",
        "coverage/lcov.info",
        "lib/utils.ts",
    ]
    .iter()
    .cycle()
    .take(1000)
    .map(|s| PathBuf::from(*s))
    .collect();

    let mut group = c.benchmark_group("js_fast_path");

    group.bench_function("extension_check_1k_mixed", |b| {
        b.iter(|| {
            for path in &mixed_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.bench_function("extension_check_1k_js_only", |b| {
        b.iter(|| {
            for path in &js_only_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.bench_function("ignore_prefix_1k_mixed", |b| {
        b.iter(|| {
            for path in &ignore_paths {
                black_box(js_adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark JavaScript workspace detection.
fn bench_js_workspace_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("js_workspace_detection");

    let js_simple = fixture_path("js-simple");
    let js_monorepo = fixture_path("js-monorepo");

    if js_simple.exists() {
        group.bench_with_input(
            BenchmarkId::new("from_root", "js-simple"),
            &js_simple,
            |b, path| b.iter(|| black_box(JsWorkspace::from_root(path))),
        );
    }

    if js_monorepo.exists() {
        group.bench_with_input(
            BenchmarkId::new("from_root", "js-monorepo"),
            &js_monorepo,
            |b, path| b.iter(|| black_box(JsWorkspace::from_root(path))),
        );
    }

    group.finish();
}

/// Benchmark ESLint/Biome suppress parsing.
fn bench_js_suppress_parse(c: &mut Criterion) {
    // Content with various ESLint suppresses
    let content_with_eslint: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                "// eslint-disable-next-line no-unused-vars -- OK: intentional\nconst x = 1;\n"
                    .to_string()
            } else if i % 15 == 0 {
                "/* eslint-disable @typescript-eslint/no-explicit-any */\nfunction legacy(): any {}\n/* eslint-enable */\n"
                    .to_string()
            } else {
                format!("const value_{} = {};\n", i, i)
            }
        })
        .collect();

    // Content with Biome suppresses
    let content_with_biome: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                "// biome-ignore lint/suspicious/noExplicitAny: legacy code\nfunction legacy(): any {}\n"
                    .to_string()
            } else {
                format!("const value_{} = {};\n", i, i)
            }
        })
        .collect();

    let content_without: String =
        (0..100).map(|i| format!("const value_{} = {};\n", i, i)).collect();

    let mut group = c.benchmark_group("js_suppress_parse");

    group.bench_function("eslint_100_lines", |b| {
        b.iter(|| black_box(parse_javascript_suppresses(&content_with_eslint, None)))
    });

    group.bench_function("biome_100_lines", |b| {
        b.iter(|| black_box(parse_javascript_suppresses(&content_with_biome, None)))
    });

    group.bench_function("none_100_lines", |b| {
        b.iter(|| black_box(parse_javascript_suppresses(&content_without, None)))
    });

    // Larger file
    let large_eslint: String = content_with_eslint.repeat(10);
    group.bench_function("eslint_1000_lines", |b| {
        b.iter(|| black_box(parse_javascript_suppresses(&large_eslint, None)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_js_adapter_creation,
    bench_js_classify,
    bench_js_fast_path,
    bench_js_workspace_detection,
    bench_js_suppress_parse,
);
criterion_main!(benches);
