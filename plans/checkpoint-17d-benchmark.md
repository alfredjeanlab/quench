# Checkpoint 17D: Benchmark - Performance

**Plan:** `checkpoint-17d-benchmark`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17C (Performance Refactor)

## Overview

Implement comprehensive performance benchmarking infrastructure to detect regressions, validate performance targets, and track metrics over time. The focus is on measurable verification of the performance characteristics specified in `docs/specs/20-performance.md`.

**Performance Targets (from spec):**

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Cold run | < 500ms | < 1s | > 2s |
| Warm run | < 100ms | < 200ms | > 500ms |
| CI checks | < 5s | < 15s | > 30s |

**Primary Use Case:** Warm runs (iterative agent development) are the common case. Benchmarks must emphasize this scenario.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── cache.rs            # NEW: Warm cache benchmarks
│   │   ├── check.rs            # ENHANCE: Add warm run variants
│   │   ├── memory.rs           # NEW: Memory high-water mark
│   │   └── regression.rs       # NEW: Regression detection helpers
│   └── src/
│       ├── timing.rs           # ENHANCE: Add memory tracking
│       └── bench_support.rs    # NEW: Benchmark utilities
├── tests/fixtures/
│   ├── bench-small/            # ENHANCE: Ensure populated
│   ├── bench-medium/           # ENHANCE: Ensure populated
│   ├── bench-large/            # ENHANCE: Ensure populated
│   └── bench-warm/             # NEW: Pre-cached fixture
├── scripts/
│   ├── fixtures/
│   │   └── generate-bench-fixtures.sh  # NEW: Fixture generator
│   └── perf/
│       ├── baseline-capture.sh     # NEW: Capture baseline
│       ├── regression-check.sh     # NEW: Check for regressions
│       └── report-perf.sh          # NEW: Generate perf report
└── .github/workflows/
    └── perf.yml                # NEW: Performance CI job
```

## Dependencies

**Existing (no changes):**
- `criterion = "0.5"` - Benchmark framework
- `rayon = "1.10"` - Parallelism (already in use)

**New (optional but recommended):**
- `peak_alloc = "0.2"` - Memory high-water mark tracking
- `cap = "0.1"` - Allocation counting (alternative to peak_alloc)

No new mandatory dependencies. Memory benchmarks can use system tools (heaptrack, Instruments) if crate integration is deferred.

## Implementation Phases

### Phase 1: Generate Benchmark Fixtures

**Goal:** Create consistent, reproducible benchmark fixtures at defined scales.

**Create:** `scripts/fixtures/generate-bench-fixtures.sh`

```bash
#!/usr/bin/env bash
# Generate benchmark fixtures at different scales
set -euo pipefail

FIXTURE_DIR="tests/fixtures"

# Small: ~50 files, ~5K LOC
generate_small() {
    local dir="$FIXTURE_DIR/bench-small"
    rm -rf "$dir"
    mkdir -p "$dir/src"

    for i in $(seq 1 50); do
        cat > "$dir/src/mod_$i.rs" << 'EOF'
pub fn function_${i}_a() -> i32 { 42 }
pub fn function_${i}_b() -> &'static str { "hello" }
pub fn function_${i}_c() -> bool { true }
EOF
    done

    echo 'fn main() {}' > "$dir/src/main.rs"
    echo '[package]\nname = "bench-small"\nversion = "0.1.0"' > "$dir/Cargo.toml"
}

# Medium: ~500 files, ~50K LOC
generate_medium() {
    local dir="$FIXTURE_DIR/bench-medium"
    rm -rf "$dir"
    mkdir -p "$dir/src"

    for i in $(seq 1 500); do
        # 100 lines per file
        printf 'pub fn func_%d() -> i32 { %d }\n' $(seq 1 100 | xargs -I {} echo "$i" {}) > "$dir/src/mod_$i.rs"
    done

    echo 'fn main() {}' > "$dir/src/main.rs"
    echo '[package]\nname = "bench-medium"\nversion = "0.1.0"' > "$dir/Cargo.toml"
}

# Large: ~5K files, ~500K LOC
generate_large() {
    local dir="$FIXTURE_DIR/bench-large"
    rm -rf "$dir"

    # Create crate structure with modules
    for crate in $(seq 1 10); do
        mkdir -p "$dir/crates/crate_$crate/src"
        for i in $(seq 1 500); do
            printf 'pub fn func_%d() -> i32 { %d }\n' $(seq 1 100 | xargs -I {} echo "$i" {}) > "$dir/crates/crate_$crate/src/mod_$i.rs"
        done
        echo '[package]\nname = "crate-'$crate'"\nversion = "0.1.0"' > "$dir/crates/crate_$crate/Cargo.toml"
    done

    cat > "$dir/Cargo.toml" << 'EOF'
[workspace]
members = ["crates/*"]
EOF
}

