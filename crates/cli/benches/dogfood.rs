// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Dogfooding benchmarks - quench checking quench.
//!
//! These are the most important benchmarks as they represent
//! real-world performance on a real codebase.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::path::Path;
use std::process::Command;

fn quench_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn bench_dogfood_fast(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let root = quench_root();

    let mut group = c.benchmark_group("dogfood");

    // Fast mode (default) - target: <1s
    group.bench_function("fast", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("check")
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    // JSON output mode
    group.bench_function("fast_json", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "-o", "json"])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

fn bench_dogfood_individual_checks(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let root = quench_root();

    let mut group = c.benchmark_group("dogfood_checks");

    // cloc check alone
    group.bench_function("cloc_only", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--cloc", "--no-escapes", "--no-agents"])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    // escapes check alone
    group.bench_function("escapes_only", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--escapes", "--no-cloc", "--no-agents"])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    // agents check alone
    group.bench_function("agents_only", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--agents", "--no-cloc", "--no-escapes"])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

fn bench_dogfood_ci(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let root = quench_root();

    // CI mode - target: 1-5s
    c.bench_function("dogfood_ci", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--ci"])
                .current_dir(root)
                .output()
                .expect("quench should run")
        })
    });
}

criterion_group!(
    benches,
    bench_dogfood_fast,
    bench_dogfood_individual_checks,
    bench_dogfood_ci
);
criterion_main!(benches);
