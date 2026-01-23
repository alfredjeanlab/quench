// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Baseline benchmarks for quench performance tracking.
//!
//! These benchmarks establish performance baselines for:
//! - CLI startup time
//! - File walking (when implemented)
//! - Check execution (when implemented)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::process::Command;

/// Benchmark CLI startup time (no-op execution)
fn bench_cli_startup(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    c.bench_function("cli_startup", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("--help")
                .output()
                .expect("quench should run")
        })
    });
}

/// Benchmark version check (minimal work)
fn bench_version_check(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    c.bench_function("version_check", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("--version")
                .output()
                .expect("quench should run")
        })
    });
}

criterion_group!(benches, bench_cli_startup, bench_version_check);
criterion_main!(benches);