case "${1:-all}" in
    small)  generate_small ;;
    medium) generate_medium ;;
    large)  generate_large ;;
    all)    generate_small; generate_medium; generate_large ;;
esac
```

**Fixture specifications:**

| Fixture | Files | LOC | Purpose |
|---------|-------|-----|---------|
| bench-small | 50 | 5K | Quick sanity check |
| bench-medium | 500 | 50K | Target performance case |
| bench-large | 5K | 500K | Stress test |
| bench-deep | 1K | 50K | 50+ levels deep (exists) |

**Verification:**
```bash
./scripts/fixtures/generate-bench-fixtures.sh
find tests/fixtures/bench-* -type f | wc -l  # Verify counts
```

---

### Phase 2: Implement Warm Cache Benchmarks

**Goal:** Benchmark the primary use case - iterative runs with populated cache.

**Create:** `crates/cli/benches/cache.rs`

```rust
//! Warm cache benchmarks - the primary use case.
//!
//! Agents iterate repeatedly on a codebase. Most runs have a warm cache
//! where 95%+ of files are unchanged. This must be < 100ms.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::path::PathBuf;
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Warm the cache by running quench once, then benchmark subsequent runs.
fn bench_warm_cache(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("warm_cache");

    for fixture in ["bench-small", "bench-medium"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            eprintln!("Skipping {fixture}: run generate-bench-fixtures.sh");
            continue;
        }

        // Warm the cache (setup, not measured)
        Command::new(quench_bin)
            .args(["check", "--no-limit"])
            .current_dir(&path)
            .output()
            .expect("warmup should succeed");

        // Benchmark warm runs
        group.bench_with_input(
            BenchmarkId::new("check_warm", fixture),
            &path,
            |b, path| {
                b.iter(|| {
                    Command::new(quench_bin)
                        .args(["check", "--no-limit"])
                        .current_dir(path)
                        .output()
                        .expect("quench should run")
                })
            },
        );
    }

    group.finish();
}

/// Measure cache speedup ratio (cold time / warm time).
fn bench_cache_speedup(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("cache_speedup");

    let path = fixture_path("bench-medium");
    if !path.exists() {
        return;
    }

    // Cold run (clear cache first)
    let cache_dir = path.join(".quench");
    group.bench_function("cold", |b| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let _ = std::fs::remove_dir_all(&cache_dir);
                let start = std::time::Instant::now();
                Command::new(quench_bin)
                    .args(["check", "--no-limit"])
                    .current_dir(&path)
                    .output()
                    .expect("quench should run");
                total += start.elapsed();
            }
            total
        })
    });

    // Ensure cache is warm
    Command::new(quench_bin)
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warmup should succeed");

    // Warm run
    group.bench_function("warm", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .args(["check", "--no-limit"])
                .current_dir(&path)
                .output()
                .expect("quench should run")
        })
    });

    group.finish();
}

criterion_group!(benches, bench_warm_cache, bench_cache_speedup);
criterion_main!(benches);
```

**Add to `Cargo.toml`:**
```toml
[[bench]]
name = "cache"
harness = false
```

**Verification:**
```bash
cargo bench --bench cache
# Verify warm runs are < 100ms on bench-medium
```

---

### Phase 3: Enhance Check Benchmarks with Timing Assertions

**Goal:** Add threshold-based pass/fail to benchmarks for CI integration.

**Modify:** `crates/cli/benches/check.rs` - Add timing validation

```rust
/// Benchmark with timing assertion for CI.
fn bench_check_with_threshold(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("check_threshold");

    // Set measurement time for stable results
    group.measurement_time(std::time::Duration::from_secs(10));

    let fixtures_and_thresholds = [
        ("bench-small", 200),   // < 200ms acceptable
        ("bench-medium", 1000), // < 1s acceptable
    ];

    for (fixture, threshold_ms) in fixtures_and_thresholds {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("cold", fixture),
            &(path.clone(), threshold_ms),
            |b, (path, _threshold)| {
                b.iter_custom(|iters| {
                    let cache_dir = path.join(".quench");
                    let mut total = std::time::Duration::ZERO;
                    for _ in 0..iters {
                        let _ = std::fs::remove_dir_all(&cache_dir);
                        let start = std::time::Instant::now();
                        Command::new(quench_bin)
                            .args(["check", "--no-limit"])
                            .current_dir(path)
                            .output()
                            .expect("quench should run");
                        total += start.elapsed();
                    }
                    total
                })
            },
        );
    }

    group.finish();
}
```

**Create:** `scripts/perf/regression-check.sh`

```bash
#!/usr/bin/env bash
# Check for performance regressions against baseline
set -euo pipefail

