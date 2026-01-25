# Checkpoint 11D: Benchmark - Tests CI Mode

## Overview

Add benchmark coverage for the tests check CI mode functionality. The existing `benches/tests.rs` focuses on correlation detection (test file matching), but CI mode metrics collection (running tests, parsing output, coverage/timing aggregation) is not benchmarked. This checkpoint adds benchmarks for CI mode metrics overhead and establishes performance baselines.

## Project Structure

```
quench/
├── crates/cli/benches/
│   ├── tests.rs                      # UNCHANGED: Correlation benchmarks
│   └── tests_ci.rs                   # NEW: CI mode metrics benchmarks
├── reports/
│   └── benchmark-baseline.json       # MODIFIED: Add tests CI baselines
├── scripts/
│   └── benchmark                     # UNCHANGED: Runs dogfood benchmarks
└── tests/fixtures/
    └── tests-ci/                     # UNCHANGED: Minimal CI test fixture
```

## Dependencies

No new dependencies. Uses existing criterion and test infrastructure.

## Implementation Phases

### Phase 1: Create CI Mode Benchmark File

**Goal:** Create `benches/tests_ci.rs` with benchmark scaffolding.

**File:** `crates/cli/benches/tests_ci.rs`

```rust
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

// ... benchmark functions added in subsequent phases

criterion_group!(benches, /* functions */);
criterion_main!(benches);
```

**Update:** `crates/cli/Cargo.toml` to add the benchmark:

```toml
[[bench]]
name = "tests_ci"
harness = false
```

**Verification:**
```bash
cargo build --bench tests_ci
```

### Phase 2: Add tests-ci Fixture Benchmarks

**Goal:** Benchmark the tests check on the `tests-ci` fixture in both fast and CI modes.

**Benchmarks to add:**

```rust
fn bench_tests_ci_fixture(c: &mut Criterion) {
    let quench_bin = quench_bin();
    let path = fixture_path("tests-ci");

    let mut group = c.benchmark_group("tests_ci");
    group.sample_size(20); // Fewer samples - runs actual cargo test

    // Fast mode (no metrics collection)
    group.bench_function("fast", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents"])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    // CI mode (full metrics collection)
    group.bench_function("ci", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents", "--ci"])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    // CI mode JSON output
    group.bench_function("ci_json", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents", "--ci", "-o", "json"])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}
```

**Verification:**
```bash
cargo bench --bench tests_ci -- tests_ci
```

### Phase 3: Add CI Mode Overhead Measurement

**Goal:** Measure the overhead of CI mode metrics collection vs fast mode.

**Benchmarks to add:**

```rust
/// Measure overhead of --ci flag on dogfood (quench repo).
fn bench_ci_mode_overhead(c: &mut Criterion) {
    let quench_bin = quench_bin();
    let root = quench_root();

    let mut group = c.benchmark_group("ci_overhead");

    // Tests check fast mode
    group.bench_function("tests_fast", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents"])
                .current_dir(&root)
                .output()
                .expect("quench should run")
        })
    });

    // Tests check CI mode
    group.bench_function("tests_ci", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents", "--ci"])
                .current_dir(&root)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

fn quench_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}
```

**Verification:**
```bash
cargo bench --bench tests_ci -- ci_overhead
```

### Phase 4: Add Metrics Parsing Benchmarks

**Goal:** Benchmark the internal metrics parsing without running actual tests.

**Benchmarks to add:**

```rust
use quench::checks::tests::runners::cargo::parse_cargo_test_output;
use quench::checks::tests::metrics::TestMetrics;

/// Benchmark cargo test output parsing.
fn bench_metrics_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests_metrics_parsing");

    // Small test output (10 tests)
    let small_output = generate_cargo_test_output(10);
    group.bench_function("small_10_tests", |b| {
        b.iter(|| parse_cargo_test_output(&small_output))
    });

    // Medium test output (100 tests)
    let medium_output = generate_cargo_test_output(100);
    group.bench_function("medium_100_tests", |b| {
        b.iter(|| parse_cargo_test_output(&medium_output))
    });

    // Large test output (1000 tests)
    let large_output = generate_cargo_test_output(1000);
    group.bench_function("large_1000_tests", |b| {
        b.iter(|| parse_cargo_test_output(&large_output))
    });

    group.finish();
}

fn generate_cargo_test_output(test_count: usize) -> String {
    let mut output = String::from("running {} tests\n");
    for i in 0..test_count {
        output.push_str(&format!("test test_{} ... ok\n", i));
    }
    output.push_str(&format!(
        "\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s\n",
        test_count
    ));
    output
}
```

