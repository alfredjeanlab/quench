# Checkpoint 10E: Performance - Dogfooding Milestone 2

**Root Feature:** `quench-10e`
**Follows:** checkpoint-10d-benchmark (Benchmark Infrastructure)

## Overview

Add performance visibility and ensure sustained performance through dogfooding. This checkpoint:

1. **`--timing` flag** - Show performance breakdown during checks
2. **Timing metrics collection** - Measure file walking, cache lookups, checking
3. **CI benchmark enforcement** - Ensure regressions >20% fail CI
4. **Profiling infrastructure** - Document how to profile and identify bottlenecks

**Goal:** Developers and AI agents can see where time goes, and performance regressions are caught before merge.

**Background:** The dogfooding-milestone-2 report noted that `--timing` is not available, and timing was measured using the shell's `time` command. This checkpoint adds native timing support.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── timing.rs               # CREATE: Timing infrastructure
│   ├── timing_tests.rs         # CREATE: Unit tests
│   ├── cmd_check.rs            # MODIFY: Integrate timing
│   └── lib.rs                  # MODIFY: Export timing module
├── crates/cli/benches/
│   └── dogfood.rs              # MODIFY: Add timing benchmark
├── .github/workflows/
│   └── ci.yml                  # MODIFY: Add benchmark regression job
├── docs/
│   └── profiling.md            # CREATE: Profiling guide
└── plans/
    └── checkpoint-10e-perf.md  # THIS FILE
```

## Dependencies

No new crate dependencies. Uses existing:
- `std::time::Instant` for timing measurements
- `serde` for timing data serialization (already in deps)

## Implementation Phases

### Phase 1: Timing Infrastructure

Create a timing module to track performance breakdown.

**File:** `crates/cli/src/timing.rs`

```rust
use std::time::{Duration, Instant};

/// Tracks timing breakdown for a check run
#[derive(Debug, Default, Clone)]
pub struct Timing {
    pub total: Duration,
    pub file_walking: Duration,
    pub cache_lookups: Duration,
    pub file_reading: Duration,
    pub checking: Duration,
    pub files_scanned: usize,
    pub files_cached: usize,
    pub files_checked: usize,
}

impl Timing {
    pub fn new() -> Self {
        Self::default()
    }

    /// Format timing for display
    pub fn display(&self) -> String {
        let cache_rate = if self.files_scanned > 0 {
            (self.files_cached as f64 / self.files_scanned as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "Timing:\n  \
             total:        {:>7.2}ms\n  \
             file_walking: {:>7.2}ms ({:>4.1}%)\n  \
             cache_lookup: {:>7.2}ms ({:>4.1}%)\n  \
             file_reading: {:>7.2}ms ({:>4.1}%)\n  \
             checking:     {:>7.2}ms ({:>4.1}%)\n  \
             files:        {} scanned, {} cached ({:.0}% hit rate), {} checked",
            self.total.as_secs_f64() * 1000.0,
            self.file_walking.as_secs_f64() * 1000.0,
            self.percent(self.file_walking),
            self.cache_lookups.as_secs_f64() * 1000.0,
            self.percent(self.cache_lookups),
            self.file_reading.as_secs_f64() * 1000.0,
            self.percent(self.file_reading),
            self.checking.as_secs_f64() * 1000.0,
            self.percent(self.checking),
            self.files_scanned,
            self.files_cached,
            cache_rate,
            self.files_checked,
        )
    }

    fn percent(&self, duration: Duration) -> f64 {
        if self.total.as_nanos() == 0 {
            0.0
        } else {
            (duration.as_nanos() as f64 / self.total.as_nanos() as f64) * 100.0
        }
    }
}

/// RAII timer that accumulates into a Duration
pub struct Timer<'a> {
    start: Instant,
    target: &'a mut Duration,
}

impl<'a> Timer<'a> {
    pub fn start(target: &'a mut Duration) -> Self {
        Self {
            start: Instant::now(),
            target,
        }
    }
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        *self.target += self.start.elapsed();
    }
}
```

**File:** `crates/cli/src/timing_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::time::Duration;