BASELINE_FILE="${BASELINE_FILE:-target/criterion-baseline.json}"
THRESHOLD_PCT="${THRESHOLD_PCT:-20}"  # 20% regression threshold

# Run benchmarks in baseline comparison mode
cargo bench --bench check -- --baseline main --save-baseline current

# Compare results
if cargo bench --bench check -- --baseline main --compare 2>&1 | grep -q "regressed"; then
    echo "ERROR: Performance regression detected (>${THRESHOLD_PCT}% slower)"
    exit 1
fi

echo "Performance check passed"
```

**Verification:**
```bash
cargo bench --bench check -- --save-baseline main
# Make changes...
./scripts/perf/regression-check.sh
```

---

### Phase 4: Add Memory Benchmarks

**Goal:** Track memory high-water mark to ensure bounded growth.

**Create:** `crates/cli/benches/memory.rs`

```rust
//! Memory usage benchmarks.
//!
//! Validates memory targets from docs/specs/20-performance.md:
//! - Fast checks: < 100MB target, 500MB hard limit
//! - CI checks: < 500MB target, 2GB hard limit

use std::path::PathBuf;
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures")
        .join(name)
}

/// Parse peak memory from /usr/bin/time output (macOS/Linux).
fn measure_peak_memory(fixture: &str) -> Option<u64> {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let path = fixture_path(fixture);

    if !path.exists() {
        return None;
    }

    // Use /usr/bin/time to measure peak RSS
    #[cfg(target_os = "macos")]
    let output = Command::new("/usr/bin/time")
        .args(["-l", quench_bin, "check", "--no-limit"])
        .current_dir(&path)
        .output()
        .ok()?;

    #[cfg(target_os = "linux")]
    let output = Command::new("/usr/bin/time")
        .args(["-v", quench_bin, "check", "--no-limit"])
        .current_dir(&path)
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse memory from time output
    #[cfg(target_os = "macos")]
    {
        // macOS: "  12345678  peak memory footprint"
        for line in stderr.lines() {
            if line.contains("peak memory footprint") {
                let bytes: u64 = line.trim().split_whitespace().next()?.parse().ok()?;
                return Some(bytes);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: "Maximum resident set size (kbytes): 12345"
        for line in stderr.lines() {
            if line.contains("Maximum resident set size") {
                let kb: u64 = line.split(':').nth(1)?.trim().parse().ok()?;
                return Some(kb * 1024);
            }
        }
    }

    None
}

#[test]
fn test_memory_bench_medium() {
    if let Some(peak_bytes) = measure_peak_memory("bench-medium") {
        let peak_mb = peak_bytes / (1024 * 1024);
        println!("bench-medium peak memory: {}MB", peak_mb);

        // Target: < 100MB, Hard limit: 500MB
        assert!(peak_mb < 500, "Memory exceeded hard limit: {}MB > 500MB", peak_mb);

        if peak_mb > 100 {
            eprintln!("WARNING: Memory above target: {}MB > 100MB", peak_mb);
        }
    }
}

#[test]
fn test_memory_bench_large() {
    if let Some(peak_bytes) = measure_peak_memory("bench-large") {
        let peak_mb = peak_bytes / (1024 * 1024);
        println!("bench-large peak memory: {}MB", peak_mb);

        // CI target: < 500MB, Hard limit: 2GB
        assert!(peak_mb < 2048, "Memory exceeded hard limit: {}MB > 2GB", peak_mb);

        if peak_mb > 500 {
            eprintln!("WARNING: Memory above target: {}MB > 500MB", peak_mb);
        }
    }
}
```

**Alternative (using peak_alloc crate):**

If adding dependency, update `Cargo.toml`:
```toml
[dev-dependencies]
peak_alloc = "0.2"
```

```rust
#[global_allocator]
static PEAK_ALLOC: peak_alloc::PeakAlloc = peak_alloc::PeakAlloc;

#[test]
fn test_memory_internal() {
    // Run check logic directly (not subprocess)
    // ...

    let peak = PEAK_ALLOC.peak_usage();
    assert!(peak < 100 * 1024 * 1024, "Peak memory: {} > 100MB", peak);
}
```

**Verification:**
```bash
cargo test --bench memory -- --nocapture
```

---

### Phase 5: CI Performance Job

**Goal:** Automated performance regression detection in CI.

**Create:** `.github/workflows/perf.yml`

```yaml
name: Performance

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Generate fixtures
        run: ./scripts/fixtures/generate-bench-fixtures.sh

      - name: Build release
        run: cargo build --release

      - name: Run benchmarks
        run: |
          cargo bench --bench cache -- --save-baseline pr-${{ github.sha }}
          cargo bench --bench check -- --save-baseline pr-${{ github.sha }}

      - name: Check cold run target
        run: |
          # Verify bench-medium cold run < 1s (acceptable threshold)
          ./target/release/quench check tests/fixtures/bench-medium \
            --timing 2>&1 | grep -E "total: [0-9]+ms" | \
            awk -F'[: m]' '{if ($2 > 1000) exit 1}'

      - name: Check warm run target
        run: |
          # Run once to warm cache
          ./target/release/quench check tests/fixtures/bench-medium
          # Verify warm run < 200ms (acceptable threshold)
          ./target/release/quench check tests/fixtures/bench-medium \
            --timing 2>&1 | grep -E "total: [0-9]+ms" | \
            awk -F'[: m]' '{if ($2 > 200) exit 1}'

      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmarks-${{ github.sha }}
          path: target/criterion/
          retention-days: 30

  memory-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Generate fixtures
        run: ./scripts/fixtures/generate-bench-fixtures.sh

      - name: Build release
        run: cargo build --release

      - name: Check memory usage
        run: |
          # Measure peak memory on bench-medium
          /usr/bin/time -v ./target/release/quench check tests/fixtures/bench-medium 2>&1 | \
            grep "Maximum resident" | \
            awk -F': ' '{kb=$2; mb=kb/1024; if (mb > 500) {print "FAIL: "mb"MB > 500MB"; exit 1} else {print "OK: "mb"MB"}}'
```

**Verification:**
```bash
# Test locally with act or manually
act -j benchmark
```

---

### Phase 6: Performance Reporting Script

**Goal:** Generate human-readable performance reports for tracking over time.

**Create:** `scripts/perf/report-perf.sh`

```bash
#!/usr/bin/env bash
# Generate performance report
set -euo pipefail

QUENCH="${QUENCH:-./target/release/quench}"
FIXTURES_DIR="tests/fixtures"
REPORT_FILE="${1:-reports/perf-$(date +%Y%m%d).md}"

mkdir -p "$(dirname "$REPORT_FILE")"

cat > "$REPORT_FILE" << EOF
# Performance Report - $(date +%Y-%m-%d)

## Environment
- Commit: $(git rev-parse --short HEAD)
- Platform: $(uname -sm)
- Rust: $(rustc --version)

## Results

| Fixture | Mode | Files | Time (ms) | Cache Hits | Status |
|---------|------|-------|-----------|------------|--------|
EOF

for fixture in bench-small bench-medium bench-large; do
    path="$FIXTURES_DIR/$fixture"
    [ -d "$path" ] || continue

    files=$(find "$path" -type f -name '*.rs' | wc -l | tr -d ' ')

    # Cold run
    rm -rf "$path/.quench"
    cold=$("$QUENCH" check "$path" --timing 2>&1 | grep "total:" | awk '{print $2}' | tr -d 'ms')

    # Warm run
    warm=$("$QUENCH" check "$path" --timing 2>&1 | grep "total:" | awk '{print $2}' | tr -d 'ms')
    hits=$("$QUENCH" check "$path" --timing 2>&1 | grep "cache:" | awk -F'/' '{print $1}' | awk '{print $2}')

    # Determine status based on spec thresholds
    cold_status="OK"
    [ "$cold" -gt 2000 ] && cold_status="FAIL"
    [ "$cold" -gt 1000 ] && [ "$cold" -le 2000 ] && cold_status="WARN"

    warm_status="OK"
    [ "$warm" -gt 500 ] && warm_status="FAIL"
    [ "$warm" -gt 200 ] && [ "$warm" -le 500 ] && warm_status="WARN"

    echo "| $fixture | cold | $files | $cold | - | $cold_status |" >> "$REPORT_FILE"
    echo "| $fixture | warm | $files | $warm | $hits | $warm_status |" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" << EOF

## Thresholds (from docs/specs/20-performance.md)

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Cold | < 500ms | < 1s | > 2s |
| Warm | < 100ms | < 200ms | > 500ms |

## Notes

$([ -f reports/perf-notes.md ] && cat reports/perf-notes.md || echo "No notes.")
EOF

echo "Report written to: $REPORT_FILE"
cat "$REPORT_FILE"
```

**Verification:**
```bash
cargo build --release
./scripts/perf/report-perf.sh
cat reports/perf-*.md
```

## Key Implementation Details

### Benchmark Categories

| Category | Purpose | Tool |
|----------|---------|------|
| Warm cache | Primary use case validation | Criterion |
| Cold vs warm speedup | Cache effectiveness | Criterion |
| Memory high-water | Bounded growth verification | /usr/bin/time or peak_alloc |
| Regression detection | CI gate | Criterion baselines |
| Scaling behavior | Performance model validation | Custom harness |

### Cache Warming Strategy

For warm cache benchmarks, the setup phase must:
1. Run quench once to populate `.quench/cache.bin`
2. Verify cache file exists before benchmarking
3. Not modify any source files between warmup and benchmark

```rust
// Correct: warmup is separate from measurement
fn setup(path: &Path) {
    Command::new(quench_bin).args(["check"]).current_dir(path).output().unwrap();
    assert!(path.join(".quench/cache.bin").exists(), "Cache not created");
}

// Benchmark measures only warm runs
b.iter(|| Command::new(quench_bin).args(["check"]).current_dir(path).output());
```

### Threshold Enforcement

Use Criterion's `--compare` feature for regression detection:

```bash
# Save baseline on main branch
cargo bench -- --save-baseline main

# Compare PR against baseline
cargo bench -- --baseline main

# Fail if > 20% regression
cargo bench -- --baseline main --compare 2>&1 | grep -q "regressed" && exit 1
```

### Memory Measurement Approach

Prefer external measurement (`/usr/bin/time`) over internal tracking:
- Works with optimized release builds
- Measures actual RSS, not allocator metadata
- Cross-platform (macOS uses `-l`, Linux uses `-v`)

For granular internal tracking (if needed), use `peak_alloc` with `#[global_allocator]`.

## Verification Plan

### Phase 1 Verification
```bash
./scripts/fixtures/generate-bench-fixtures.sh
ls -la tests/fixtures/bench-*/
wc -l tests/fixtures/bench-medium/src/*.rs | tail -1  # ~50K lines
```

### Phase 2 Verification
```bash
cargo bench --bench cache
# Output should show warm runs < 100ms on bench-medium
```

### Phase 3 Verification
```bash
cargo bench --bench check -- --save-baseline test
cargo bench --bench check -- --baseline test
```

### Phase 4 Verification
```bash
cargo test --bench memory -- --nocapture
# Should report peak memory and pass assertions
```

### Phase 5 Verification
```bash
# Local CI simulation
./scripts/perf/regression-check.sh
```

### Phase 6 Verification
```bash
cargo build --release
./scripts/perf/report-perf.sh reports/test-perf.md
cat reports/test-perf.md
```

### Final Verification
```bash
# Full benchmark suite
make check
cargo bench

# Performance targets met
./scripts/perf/report-perf.sh
# Verify: cold < 1s, warm < 200ms on bench-medium

# Memory targets met
/usr/bin/time -v ./target/release/quench check tests/fixtures/bench-medium 2>&1 | grep "Maximum resident"
# Verify: < 500MB
```

## Exit Criteria

- [ ] Benchmark fixtures generated: bench-small (50 files), bench-medium (500 files), bench-large (5K files)
- [ ] Warm cache benchmark (`benches/cache.rs`) implemented and passing
- [ ] Cache speedup ratio measured (expect 5-10x)
- [ ] Check benchmarks enhanced with threshold validation
- [ ] Memory benchmark tests passing (< 500MB on bench-medium)
- [ ] CI performance job configured (`.github/workflows/perf.yml`)
- [ ] Performance report script working (`scripts/perf/report-perf.sh`)
- [ ] All benchmarks pass: `cargo bench`
- [ ] Warm run < 200ms on bench-medium (acceptable threshold)
- [ ] Cold run < 1s on bench-medium (acceptable threshold)
- [ ] `make check` passes
