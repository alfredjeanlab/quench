# Checkpoint 6E: Performance - Dogfooding Milestone 1

**Root Feature:** `quench-2bcc`

## Overview

Building on Checkpoint 6D's benchmark baselines, this checkpoint enhances performance observability with memory profiling, expanded stress testing, and CI integration for automated regression detection. While current performance exceeds targets by 50-1500x, establishing proper monitoring infrastructure ensures regressions are caught early as features are added.

Key goals:
1. **Memory profiling** - Establish memory baselines and tracking infrastructure
2. **Stress test expansion** - Add end-to-end stress fixtures that exercise the full pipeline
3. **CI integration** - Automated benchmark regression detection in CI
4. **Profiling documentation** - Guide for identifying future bottlenecks

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── stress.rs         # UPDATE: Add end-to-end stress tests
│   │   └── memory.rs         # NEW: Memory profiling benchmarks
│   └── src/
│       └── lib.rs            # May need instrumentation hooks
├── tests/fixtures/
│   ├── stress-huge-files/    # NEW: 50K+ file stress test
│   ├── stress-monorepo/      # NEW: Simulated monorepo (5K files)
│   └── stress-large-file/    # NEW: 1-5MB file edge case
├── scripts/
│   ├── generate-stress-fixtures  # NEW: Generate stress test fixtures
│   └── bench-ci               # NEW: CI benchmark runner with baseline comparison
├── reports/
│   └── benchmark-milestone-1.md  # UPDATE: Add memory metrics
├── docs/
│   └── profiling.md          # NEW: Profiling guide
└── .github/workflows/
    └── bench.yml             # NEW: CI benchmark workflow
```

## Dependencies

**Existing:**
- `criterion` - Benchmark framework (already in dev-dependencies)
- `ignore` crate - File walking (already in dependencies)

**New (optional):**
- `peak_alloc` or custom allocator wrapper - Memory high-water mark tracking
- `jemalloc` with profiling - Heap profiling (optional, dev only)

No runtime dependencies added. Memory profiling uses Rust's `GlobalAlloc` trait.

## Implementation Phases

### Phase 1: Memory Profiling Infrastructure

**Goal:** Track memory high-water mark during benchmarks.

The performance spec targets <100MB for fast checks and <500MB for CI checks. Currently there's no memory measurement.

**Implementation approach:**

Create a simple allocator wrapper that tracks peak allocation:

```rust
// crates/cli/src/alloc.rs (dev feature only)
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TrackingAllocator;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let current = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(current, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

pub fn peak_memory_mb() -> f64 {
    PEAK.load(Ordering::Relaxed) as f64 / 1_048_576.0
}

pub fn reset_peak() {
    PEAK.store(CURRENT.load(Ordering::Relaxed), Ordering::Relaxed);
}
```

**Benchmark file:** `crates/cli/benches/memory.rs`

```rust
//! Memory profiling benchmarks.
//!
//! Measures peak memory usage for various workloads.

use criterion::{Criterion, criterion_group, criterion_main};
use std::process::Command;

fn bench_memory_fast(c: &mut Criterion) {
    // Use /usr/bin/time -l on macOS or /usr/bin/time -v on Linux
    // to measure RSS externally, avoiding allocator complexity

    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let root = quench_root();

    c.bench_function("memory_fast", |b| {
        b.iter(|| {
            // Criterion measures time; we'll capture memory separately
            Command::new(quench_bin)
                .arg("check")
                .current_dir(root)
                .output()
        })
    });
}
```

**Simpler approach:** Use external memory measurement (`/usr/bin/time -l` on macOS, `time -v` on Linux) in CI scripts rather than instrumenting the binary.

**Verification:**
```bash
/usr/bin/time -l cargo run --release -- check 2>&1 | grep 'maximum resident'
# Should show <100MB for fast mode
```

### Phase 2: Stress Test Fixtures

**Goal:** Create disk-based fixtures that exercise edge cases from the performance spec.

The existing `stress.rs` benchmarks use in-memory generated content. For realistic testing, we need actual disk fixtures.

**Fixtures to create:**

| Fixture | Files | Size | Purpose |
|---------|-------|------|---------|
| `stress-huge-files` | 50K | ~500KB total | Large file count traversal |
| `stress-monorepo` | 5K | ~50K LOC | Realistic monorepo simulation |
| `stress-large-file` | 5 | 1-5MB each | Large file handling |
| `stress-patterns` | 100 | ~10K LOC | Many escape patterns to match |

**Generator script:** `scripts/generate-stress-fixtures`

```bash
#!/bin/bash
set -euo pipefail

FIXTURE_DIR="tests/fixtures"

# stress-huge-files: 50K tiny files in flat structure
generate_huge_files() {
    local dir="$FIXTURE_DIR/stress-huge-files"
    mkdir -p "$dir/src"
    cat > "$dir/Cargo.toml" << 'EOF'
[package]
name = "stress-huge-files"
version = "0.1.0"
edition = "2021"
EOF

    # Generate 50K stub files
    for i in $(seq 1 50000); do
        echo "pub fn f$i() {}" > "$dir/src/f$i.rs"
    done

    # Create lib.rs that mods them all
    for i in $(seq 1 50000); do
        echo "mod f$i;"
    done > "$dir/src/lib.rs"
}

# stress-monorepo: 5K files in workspace structure
generate_monorepo() {
    local dir="$FIXTURE_DIR/stress-monorepo"
    mkdir -p "$dir"
    # ... workspace with 10 crates, 500 files each
}

# stress-large-file: Few files, each 1-5MB
generate_large_files() {
    local dir="$FIXTURE_DIR/stress-large-file"
    mkdir -p "$dir/src"
    # Generate 5 files with 20K-50K lines each
}
```

**Important:** Add fixtures to `.gitignore` - they're generated on-demand, not checked in:

```gitignore
# Generated stress test fixtures
tests/fixtures/stress-huge-files/
tests/fixtures/stress-monorepo/
tests/fixtures/stress-large-file/
```

**Verification:**
```bash
./scripts/generate-stress-fixtures
ls tests/fixtures/stress-*/
```

### Phase 3: End-to-End Stress Benchmarks

**Goal:** Add criterion benchmarks that run quench against stress fixtures.

**Update:** `crates/cli/benches/stress.rs`

Add new benchmark group for end-to-end stress tests:

```rust
/// End-to-end stress benchmarks using disk fixtures.
fn bench_stress_e2e(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_e2e");
    group.sample_size(10); // Fewer samples for slow benchmarks

    let quench_bin = env!("CARGO_BIN_EXE_quench");

    // Only run if fixtures exist (generated on-demand)
    let fixtures = ["stress-huge-files", "stress-monorepo", "stress-large-file"];

    for fixture in fixtures {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: run ./scripts/generate-stress-fixtures");
            continue;
        }

        group.bench_function(fixture, |b| {
            b.iter(|| {
                Command::new(quench_bin)
                    .arg("check")
                    .current_dir(&path)
                    .output()
                    .expect("quench should run")
            })
        });
    }

    group.finish();
}
```

**Performance targets for stress tests:**

| Fixture | Target | Rationale |
|---------|--------|-----------|
| stress-huge-files (50K) | <30s | Spec allows 30s max for pathological cases |
| stress-monorepo (5K) | <10s | 10x larger than "large project" spec |
| stress-large-file (5MB) | <5s | Should skip >10MB, process 1-5MB |

**Verification:**
```bash
./scripts/generate-stress-fixtures
cargo bench --bench stress -- stress_e2e
```

### Phase 4: CI Benchmark Integration

**Goal:** Automatically detect performance regressions in CI.

**Approach:** Use criterion's baseline comparison with a script that fails on significant regression.

**File:** `scripts/bench-ci`

```bash
#!/bin/bash
set -euo pipefail