#[test]
fn timing_display_shows_breakdown() {
    let timing = Timing {
        total: Duration::from_millis(100),
        file_walking: Duration::from_millis(40),
        cache_lookups: Duration::from_millis(10),
        file_reading: Duration::from_millis(20),
        checking: Duration::from_millis(30),
        files_scanned: 100,
        files_cached: 90,
        files_checked: 10,
    };
    let display = timing.display();
    assert!(display.contains("total:"));
    assert!(display.contains("file_walking:"));
    assert!(display.contains("90% hit rate"));
}

#[test]
fn timer_accumulates_duration() {
    let mut total = Duration::ZERO;
    {
        let _timer = Timer::start(&mut total);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(total >= Duration::from_millis(10));
}

#[test]
fn timing_handles_zero_files() {
    let timing = Timing::default();
    let display = timing.display();
    assert!(display.contains("0 scanned"));
}
```

**Verification:**
```bash
cargo test --lib timing
```

---

### Phase 2: CLI Integration

Add `--timing` flag to the check command.

**Modify:** `crates/cli/src/cmd_check.rs`

Add timing integration to existing check flow:

```rust
use crate::timing::{Timing, Timer};

/// Check command options
#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    // ... existing args ...

    /// Show timing breakdown
    #[arg(long)]
    pub timing: bool,
}

impl CheckArgs {
    pub fn run(&self, config: &Config) -> Result<CheckResult> {
        let mut timing = Timing::new();
        let start = Instant::now();

        // File walking
        let files = {
            let _timer = Timer::start(&mut timing.file_walking);
            self.discover_files(config)?
        };
        timing.files_scanned = files.len();

        // Check files (with cache)
        let violations = {
            let (violations, cache_stats) = self.check_files(&files, config, &mut timing)?;
            timing.files_cached = cache_stats.hits;
            timing.files_checked = cache_stats.misses;
            violations
        };

        timing.total = start.elapsed();

        // Output timing if requested
        if self.timing {
            eprintln!("{}", timing.display());
        }

        Ok(CheckResult { violations, timing })
    }
}
```

**Modify:** `crates/cli/src/lib.rs`

Export the timing module:

```rust
pub mod timing;

#[cfg(test)]
#[path = "timing_tests.rs"]
mod timing_tests;
```

**Verification:**
```bash
# Build and test
cargo build --release
./target/release/quench check --timing

# Expected output (appended after normal output):
# Timing:
#   total:         35.42ms
#   file_walking:  12.50ms (35.3%)
#   cache_lookup:   2.10ms ( 5.9%)
#   file_reading:   5.80ms (16.4%)
#   checking:      15.02ms (42.4%)
#   files:        837 scanned, 837 cached (100% hit rate), 0 checked
```

---

### Phase 3: JSON Output Support

Add timing to JSON output for tooling integration.

**Modify timing to support serde:**

```rust
use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Timing {
    #[serde(serialize_with = "serialize_duration_ms")]
    pub total: Duration,
    #[serde(serialize_with = "serialize_duration_ms")]
    pub file_walking: Duration,
    #[serde(serialize_with = "serialize_duration_ms")]
    pub cache_lookups: Duration,
    #[serde(serialize_with = "serialize_duration_ms")]
    pub file_reading: Duration,
    #[serde(serialize_with = "serialize_duration_ms")]
    pub checking: Duration,
    pub files_scanned: usize,
    pub files_cached: usize,
    pub files_checked: usize,
}

fn serialize_duration_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64() * 1000.0)
}
```

**JSON output format:**

```json
{
  "status": "pass",
  "checks": [...],
  "timing": {
    "total": 35.42,
    "file_walking": 12.50,
    "cache_lookups": 2.10,
    "file_reading": 5.80,
    "checking": 15.02,
    "files_scanned": 837,
    "files_cached": 837,
    "files_checked": 0
  }
}
```

**Verification:**
```bash
./target/release/quench check --timing -o json | jq '.timing'
```

---

### Phase 4: CI Benchmark Enforcement

Ensure CI fails on performance regressions.

**Modify:** `.github/workflows/ci.yml`

Add a dedicated benchmark job that runs `scripts/benchmark` and fails on regression:

```yaml
  benchmark:
    name: Benchmark Regression Check
    runs-on: ubuntu-latest
    needs: [build]  # Run after build succeeds
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-bench-${{ hashFiles('**/Cargo.lock') }}

      - name: Build release binary
        run: cargo build --release -p quench

      - name: Run regression tests
        run: cargo test --release --bench regression -- --nocapture

      - name: Run benchmark comparison
        run: ./scripts/benchmark
        env:
          CI: true
