# Checkpoint 6D: Benchmark - Dogfooding Milestone 1

**Root Feature:** `quench-2bcc`

## Overview

This checkpoint establishes performance benchmarking for quench by running it on itself (dogfooding). While Checkpoint 6C validated correctness, this checkpoint validates performance and establishes baselines for regression tracking.

Key goals:
1. **Fix cloc metrics** - The dogfooding report shows 0 files detected; quench's Rust source files in `crates/cli/src/` aren't being counted
2. **Create benchmark fixtures** - The criterion benchmarks reference fixtures that don't exist
3. **Establish baselines** - Run quench on itself and record performance metrics
4. **Validate performance goals** - Sub-second for fast checks per the spec

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── check.rs          # Update to use real fixtures
│   │   ├── dogfood.rs        # NEW: Run quench on quench
│   │   └── baseline.rs       # Existing baseline benchmarks
│   └── src/
│       ├── adapter/
│       │   └── rust/         # May need fixes for workspace detection
│       └── checks/
│           └── cloc.rs       # May need fixes for nested source detection
├── tests/fixtures/
│   ├── bench-small/          # NEW: ~10 files, ~500 LOC
│   ├── bench-medium/         # NEW: ~50 files, ~5k LOC
│   ├── bench-large/          # NEW: ~200 files, ~20k LOC
│   ├── bench-deep/           # NEW: 10-level deep directories
│   └── bench-large-files/    # NEW: Few files, 1000+ lines each
├── reports/
│   └── benchmark-milestone-1.md  # NEW: Performance report
└── quench.toml               # Update to detect crates/cli/src/
```

## Dependencies

No new external dependencies. Uses existing infrastructure:
- `criterion` crate for benchmarks (already in dev-dependencies)
- `hyperfine` CLI for wall-clock measurements (external, optional)
- Existing test fixtures and benchmark harness

## Implementation Phases

### Phase 1: Diagnose and Fix cloc Metrics

**Goal:** Fix the cloc check to properly count quench's own source files.

**Problem:** Dogfooding report shows:
```json
"cloc": { "source_files": 0, "source_lines": 0 }
```

But quench has substantial source code in `crates/cli/src/`.

**Investigation tasks:**
1. Run `quench check -o json` on quench root and examine output
2. Check if Rust adapter is detecting `crates/cli/` as a Rust package
3. Verify source patterns match `crates/cli/src/**/*.rs`
4. Check if workspace detection is working correctly

**Likely fixes:**
- Add explicit source patterns in `quench.toml` for nested crates
- Or fix adapter's workspace member detection to recursively find Cargo.toml files

**quench.toml update:**
```toml
[check.cloc]
# Explicitly point to source locations for nested workspace
source = ["crates/cli/src/**/*.rs"]
test = ["crates/cli/src/**/*_tests.rs", "tests/**/*.rs"]
exclude = ["tests/fixtures/cloc/**"]
```

**Verification:**
```bash
cargo run -- check -o json | jq '.checks[] | select(.name == "cloc") | .metrics'
# Should show non-zero source_files and source_lines
```

### Phase 2: Create Benchmark Fixtures

**Goal:** Create benchmark fixtures referenced by `benches/check.rs`.

**Fixtures to create:**

| Fixture | Files | Lines | Structure |
|---------|-------|-------|-----------|
| `bench-small` | ~10 | ~500 | Flat Rust project |
| `bench-medium` | ~50 | ~5k | Multi-module Rust |
| `bench-large` | ~200 | ~20k | Workspace with 3 crates |
| `bench-deep` | ~30 | ~1k | 10-level deep nesting |
| `bench-large-files` | ~5 | ~5k | Large individual files |

**Structure for bench-small:**
```
bench-small/
├── Cargo.toml
├── src/
│   ├── main.rs         # ~50 lines
│   ├── lib.rs          # ~100 lines
│   ├── parser.rs       # ~150 lines
│   └── output.rs       # ~100 lines
└── tests/
    └── integration.rs  # ~100 lines