# Run benchmarks comparing against stored baseline
# Fails if any benchmark regresses >20%

BASELINE="milestone-1"
THRESHOLD_PERCENT=20

echo "Running benchmarks against baseline: $BASELINE"

# Run criterion benchmarks with baseline comparison
cargo bench --bench dogfood -- --baseline "$BASELINE" --noplot 2>&1 | tee bench-output.txt

# Parse output for regressions
# Criterion outputs: "Performance has regressed" for >+10% changes
if grep -q "Performance has regressed" bench-output.txt; then
    echo "ERROR: Performance regression detected!"
    grep -A5 "Performance has regressed" bench-output.txt
    exit 1
fi

echo "No significant regressions detected"
```

**GitHub workflow:** `.github/workflows/bench.yml`

```yaml
name: Benchmarks

on:
  pull_request:
    paths:
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Restore baseline
        uses: actions/cache@v4
        with:
          path: target/criterion
          key: criterion-baseline-${{ github.base_ref }}

      - name: Run benchmarks
        run: ./scripts/bench-ci

      - name: Save baseline
        if: github.ref == 'refs/heads/main'
        uses: actions/cache@v4
        with:
          path: target/criterion
          key: criterion-baseline-main-${{ github.sha }}
```

**Verification:**
```bash
./scripts/bench-ci
```

### Phase 5: Documentation and Profiling Guide

**Goal:** Document how to profile quench for future optimization work.

**File:** `docs/profiling.md`

```markdown
# Profiling Guide

How to identify and fix performance bottlenecks in quench.

## Quick Start

### Time Profiling

**macOS (Instruments):**
```bash
xcrun xctrace record --template 'Time Profiler' \
    --launch ./target/release/quench check /path/to/repo
```

**Linux (perf):**
```bash
perf record -g ./target/release/quench check /path/to/repo
perf report
```

**Flame graphs (cross-platform):**
```bash
cargo install flamegraph
flamegraph -- ./target/release/quench check /path/to/repo
```

### Memory Profiling

**macOS:**
```bash
/usr/bin/time -l ./target/release/quench check 2>&1 | grep 'maximum resident'
```

**Linux:**
```bash
/usr/bin/time -v ./target/release/quench check 2>&1 | grep 'Maximum resident'
```