```

**Modify:** `scripts/benchmark`

Ensure script exits non-zero on regression:

```bash
# Add regression detection after running benchmarks
check_regression() {
    local bench="$1"
    local current_ms="$2"
    local baseline_ms=$(jq -r ".benchmarks[\"$bench\"].mean_ms // empty" "$BASELINE")
    local threshold_pct=$(jq -r '.thresholds.regression_threshold_pct // 20' "$BASELINE")

    if [[ -n "$baseline_ms" ]]; then
        local max_allowed=$(echo "$baseline_ms * (1 + $threshold_pct / 100)" | bc -l)
        if (( $(echo "$current_ms > $max_allowed" | bc -l) )); then
            echo "REGRESSION: $bench is ${current_ms}ms vs baseline ${baseline_ms}ms (>${threshold_pct}% slower)"
            return 1
        fi
    fi
    return 0
}
```

**Verification:**
```bash
# Test locally
./scripts/benchmark
echo $?  # Should be 0 if no regression

# Simulate regression (modify a baseline to lower value, run benchmark)
```

---

### Phase 5: Profiling Documentation

Document how to profile quench for performance optimization.

**File:** `docs/profiling.md`

```markdown
# Performance Profiling Guide

This guide covers how to profile quench to identify performance bottlenecks.

## Quick Performance Check

Use `--timing` to see where time goes:

```bash
quench check --timing
```

Example output:
```
Timing:
  total:         35.42ms
  file_walking:  12.50ms (35.3%)
  cache_lookup:   2.10ms ( 5.9%)
  file_reading:   5.80ms (16.4%)
  checking:      15.02ms (42.4%)
  files:        837 scanned, 837 cached (100% hit rate), 0 checked
```

## Interpreting Timing Results

| Phase | Expected % | If Higher |
|-------|------------|-----------|
| file_walking | 30-50% | Consider caching file list |
| cache_lookup | <10% | Cache implementation issue |
| file_reading | 20-30% | Many uncached files, or slow filesystem |
| checking | 20-40% | Pattern matching bottleneck |

## Flame Graph Profiling

For detailed profiling, generate a flame graph:

```bash
# Install flamegraph
cargo install flamegraph

# Linux: uses perf
sudo flamegraph -o flamegraph.svg -- ./target/release/quench check

# macOS: uses dtrace (requires SIP disabled or special entitlements)
flamegraph -o flamegraph.svg -- ./target/release/quench check

# View
open flamegraph.svg
```

## CPU Profiling (Linux)

```bash
# Record with perf
perf record -g ./target/release/quench check /path/to/repo

# View report
perf report

# Generate annotated source
perf annotate
```

## CPU Profiling (macOS)

```bash
# Record with Instruments
xcrun xctrace record --template 'Time Profiler' \
    --launch ./target/release/quench check /path/to/repo

# Open in Instruments
open *.trace
```

## Memory Profiling

```bash
# Using heaptrack (Linux)
heaptrack ./target/release/quench check
heaptrack_gui heaptrack.quench.*.gz

# Using Instruments (macOS)
xcrun xctrace record --template 'Allocations' \
    --launch ./target/release/quench check
```

## Benchmark-Driven Optimization

1. **Establish baseline:** Run benchmarks, update baseline
   ```bash
   ./scripts/benchmark
   ./scripts/update-baseline
   ```

2. **Make optimization**

3. **Verify improvement:**
   ```bash
   cargo bench --bench dogfood
   ```

4. **Check for regressions:**
   ```bash
   ./scripts/benchmark
   ```

