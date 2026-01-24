// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command benchmarks.
//!
//! Measures report formatting performance across:
//! - Different output formats (text, JSON, HTML)
//! - Various baseline sizes (minimal to large)
//!
//! These benchmarks focus on formatter performance, not file I/O.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::io::Write;
use std::path::{Path, PathBuf};

use quench::Baseline;
use quench::cli::{OutputFormat, ReportArgs};
use quench::report::{format_report, format_report_to, format_report_with_options};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/bench-report")
        .join(name)
}

fn load_fixture_baseline(name: &str) -> Baseline {
    let path = fixture_path(name).join(".quench/baseline.json");
    Baseline::load(&path)
        .expect("baseline should load")
        .expect("fixture must exist")
}

/// Benchmark text format across all fixture sizes.
fn bench_text_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/text");

    for fixture in ["minimal", "typical", "comprehensive", "large-escapes"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: fixture not found at {path:?}");
            continue;
        }

        let baseline = load_fixture_baseline(fixture);
        let args = ReportArgs::default();

        group.bench_with_input(BenchmarkId::new("format", fixture), &baseline, |b, bl| {
            b.iter(|| format_report(OutputFormat::Text, Some(black_box(bl)), &args))
        });
    }
    group.finish();
}

/// Benchmark JSON format across all fixture sizes.
fn bench_json_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/json");

    for fixture in ["minimal", "typical", "comprehensive", "large-escapes"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        let baseline = load_fixture_baseline(fixture);
        let args = ReportArgs::default();

        group.bench_with_input(BenchmarkId::new("format", fixture), &baseline, |b, bl| {
            b.iter(|| format_report(OutputFormat::Json, Some(black_box(bl)), &args))
        });
    }
    group.finish();
}

/// Benchmark HTML format across all fixture sizes.
fn bench_html_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/html");

    for fixture in ["minimal", "typical", "comprehensive", "large-escapes"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        let baseline = load_fixture_baseline(fixture);
        let args = ReportArgs::default();

        group.bench_with_input(BenchmarkId::new("format", fixture), &baseline, |b, bl| {
            b.iter(|| format_report(OutputFormat::Html, Some(black_box(bl)), &args))
        });
    }
    group.finish();
}

/// Compare all formats on the typical fixture.
fn bench_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/format-comparison");

    let path = fixture_path("typical");
    if !path.exists() {
        eprintln!("Skipping format comparison: typical fixture not found");
        return;
    }

    let baseline = load_fixture_baseline("typical");
    let args = ReportArgs::default();

    for (format, name) in [
        (OutputFormat::Text, "text"),
        (OutputFormat::Json, "json"),
        (OutputFormat::Html, "html"),
    ] {
        group.bench_with_input(BenchmarkId::new("typical", name), &baseline, |b, bl| {
            b.iter(|| format_report(format, Some(black_box(bl)), &args))
        });
    }
    group.finish();
}

/// Benchmark streaming output vs. String allocation.
fn bench_streaming_vs_buffered(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/streaming");

    let path = fixture_path("large-escapes");
    if !path.exists() {
        eprintln!("Skipping streaming benchmark: large-escapes fixture not found");
        return;
    }

    let baseline = load_fixture_baseline("large-escapes");
    let args = ReportArgs::default();

    // Buffered (current): format to String, then write
    group.bench_function("buffered/html", |b| {
        b.iter(|| {
            let output =
                format_report(OutputFormat::Html, Some(black_box(&baseline)), &args).unwrap();
            std::io::sink().write_all(output.as_bytes()).unwrap();
        })
    });

    // Streaming: write directly to sink
    group.bench_function("streaming/html", |b| {
        b.iter(|| {
            format_report_to(
                &mut std::io::sink(),
                OutputFormat::Html,
                Some(black_box(&baseline)),
                &args,
                false,
            )
            .unwrap();
        })
    });

    // Buffered JSON
    group.bench_function("buffered/json", |b| {
        b.iter(|| {
            let output =
                format_report(OutputFormat::Json, Some(black_box(&baseline)), &args).unwrap();
            std::io::sink().write_all(output.as_bytes()).unwrap();
        })
    });

    // Streaming JSON
    group.bench_function("streaming/json", |b| {
        b.iter(|| {
            format_report_to(
                &mut std::io::sink(),
                OutputFormat::Json,
                Some(black_box(&baseline)),
                &args,
                false,
            )
            .unwrap();
        })
    });

    group.finish();
}

/// Benchmark compact vs. pretty JSON output.
fn bench_compact_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/json-compact");

    let path = fixture_path("large-escapes");
    if !path.exists() {
        eprintln!("Skipping compact benchmark: large-escapes fixture not found");
        return;
    }

    let baseline = load_fixture_baseline("large-escapes");
    let args = ReportArgs::default();

    group.bench_function("pretty", |b| {
        b.iter(|| {
            format_report_with_options(OutputFormat::Json, Some(black_box(&baseline)), &args, false)
        })
    });

    group.bench_function("compact", |b| {
        b.iter(|| {
            format_report_with_options(OutputFormat::Json, Some(black_box(&baseline)), &args, true)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_text_format,
    bench_json_format,
    bench_html_format,
    bench_format_comparison,
    bench_streaming_vs_buffered,
    bench_compact_json,
);
criterion_main!(benches);