**Heap profiling with jemalloc:**
```bash
MALLOC_CONF=prof:true,prof_prefix:jeprof.out \
    cargo run --release --features jemalloc-prof -- check
jeprof --svg target/release/quench jeprof.out.*.heap > heap.svg
```

## Benchmark Commands

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench --bench dogfood

# Compare against baseline
cargo bench --bench dogfood -- --baseline milestone-1

# Save new baseline
cargo bench --bench dogfood -- --save-baseline milestone-2
```

## Performance Model

See `docs/specs/20-performance.md` for the performance model:
- File discovery: 30-50% of time
- File reading: 20-30% of time
- Pattern matching: 20-40% of time
- Output: <5% of time

## Optimization Priorities

1. **P0 (Done):** File-level caching
2. **P1 (Done):** Adaptive parallel/sequential file walking
3. **P2:** Pattern matching (when >50% of time)
4. **P3:** Memory constraints (when >100MB for fast mode)
5. **P4:** Micro-optimizations (string interning, arenas)

**Rule:** Measure first, optimize second. Profile before implementing P2+.
```

**Verification:**
```bash
cat docs/profiling.md
```

### Phase 6: Final Verification and Report Update

**Goal:** Verify all changes and update the benchmark report.

**Steps:**

1. Run `make check` to verify tests pass
2. Generate stress fixtures and run stress benchmarks
3. Measure memory usage
4. Update `reports/benchmark-milestone-1.md` with memory metrics

**Update report with memory section:**

```markdown
### Memory Usage

| Mode | Target | Measured | Status |
|------|--------|----------|--------|
| fast | <100MB | X.X MB | PASS/FAIL |
| ci | <500MB | X.X MB | PASS/FAIL |

### Stress Test Results

| Fixture | Files | Target | Measured | Status |
|---------|-------|--------|----------|--------|
| stress-huge-files | 50K | <30s | X.Xs | PASS/FAIL |
| stress-monorepo | 5K | <10s | X.Xs | PASS/FAIL |
| stress-large-file | 5×5MB | <5s | X.Xs | PASS/FAIL |
```

**Verification:**
```bash
make check
./scripts/generate-stress-fixtures
cargo bench --bench stress -- stress_e2e
/usr/bin/time -l cargo run --release -- check 2>&1 | grep -i resident
```

## Key Implementation Details

### Memory Measurement Strategy

Rather than instrumenting the allocator (which adds complexity and overhead), use external tools:

- **macOS:** `/usr/bin/time -l` reports `maximum resident set size`
- **Linux:** `/usr/bin/time -v` reports `Maximum resident set size`
- **CI:** Capture these in benchmark scripts for tracking

This approach:
- Zero runtime overhead
- Works on release builds
- Measures actual RSS, not just allocator tracking
- Simpler to implement and maintain

### Stress Fixture Trade-offs

**Checked in vs Generated:**
- Generated fixtures avoid bloating the repo (50K files = large)
- Generated on CI means deterministic but requires setup step
- Compromise: Small stress fixtures checked in, large ones generated

**Fixture size selection:**
- 50K files: Tests file walking at scale (monorepo edge case)
- 5K files: Tests realistic large project
- 5MB files: Tests mmap path and large file handling

### CI Regression Detection

Criterion's built-in regression detection:
- Flags changes >10% as "regressed" or "improved"
- Statistical confidence from multiple samples
- Baseline comparison avoids noise from machine variability

**Threshold choice:** 20% regression triggers failure
- Catches real regressions
- Allows minor fluctuations from CI variance
- Can be tightened as baseline stabilizes

## Verification Plan

### Phase 1 Verification
```bash
# Memory measurement works
/usr/bin/time -l cargo run --release -- check 2>&1 | grep -i resident
# Should output memory in KB
```

### Phase 2 Verification
```bash
# Stress fixtures generate correctly
./scripts/generate-stress-fixtures
ls tests/fixtures/stress-*/
find tests/fixtures/stress-huge-files -name "*.rs" | wc -l
# Should show ~50K files
```

### Phase 3 Verification
```bash
# Stress benchmarks run
cargo bench --bench stress -- stress_e2e --test
# Should list stress test benchmarks
```

### Phase 4 Verification
```bash
# CI script works
./scripts/bench-ci
# Should complete without regression errors
```

### Phase 5 Verification
```bash
# Documentation exists and is valid
test -f docs/profiling.md && echo "Profiling guide exists"
```

### Phase 6 (Final) Verification
```bash
make check
cargo bench --bench dogfood
cat reports/benchmark-milestone-1.md | grep -A5 "Memory Usage"
```

## Exit Criteria

- [ ] Memory measurement infrastructure in place
- [ ] Stress fixtures generated: stress-huge-files, stress-monorepo, stress-large-file
- [ ] Stress benchmarks added to `benches/stress.rs`
- [ ] CI benchmark script created: `scripts/bench-ci`
- [ ] GitHub workflow for benchmark regression detection
- [ ] Profiling guide documented: `docs/profiling.md`
- [ ] Benchmark report updated with memory metrics
- [ ] All tests pass: `make check`