**Note:** The actual function names may differ based on the implementation. Adjust imports as needed after checking `src/checks/tests/`.

**Verification:**
```bash
cargo bench --bench tests_ci -- tests_metrics_parsing
```

### Phase 5: Add Regression Tests

**Goal:** Add hard time limits for CI mode performance.

**File:** `crates/cli/benches/regression.rs` (append to existing)

```rust
/// CI mode on tests-ci fixture must complete within 30s.
///
/// This is a conservative limit since it runs `cargo test` which
/// compiles and runs tests. The 30s limit matches the "unacceptable"
/// threshold from docs/specs/20-performance.md.
#[test]
fn tests_ci_mode_under_30s() {
    let path = fixture_path("tests-ci");
    if !path.exists() {
        eprintln!("Skipping: tests-ci fixture not found");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        eprintln!("Run: cargo build --release");
        return;
    }

    let start = Instant::now();
    let output = Command::new(&bin)
        .args(["check", "--tests", "--ci"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    eprintln!("Tests CI mode time: {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(30),
        "Tests CI mode took {:?}, exceeds 30s limit",
        elapsed
    );

    // Should complete successfully (tests pass)
    assert!(
        output.status.code().unwrap_or(-1) <= 1,
        "Unexpected exit code: {:?}",
        output.status
    );
}
```

**Verification:**
```bash
cargo test --bench regression tests_ci_mode
```

### Phase 6: Update Baseline and Verify

**Goal:** Run benchmarks, update baseline, verify all pass.

**Steps:**

1. Run all benchmarks:
```bash
cargo bench --bench tests_ci
```

2. Update baseline if needed (manual review):
```bash
# After confirming results are reasonable
./scripts/update-baseline
```

3. Run full verification:
```bash
# All tests
cargo test --all

# All benchmarks compile
cargo build --benches

# Full check
make check
```

**Verification:**
```bash
make check
```

## Key Implementation Details

### Benchmark Groups

| Group | Purpose | Sample Size |
|-------|---------|-------------|
| `tests_ci` | Fixture benchmarks | 20 (slow) |
| `ci_overhead` | Fast vs CI comparison | 10 (slow) |
| `tests_metrics_parsing` | Internal parsing | 100 (fast) |

### Expected Performance Characteristics

Based on performance targets from `docs/specs/20-performance.md`:

| Operation | Target | Acceptable | Unacceptable |
|-----------|--------|------------|--------------|
| Tests CI fixture | < 5s | < 15s | > 30s |
| CI overhead vs fast | < 50% | < 100% | > 200% |
| Metrics parsing (1k tests) | < 10ms | < 50ms | > 100ms |

### Why tests-ci Fixture

The `tests-ci` fixture created in checkpoint 11b is ideal for benchmarking:
- Minimal Cargo project with one passing test
- Configured with coverage and timing thresholds
- Deterministic output
- Fast compilation

### What's Not Benchmarked (Out of Scope)

- Coverage collection (requires `cargo-llvm-cov` installed)
- Multi-suite metrics (would need larger fixtures)
- Test runner discovery (already benchmarked in `benches/tests.rs`)

## Verification Plan

| Step | Command | Expected Result |
|------|---------|-----------------|
| Build compiles | `cargo build --bench tests_ci` | Success |
| Benchmarks run | `cargo bench --bench tests_ci` | Results printed |
| Regression test | `cargo test --bench regression tests_ci` | Passes |
| Full suite | `cargo test --all` | All pass |
| Lint check | `make check` | All checks pass |

## Completion Criteria

- [ ] `benches/tests_ci.rs` created with CI mode benchmarks
- [ ] `Cargo.toml` updated with new benchmark
- [ ] Fixture benchmarks measure fast vs CI mode
- [ ] CI overhead benchmarks compare modes
- [ ] Metrics parsing benchmarks measure internal functions
- [ ] Regression test added with 30s limit
- [ ] All benchmarks run successfully
- [ ] `make check` passes
- [ ] Changes committed
- [ ] `./done` executed
