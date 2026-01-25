# Profiling Guide

How to identify and fix performance bottlenecks in quench.

## Automated Profiling Scripts

### Quick Start

```bash
# Run profiling (generates reports in reports/profiling/)
./scripts/perf/profile.sh tests/fixtures/stress-monorepo

# Generate flame graphs
./scripts/perf/flamegraph.sh tests/fixtures/stress-monorepo

# Check performance budgets
./scripts/perf/budget-check.sh
```

### Performance Budget Check

The `budget-check.sh` script enforces hard limits and is run in CI:

```bash
./scripts/perf/budget-check.sh
```

Outputs human-readable results with GitHub Actions annotations for CI.

### Performance Report

Generate a detailed performance report:

```bash
./scripts/perf/report-perf.sh reports/perf-$(date +%Y%m%d).md
```

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

## CI Benchmark Script

```bash
# Run benchmarks (outputs summary)
./scripts/bench-ci

# Save current results as baseline
./scripts/bench-ci --save-baseline

# Compare against saved baseline
./scripts/bench-ci --compare
```

## Performance Model

See `docs/specs/20-performance.md` for the performance model:
- File discovery: 30-50% of time
- File reading: 20-30% of time
- Pattern matching: 20-40% of time
- Output: <5% of time

### Performance Targets

| Mode | Time Target | Memory Target |
|------|-------------|---------------|
| fast | <1s | <100MB |
| ci | <5s | <500MB |

## Stress Test Fixtures

Generate stress fixtures for edge-case testing:

```bash
# Generate all stress fixtures
./scripts/fixtures/generate-stress-fixtures

# Available fixtures:
# - stress-huge-files: 50K tiny files (traversal stress)
# - stress-monorepo: 5K files in workspace (realistic large project)
# - stress-large-file: 5 files of 1-5MB (large file handling)
```

Run stress benchmarks:

```bash
./scripts/fixtures/generate-stress-fixtures
cargo bench --bench stress -- stress_e2e
```

## Optimization Priorities

1. **P0 (Done):** File-level caching
2. **P1 (Done):** Adaptive parallel/sequential file walking
3. **P2:** Pattern matching (when >50% of time)
4. **P3:** Memory constraints (when >100MB for fast mode)
5. **P4:** Micro-optimizations (string interning, arenas)

**Rule:** Measure first, optimize second. Profile before implementing P2+.

## Interpreting Results

### Criterion Output

```
check_fast_mode         time:   [18.91 ms 18.97 ms 19.03 ms]
                        change: [-0.4821% +0.2073% +0.9266%] (p = 0.56 > 0.05)
                        No change in performance detected.
```

- **time:** [lower bound, estimate, upper bound] with 95% confidence
- **change:** Percentage change from baseline (if comparing)
- **p-value:** Statistical significance (p < 0.05 means significant change)

### Regression Detection

Criterion flags changes as:
- **Performance has improved:** >10% faster
- **No change detected:** Within noise margin
- **Performance has regressed:** >10% slower

## Troubleshooting

### Benchmark Noise

If results vary significantly:
1. Close other applications
2. Disable CPU frequency scaling
3. Increase sample size: `cargo bench -- --sample-size 100`
4. Run multiple times and compare

### Memory Measurement Differences

External tools like `time -l` measure RSS (Resident Set Size), which includes:
- Heap allocations
- Stack
- Memory-mapped files
- Shared libraries

This is typically higher than allocator-tracked memory but more representative of actual resource usage.

### Large Fixture Generation

The stress fixture generator may need several minutes for 50K files:
```bash
# Generate with progress output
./scripts/fixtures/generate-stress-fixtures 2>&1 | tee fixture-gen.log
```

## Optimization History

### Checkpoint 17E: Performance Budget Enforcement

**Date:** 2025-01-24

**Summary:** Established performance budgets and regression prevention infrastructure.

**Changes:**
- Added `scripts/perf/profile.sh` for automated profiling
- Added `scripts/perf/flamegraph.sh` for flame graph generation
- Added `scripts/perf/budget-check.sh` for CI budget enforcement
- Added `crates/cli/benches/regression.rs` for hard limit tests
- Updated CI workflow to run budget checks

**Performance Baseline (bench-medium fixture):**

| Metric | Target | Acceptable | Measured |
|--------|--------|------------|----------|
| Cold run | < 500ms | < 1s | ~316ms |
| Warm run | < 100ms | < 200ms | ~47ms |
| Memory | < 100MB | < 500MB | ~14.5MB |

### Deferred Optimizations

1. **P3: Bounded cache (moka)**
   - **Status:** Deferred
   - **Reason:** Memory usage 14.5MB << 100MB target

2. **P4: String interning (lasso)**
   - **Status:** Deferred
   - **Reason:** No allocation bottleneck identified

### Profiling Methodology

1. **Baseline first** - Profile before any changes to identify real bottlenecks
2. **Measure twice** - Run profiling multiple times to ensure consistency
3. **Fix one thing** - Apply one optimization, measure, commit, repeat
4. **Document decisions** - Record why optimizations were applied or deferred