5. **Update baseline if improved:**
   ```bash
   ./scripts/update-baseline
   git add reports/benchmark-baseline.json
   git commit -m "perf: update baseline after optimization"
   ```

## Common Bottlenecks

### File Walking Slow (>50%)

- Gitignore patterns too complex
- Too many directories traversed
- Solution: Review `.gitignore`, add more exclusions

### Cache Miss Rate High

- Config changed between runs
- Quench version changed
- Files modified
- Solution: Check `quench check --timing` after warm-up run

### Checking Slow (>50%)

- Complex regex patterns
- Many patterns to match
- Solution: Use literal patterns where possible

## Performance Targets

From `docs/specs/20-performance.md`:

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Warm run | < 100ms | < 200ms | > 500ms |
| Cold run | < 500ms | < 1s | > 2s |
| CI checks | < 5s | < 15s | > 30s |
```

**Verification:**
```bash
cat docs/profiling.md
```

---

### Phase 6: Timing Benchmark

Add timing overhead measurement to ensure `--timing` doesn't impact performance.

**Modify:** `crates/cli/benches/dogfood.rs`

Add benchmark for timing overhead:

```rust
fn dogfood_timing(c: &mut Criterion) {
    let mut group = c.benchmark_group("dogfood");

    // Baseline without timing
    group.bench_function("fast", |b| {
        b.iter(|| {
            Command::new(QUENCH)
                .args(["check"])
                .output()
        })
    });

    // With timing enabled
    group.bench_function("fast_timing", |b| {
        b.iter(|| {
            Command::new(QUENCH)
                .args(["check", "--timing"])
                .output()
        })
    });

    group.finish();
}
```

**Verification:**
```bash
cargo bench --bench dogfood -- dogfood/fast
# Compare fast vs fast_timing - timing overhead should be <5%
```

---

## Key Implementation Details

### Timer Design

The `Timer` struct uses RAII to automatically accumulate timing:

```rust
// Timing is accumulated when timer goes out of scope
{
    let _timer = Timer::start(&mut timing.file_walking);
    // ... file walking code ...
} // Timer dropped here, duration added
```

This ensures timing is accurate even with early returns or errors.

### Timing Accuracy

- Use `Instant::now()` for monotonic clock (not affected by system time changes)
- Accumulate durations to handle re-entrant sections
- Total may not equal sum of parts due to unmeasured overhead

### Cache Stats Integration

Timing integrates with existing cache to report:
- `files_cached`: Files with cache hits (no re-check needed)
- `files_checked`: Files actually checked (cache misses)
- Hit rate = cached / scanned

### JSON Output Compatibility

Timing is only included in JSON output when `--timing` is passed:
- Without `--timing`: JSON output unchanged (backward compatible)
- With `--timing`: Adds `timing` object to output

---

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test --lib timing` | All timing tests pass |
| 2 | `quench check --timing` | Shows timing breakdown |
| 3 | `quench check --timing -o json \| jq .timing` | JSON includes timing |
| 4 | `./scripts/benchmark` | Exits 0 if no regression |
| 5 | `cat docs/profiling.md` | Documentation complete |
| 6 | `cargo bench --bench dogfood` | Timing overhead <5% |

---

## Summary

| Phase | Deliverable | Purpose |
|-------|-------------|---------|
| 1 | `timing.rs` | Core timing infrastructure |
| 2 | `--timing` flag | CLI performance visibility |
| 3 | JSON timing | Tooling integration |
| 4 | CI benchmark job | Prevent regressions |
| 5 | `profiling.md` | Developer documentation |
| 6 | Timing benchmark | Verify minimal overhead |

---

## Completion Criteria

- [ ] Phase 1: `cargo test --lib timing` passes
- [ ] Phase 2: `quench check --timing` shows breakdown
- [ ] Phase 3: `quench check --timing -o json` includes timing object
- [ ] Phase 4: CI workflow includes benchmark job
- [ ] Phase 5: `docs/profiling.md` documents profiling workflow
- [ ] Phase 6: Timing overhead <5% in benchmarks
- [ ] `make check` passes
- [ ] `./done` executed successfully