```

**Structure for bench-large:**
```
bench-large/
├── Cargo.toml          # Workspace
├── crates/
│   ├── core/           # ~80 files, ~8k LOC
│   ├── cli/            # ~60 files, ~6k LOC
│   └── utils/          # ~60 files, ~6k LOC
└── tests/
    └── integration/    # ~20 files, ~2k LOC
```

**Generation approach:** Create a script that generates synthetic but realistic Rust code:
- Modules with functions, structs, impls
- Realistic naming patterns
- Some test files with `_tests.rs` suffix
- Some files close to 750 line limit

**File:** `scripts/generate-bench-fixtures`
```bash
#!/bin/bash
# Generate benchmark fixtures for criterion tests
```

**Verification:**
```bash
ls tests/fixtures/bench-*/
cargo bench --bench check -- --test  # Verify fixtures are found
```

### Phase 3: Add Dogfooding Benchmark

**Goal:** Create a benchmark that measures quench running on the quench codebase itself.

**File:** `crates/cli/benches/dogfood.rs`

```rust
//! Dogfooding benchmarks - quench checking quench.
//!
//! These are the most important benchmarks as they represent
//! real-world performance on a real codebase.

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

    // Individual checks
    for check in ["cloc", "escapes", "agents"] {
        group.bench_function(format!("only_{check}"), |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .args(["check", &format!("--{check}"), "--no-cloc", "--no-escapes", "--no-agents"])
                    .arg(format!("--{check}"))
                    .current_dir(root)
                    .output()
                    .expect("quench should run")
            })
        });
    }

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

criterion_group!(benches, bench_dogfood_fast, bench_dogfood_ci);
criterion_main!(benches);
```

**Register in Cargo.toml:**
```toml
[[bench]]
name = "dogfood"
harness = false
```

**Verification:**
```bash
cargo bench --bench dogfood
```

### Phase 4: Run Benchmarks and Collect Baselines

**Goal:** Execute all benchmarks and record baseline metrics.

**Steps:**
1. Run criterion benchmarks to get statistical measurements
2. Run hyperfine for wall-clock time (human-readable)
3. Record metrics in report

**Commands:**
```bash
# Criterion benchmarks (statistical)
cargo bench --bench dogfood -- --save-baseline milestone-1
cargo bench --bench check -- --save-baseline milestone-1
cargo bench --bench baseline -- --save-baseline milestone-1

# Hyperfine for wall-clock (optional, if installed)
hyperfine --warmup 3 'cargo run --release -- check' \
    --export-json reports/hyperfine-dogfood.json
```

**Metrics to collect:**
| Metric | Target | Measurement |
|--------|--------|-------------|
| Fast mode | <1s | criterion mean |
| CI mode | <5s | criterion mean |
| CLI startup | <50ms | criterion |
| cloc check alone | <200ms | criterion |
| escapes check alone | <200ms | criterion |
| agents check alone | <100ms | criterion |

**Verification:**
```bash
# Compare against targets
cargo bench --bench dogfood 2>&1 | grep -E 'time:|mean'
```

### Phase 5: Create Benchmark Report

**Goal:** Document findings in a structured report.

**File:** `reports/benchmark-milestone-1.md`

```markdown
# Benchmark Milestone 1 Report

Date: YYYY-MM-DD

## Summary

First performance benchmark milestone - measuring quench on itself.

## Environment

- Hardware: [CPU, RAM]
- OS: [version]
- Rust: [version]
- quench commit: [hash]

## Results

### Dogfooding (quench on quench)

| Mode | Target | Measured | Status |
|------|--------|----------|--------|
| fast | <1s | X.XXs | PASS/FAIL |
| fast (json) | <1s | X.XXs | PASS/FAIL |
| ci | <5s | X.XXs | PASS/FAIL |

### Individual Checks

| Check | Target | Measured |
|-------|--------|----------|
| cloc | <200ms | X.XXms |
| escapes | <200ms | X.XXms |
| agents | <100ms | X.XXms |

### Fixture Benchmarks

| Fixture | Files | Lines | Time |
|---------|-------|-------|------|
| bench-small | ~10 | ~500 | X.XXms |
| bench-medium | ~50 | ~5k | X.XXms |
| bench-large | ~200 | ~20k | X.XXs |

## Observations

- [Key findings about performance]
- [Bottlenecks identified]
- [Comparison to targets]

## Baseline Established

Criterion baselines saved as `milestone-1` for regression tracking.

## Next Steps

- [Optimization opportunities identified]
- [Areas for Phase 2 investigation]
```

**Verification:**
```bash
cat reports/benchmark-milestone-1.md
```

### Phase 6: Final Verification

**Goal:** Ensure all changes pass CI and benchmarks are working.

**Steps:**
1. Run `make check` to verify all tests pass
2. Verify cloc now shows correct metrics for quench
3. Verify all benchmark fixtures exist and are used
4. Verify dogfood benchmark runs and completes

**Commands:**
```bash
# Full CI check
make check

# Verify cloc metrics fixed
cargo run -- check -o json | jq '.checks[] | select(.name == "cloc")'

# Verify benchmarks run
cargo bench --bench dogfood -- --test
cargo bench --bench check -- --test

# Final dogfood run
cargo run -- check
```

## Key Implementation Details

### Rust Workspace Detection

The cloc issue likely stems from workspace detection. When quench runs at the repo root:
1. It should detect `Cargo.toml` as a workspace
2. Find workspace members in `crates/*/`
3. Treat `crates/cli/src/` as source, `crates/cli/src/*_tests.rs` as test

If the adapter doesn't handle workspaces, explicit paths in `quench.toml` provide a workaround.

### Benchmark Fixture Generation

Fixtures should be:
- Deterministic (reproducible)
- Realistic (actual Rust syntax)
- Varied (different patterns to exercise all code paths)
- Excluded from cloc checks (in `quench.toml`)

### Criterion Configuration

For accurate benchmarks, configure criterion appropriately:
```toml
# Cargo.toml
[profile.bench]
debug = true  # For profiling

# In bench files
.measurement_time(Duration::from_secs(10))
.sample_size(50)
```

### Performance Targets (from spec)

| Metric | Target | Rationale |
|--------|--------|-----------|
| Fast checks | <1s | Spec: "sub-second for fast checks" |
| CI checks | <5s | Spec: "1-5s for full checks" |
| Max | <30s | Spec: "Unacceptable: anything over 30s" |

## Verification Plan

### Phase 1 Verification
```bash
cargo run -- check -o json | jq '.checks[] | select(.name == "cloc") | .metrics.source_files'
# Should be > 0
```

### Phase 2 Verification
```bash
ls -la tests/fixtures/bench-*/
find tests/fixtures/bench-* -name "*.rs" | wc -l
# Should find expected file counts
```

### Phase 3 Verification
```bash
cargo bench --bench dogfood -- --list
# Should show: dogfood/fast, dogfood/fast_json, dogfood_ci, etc.
```

### Phase 4 Verification
```bash
ls target/criterion/
# Should have benchmark data directories
```

### Phase 5 Verification
```bash
test -f reports/benchmark-milestone-1.md && echo "Report exists"
```

### Phase 6 (Final) Verification
```bash
make check
cargo run -- check  # Should pass with real metrics
```

## Exit Criteria

- [ ] cloc check shows non-zero source_files for quench itself
- [ ] Benchmark fixtures exist: bench-small, bench-medium, bench-large, bench-deep, bench-large-files
- [ ] Dogfood benchmark implemented and running
- [ ] Criterion baselines established
- [ ] Performance within targets (<1s fast, <5s CI)
- [ ] Report created: `reports/benchmark-milestone-1.md`
- [ ] All tests pass: `make check`
