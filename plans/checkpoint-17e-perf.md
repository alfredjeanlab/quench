# Checkpoint 17E: Performance Fixes - Performance

**Plan:** `checkpoint-17e-perf`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17D (Benchmark Infrastructure)

## Overview

Apply profile-guided performance optimizations based on benchmark results from checkpoint 17D. The focus is on identifying and fixing performance bottlenecks, implementing deferred P1/P2 optimizations where profiling justifies them, and hardening CI performance gates to prevent regressions.

**Current Performance (Baseline):**

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Cold run | < 500ms | 316.5ms | PASS |
| Warm run | < 100ms | 47.1ms | PASS |
| Memory | < 100MB | 14.5MB | PASS |

**Goal:** Fix identified bottlenecks, implement justified optimizations, and establish performance budgets that prevent regressions.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── regression.rs       # NEW: Automated regression tests
│   └── src/
│       ├── walker.rs           # ENHANCE: Apply P1 walker optimizations if needed
│       ├── pattern.rs          # ENHANCE: Apply P2 pattern combining if needed
│       └── perf_budget.rs      # NEW: Performance budget enforcement
├── scripts/perf/
│   ├── profile.sh              # NEW: Automated profiling script
│   ├── flamegraph.sh           # NEW: Flamegraph generation
│   └── budget-check.sh         # NEW: Enforce performance budgets
├── reports/
│   └── profiling/              # NEW: Profiling results
│       ├── baseline-profile.md # Initial profiling report
│       └── hotspots.md         # Identified hotspots and fixes
└── docs/
    └── profiling.md            # ENHANCE: Document findings
```

## Dependencies

**Existing (no changes):**
- `criterion = "0.5"` - Benchmark framework
- `rayon = "1.10"` - Parallelism
- `dashmap = "6.0"` - Concurrent caching

**Optional (for profiling, not runtime):**
- `flamegraph` (cargo install) - Flame graph generation
- `perf` (Linux) or Instruments (macOS) - System profilers

No new runtime dependencies.

## Implementation Phases

### Phase 1: Baseline Profiling

**Goal:** Profile the hot paths and document where time is spent.

**Create:** `scripts/perf/profile.sh`

```bash
#!/usr/bin/env bash
# Run profiling on quench and generate reports
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FIXTURE="${1:-tests/fixtures/stress-monorepo}"
REPORT_DIR="reports/profiling"
mkdir -p "$REPORT_DIR"

# Build with debug symbols for profiling
RUSTFLAGS="-g" cargo build --release

echo "=== Profiling cold run ==="
rm -rf "$FIXTURE/.quench"

if [[ "$(uname)" == "Darwin" ]]; then
    # macOS: Use Instruments or sample
    xcrun xctrace record --template 'Time Profiler' \
        --output "$REPORT_DIR/cold-trace.trace" \
        --launch -- ./target/release/quench check "$FIXTURE" || true

    # Also sample for quick view
    sample ./target/release/quench 1 -file "$REPORT_DIR/cold-sample.txt" \
        -wait &
    SAMPLE_PID=$!
    ./target/release/quench check "$FIXTURE"
    wait $SAMPLE_PID 2>/dev/null || true
else
    # Linux: Use perf
    perf record -g -o "$REPORT_DIR/cold-perf.data" \
        ./target/release/quench check "$FIXTURE"
    perf report -i "$REPORT_DIR/cold-perf.data" > "$REPORT_DIR/cold-perf.txt"
fi

echo "=== Profiling warm run ==="
# Warm the cache first
./target/release/quench check "$FIXTURE" >/dev/null

if [[ "$(uname)" == "Darwin" ]]; then
    sample ./target/release/quench 1 -file "$REPORT_DIR/warm-sample.txt" \
        -wait &
    SAMPLE_PID=$!
    ./target/release/quench check "$FIXTURE"
    wait $SAMPLE_PID 2>/dev/null || true
else
    perf record -g -o "$REPORT_DIR/warm-perf.data" \
        ./target/release/quench check "$FIXTURE"
    perf report -i "$REPORT_DIR/warm-perf.data" > "$REPORT_DIR/warm-perf.txt"
fi

echo "Profiling complete. Reports in $REPORT_DIR/"
```

**Create:** `scripts/perf/flamegraph.sh`

```bash
#!/usr/bin/env bash
# Generate flame graphs for quench
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

FIXTURE="${1:-tests/fixtures/stress-monorepo}"
REPORT_DIR="reports/profiling"
mkdir -p "$REPORT_DIR"

# Check for flamegraph
if ! command -v flamegraph &>/dev/null; then
    echo "Installing flamegraph..."
    cargo install flamegraph
fi

echo "=== Generating cold run flamegraph ==="
rm -rf "$FIXTURE/.quench"
flamegraph -o "$REPORT_DIR/cold-flamegraph.svg" \
    -- ./target/release/quench check "$FIXTURE"

echo "=== Generating warm run flamegraph ==="
# Warm first
./target/release/quench check "$FIXTURE" >/dev/null
flamegraph -o "$REPORT_DIR/warm-flamegraph.svg" \
    -- ./target/release/quench check "$FIXTURE"

echo "Flame graphs generated in $REPORT_DIR/"
```

**Create:** `reports/profiling/baseline-profile.md` (template)

```markdown
# Baseline Profiling Report

Date: YYYY-MM-DD
Commit: XXXXXXX

## Environment

- Hardware:
- OS:
- Rust:

## Cold Run Profile

Fixture: stress-monorepo (~85K LOC)
Time: XXXms

### Time Breakdown

| Phase | Time (ms) | % of Total |
|-------|-----------|------------|
| File discovery | | |
| File reading | | |
| Pattern matching | | |
| Cache write | | |
| Output | | |

### Top Functions

1. `function_name` - XX%
2. ...

### Observations

-

## Warm Run Profile

Time: XXXms

### Time Breakdown

| Phase | Time (ms) | % of Total |
|-------|-----------|------------|
| Cache load | | |
| File mtime check | | |
| Output | | |

### Top Functions

1. ...

### Observations

-

## Identified Hotspots

### Hotspot 1: [Name]

**Where:** `src/file.rs:line`
**Time:** X% of total
**Root Cause:**
**Fix:**

## Recommendations

### P1 Optimizations (If Justified)

- [ ] Walker tuning:
- [ ] File list caching:

### P2 Optimizations (If Justified)

- [ ] Pattern combining:
- [ ] Literal prefiltering:

### Defer (Not Needed)

- P3/P4 micro-optimizations: Current memory and performance well within targets
```

**Verification:**
```bash
./scripts/perf/profile.sh tests/fixtures/stress-monorepo
cat reports/profiling/baseline-profile.md
```

---

### Phase 2: Fix Identified Bottlenecks

**Goal:** Apply targeted fixes for bottlenecks identified in profiling.

This phase is data-driven. Based on profiling results, implement fixes. Common patterns:

**Pattern A: If file walking is >50% of time**

Enhance `crates/cli/src/walker.rs`:

```rust
/// Parallel threshold tuning based on profiling.
/// Below this threshold, sequential walk is faster due to thread spawn overhead.
pub const PARALLEL_THRESHOLD: usize = 500; // Tuned from 1000 based on profiling

/// Pre-filter common uninteresting directories during walk.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "__pycache__",
    ".venv",
    "vendor",
];

impl Walker {
    /// Optimized walk with pre-filtering.
    pub fn walk_optimized(&self, root: &Path) -> WalkResult {
        WalkBuilder::new(root)
            .hidden(true)
            .git_ignore(true)
            .git_exclude(true)
            .max_depth(self.max_depth)
            .filter_entry(|entry| {
                // Skip known uninteresting directories early
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy();
                    if SKIP_DIRS.iter().any(|s| name == *s) {
                        return false;
                    }
                }
                true
            })
            .threads(self.threads())
            .build_parallel()
            // ...
    }
}
```

**Pattern B: If pattern matching is >40% of time**

Create `crates/cli/src/pattern.rs` enhancement:

```rust
use aho_corasick::AhoCorasick;
use memchr::memmem;

/// Compiled pattern set with optimization tiers.
pub enum CompiledPatterns {
    /// Single literal - use memchr
    SingleLiteral(memmem::Finder<'static>),
    /// Multiple literals - use Aho-Corasick
    MultiLiteral(AhoCorasick),
    /// Mixed patterns - use regex with prefilter
    Mixed {
        literals: Option<AhoCorasick>,
        regex: Vec<regex::Regex>,
    },
}

impl CompiledPatterns {
    /// Compile patterns with automatic optimization tier selection.
    pub fn compile(patterns: &[Pattern]) -> Self {
        let literals: Vec<_> = patterns
            .iter()
            .filter_map(|p| p.as_literal())
            .collect();

        let regexes: Vec<_> = patterns
            .iter()
            .filter(|p| !p.is_literal())
            .collect();

        match (literals.len(), regexes.len()) {
            (1, 0) => Self::SingleLiteral(
                memmem::Finder::new(literals[0]).into_owned()
            ),
            (n, 0) if n > 0 => Self::MultiLiteral(
                AhoCorasick::new(&literals).unwrap()
            ),
            _ => Self::Mixed {
                literals: if literals.is_empty() {
                    None
                } else {
                    Some(AhoCorasick::new(&literals).unwrap())
                },
                regex: regexes.iter().map(|p| p.compile_regex()).collect(),
            },
        }
    }

    /// Check if content matches any pattern.
    pub fn is_match(&self, content: &str) -> bool {
        match self {
            Self::SingleLiteral(f) => f.find(content.as_bytes()).is_some(),
            Self::MultiLiteral(ac) => ac.is_match(content),
            Self::Mixed { literals, regex } => {
                literals.as_ref().map(|ac| ac.is_match(content)).unwrap_or(false)
                    || regex.iter().any(|r| r.is_match(content))
            }
        }
    }
}
```

**Pattern C: If cache I/O is significant**

Optimize cache serialization in `crates/cli/src/cache.rs`:

```rust
/// Write cache asynchronously to avoid blocking.
pub fn save_async(&self, path: &Path) -> std::thread::JoinHandle<Result<(), CacheError>> {
    let path = path.to_owned();
    let data = self.entries.clone();
    let config_hash = self.config_hash;

    std::thread::spawn(move || {
        let cache = PersistentCache {
            version: CACHE_VERSION,
            quench_version: env!("CARGO_PKG_VERSION").to_string(),
            config_hash,
            entries: data.into_iter().collect(),
        };

        let bytes = postcard::to_allocvec(&cache)?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, bytes)?;
        Ok(())
    })
}
```

**Verification:**
```bash
# Re-run profiling after fixes
./scripts/perf/profile.sh
# Compare before/after times
hyperfine --warmup 0 --runs 5 -i \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'
```

---

### Phase 3: Performance Budgets

**Goal:** Establish and enforce hard limits that fail CI on regression.

**Create:** `scripts/perf/budget-check.sh`

```bash
#!/usr/bin/env bash
# Enforce performance budgets
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Performance budgets (in milliseconds)
COLD_TARGET=500
COLD_ACCEPTABLE=1000
COLD_UNACCEPTABLE=2000

WARM_TARGET=100
WARM_ACCEPTABLE=200
WARM_UNACCEPTABLE=500

MEMORY_TARGET_MB=100
MEMORY_LIMIT_MB=500

FIXTURE="tests/fixtures/bench-medium"

# Build release
cargo build --release --quiet

# Generate fixtures if needed
if [ ! -d "$FIXTURE" ]; then
    ./scripts/fixtures/generate-bench-fixtures
fi

echo "=== Performance Budget Check ==="
echo ""

# Cold run (average of 3)
echo "Cold run (3 runs, cache cleared each time):"
rm -rf "$FIXTURE/.quench"
COLD_TOTAL=0
for i in 1 2 3; do
    rm -rf "$FIXTURE/.quench"
    START=$(python3 -c 'import time; print(int(time.time() * 1000))')
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    END=$(python3 -c 'import time; print(int(time.time() * 1000))')
    COLD_MS=$((END - START))
    COLD_TOTAL=$((COLD_TOTAL + COLD_MS))
    echo "  Run $i: ${COLD_MS}ms"
done
COLD_AVG=$((COLD_TOTAL / 3))
echo "  Average: ${COLD_AVG}ms (target: <${COLD_TARGET}ms, limit: <${COLD_ACCEPTABLE}ms)"

if [ "$COLD_AVG" -gt "$COLD_UNACCEPTABLE" ]; then
    echo "::error::FAIL: Cold run ${COLD_AVG}ms exceeds unacceptable threshold ${COLD_UNACCEPTABLE}ms"
    exit 1
elif [ "$COLD_AVG" -gt "$COLD_ACCEPTABLE" ]; then
    echo "::warning::WARN: Cold run ${COLD_AVG}ms exceeds acceptable threshold ${COLD_ACCEPTABLE}ms"
elif [ "$COLD_AVG" -gt "$COLD_TARGET" ]; then
    echo "  Note: Above target but within acceptable range"
else
    echo "  OK: Within target"
fi
echo ""

# Warm run (average of 5, after warmup)
echo "Warm run (5 runs, cache warm):"
./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true  # warmup
WARM_TOTAL=0
for i in 1 2 3 4 5; do
    START=$(python3 -c 'import time; print(int(time.time() * 1000))')
    ./target/release/quench check "$FIXTURE" >/dev/null 2>&1 || true
    END=$(python3 -c 'import time; print(int(time.time() * 1000))')
    WARM_MS=$((END - START))
    WARM_TOTAL=$((WARM_TOTAL + WARM_MS))
    echo "  Run $i: ${WARM_MS}ms"
done
WARM_AVG=$((WARM_TOTAL / 5))
echo "  Average: ${WARM_AVG}ms (target: <${WARM_TARGET}ms, limit: <${WARM_ACCEPTABLE}ms)"

if [ "$WARM_AVG" -gt "$WARM_UNACCEPTABLE" ]; then
    echo "::error::FAIL: Warm run ${WARM_AVG}ms exceeds unacceptable threshold ${WARM_UNACCEPTABLE}ms"
    exit 1
elif [ "$WARM_AVG" -gt "$WARM_ACCEPTABLE" ]; then
    echo "::warning::WARN: Warm run ${WARM_AVG}ms exceeds acceptable threshold ${WARM_ACCEPTABLE}ms"
elif [ "$WARM_AVG" -gt "$WARM_TARGET" ]; then
    echo "  Note: Above target but within acceptable range"
else
    echo "  OK: Within target"
fi
echo ""

# Memory check
echo "Memory usage:"
if command -v /usr/bin/time &>/dev/null; then
    if [[ "$(uname)" == "Linux" ]]; then
        MEM_KB=$(/usr/bin/time -v ./target/release/quench check "$FIXTURE" 2>&1 | \
            grep "Maximum resident" | awk '{print $NF}')
        MEM_MB=$((MEM_KB / 1024))
    else
        # macOS
        MEM_BYTES=$(/usr/bin/time -l ./target/release/quench check "$FIXTURE" 2>&1 | \
            grep "peak memory" | awk '{print $1}')
        MEM_MB=$((MEM_BYTES / 1024 / 1024))
    fi
    echo "  Peak memory: ${MEM_MB}MB (target: <${MEMORY_TARGET_MB}MB, limit: <${MEMORY_LIMIT_MB}MB)"

    if [ "$MEM_MB" -gt "$MEMORY_LIMIT_MB" ]; then
        echo "::error::FAIL: Memory ${MEM_MB}MB exceeds limit ${MEMORY_LIMIT_MB}MB"
        exit 1
    elif [ "$MEM_MB" -gt "$MEMORY_TARGET_MB" ]; then
        echo "::warning::WARN: Memory ${MEM_MB}MB exceeds target ${MEMORY_TARGET_MB}MB"
    else
        echo "  OK: Within target"
    fi
fi
echo ""

echo "=== All budgets passed ==="
```

**Update:** `.github/workflows/bench.yml` - Add budget check step

```yaml
      - name: Check performance budgets
        run: ./scripts/perf/budget-check.sh
```

**Verification:**
```bash
./scripts/perf/budget-check.sh
```

---

### Phase 4: Regression Test Suite

**Goal:** Prevent performance regressions with automated tests.

**Create:** `crates/cli/benches/regression.rs`

```rust
//! Performance regression tests.
//!
//! These tests have hard time limits and fail if exceeded.
//! Unlike benchmarks (which compare to baselines), these are absolute limits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

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

/// Cold run must complete within 2 seconds (unacceptable threshold).
#[test]
fn cold_run_under_2s() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        return;
    }

    let cache_dir = path.join(".quench");
    let _ = std::fs::remove_dir_all(&cache_dir);

    let start = Instant::now();
    let output = Command::new(quench_bin())
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    // Log actual time for debugging
    eprintln!("Cold run time: {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(2),
        "Cold run took {:?}, exceeds 2s limit",
        elapsed
    );

    // Also verify it ran successfully (exit 0 or 1 for violations)
    assert!(
        output.status.code().unwrap_or(-1) <= 1,
        "Unexpected exit code: {:?}",
        output.status
    );
}

/// Warm run must complete within 500ms (unacceptable threshold).
#[test]
fn warm_run_under_500ms() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        return;
    }

    // Warm the cache
    let cache_dir = path.join(".quench");
    let _ = std::fs::remove_dir_all(&cache_dir);
    Command::new(quench_bin())
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warmup should run");

    // Verify cache exists
    assert!(
        cache_dir.join("cache.bin").exists(),
        "Cache not created during warmup"
    );

    let start = Instant::now();
    let output = Command::new(quench_bin())
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("quench should run");
    let elapsed = start.elapsed();

    eprintln!("Warm run time: {:?}", elapsed);

    assert!(
        elapsed < Duration::from_millis(500),
        "Warm run took {:?}, exceeds 500ms limit",
        elapsed
    );

    assert!(
        output.status.code().unwrap_or(-1) <= 1,
        "Unexpected exit code: {:?}",
        output.status
    );
}

/// Cache speedup should be at least 3x.
#[test]
fn cache_provides_speedup() {
    let path = fixture_path("bench-medium");
    if !path.exists() {
        eprintln!("Skipping: bench-medium fixture not found");
        return;
    }

    let cache_dir = path.join(".quench");

    // Cold run
    let _ = std::fs::remove_dir_all(&cache_dir);
    let cold_start = Instant::now();
    Command::new(quench_bin())
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("cold run should complete");
    let cold_time = cold_start.elapsed();

    // Warm run
    let warm_start = Instant::now();
    Command::new(quench_bin())
        .args(["check", "--no-limit"])
        .current_dir(&path)
        .output()
        .expect("warm run should complete");
    let warm_time = warm_start.elapsed();

    eprintln!("Cold: {:?}, Warm: {:?}", cold_time, warm_time);

    let speedup = cold_time.as_millis() as f64 / warm_time.as_millis().max(1) as f64;
    eprintln!("Speedup: {:.1}x", speedup);

    assert!(
        speedup >= 3.0,
        "Cache speedup is only {:.1}x, expected at least 3x (cold: {:?}, warm: {:?})",
        speedup,
        cold_time,
        warm_time
    );
}
```

**Add to `Cargo.toml`:**
```toml
[[bench]]
name = "regression"
harness = false
```

**Verification:**
```bash
cargo test --bench regression -- --nocapture
```

---

### Phase 5: Documentation Update

**Goal:** Document profiling methodology and optimization decisions.

**Update:** `docs/profiling.md`

Add section on optimization decisions and results:

```markdown
## Optimization History

### Checkpoint 17E Profiling Results

**Date:** YYYY-MM-DD

**Cold Run Analysis (stress-monorepo, ~85K LOC):**

| Phase | Before | After | Change |
|-------|--------|-------|--------|
| File discovery | Xms (Y%) | | |
| File reading | Xms (Y%) | | |
| Pattern matching | Xms (Y%) | | |
| Cache write | Xms (Y%) | | |
| **Total** | 316ms | | |

**Warm Run Analysis:**

| Phase | Before | After | Change |
|-------|--------|-------|--------|
| Cache load | Xms (Y%) | | |
| Mtime check | Xms (Y%) | | |
| Output | Xms (Y%) | | |
| **Total** | 47ms | | |

### Applied Optimizations

1. **[Optimization Name]**
   - **Where:** `src/file.rs`
   - **Improvement:** X% faster
   - **Justification:** Profiling showed Y% of time in Z

### Deferred Optimizations

1. **P3: Bounded cache (moka)**
   - **Status:** Deferred
   - **Reason:** Memory usage 14.5MB << 100MB target

2. **P4: String interning (lasso)**
   - **Status:** Deferred
   - **Reason:** No allocation bottleneck identified

## Running Profiling

### Quick Profile (Sample-based)

```bash
./scripts/perf/profile.sh tests/fixtures/stress-monorepo
```

### Detailed Flame Graph

```bash
./scripts/perf/flamegraph.sh tests/fixtures/stress-monorepo
open reports/profiling/cold-flamegraph.svg
```

### Performance Budget Check

```bash
./scripts/perf/budget-check.sh
```
```

**Verification:**
```bash
cat docs/profiling.md
```

---

### Phase 6: Final Validation

**Goal:** Verify all performance criteria still pass after changes.

**Create:** `reports/checkpoint-17e-validation.md` (template)

```markdown
# Checkpoint 17E Validation Report

Date: YYYY-MM-DD
Commit: XXXXXXX

## Summary

**All criteria validated.**

## Changes Applied

1. [List optimizations applied]
2. [List new scripts/tests added]
3. [List documentation updates]

## Performance Results

### Before (Checkpoint 17D Baseline)

| Metric | Value |
|--------|-------|
| Cold run | 316.5ms |
| Warm run | 47.1ms |
| Memory | 14.5MB |

### After (Checkpoint 17E)

| Metric | Value | Change |
|--------|-------|--------|
| Cold run | XXXms | +/-X% |
| Warm run | XXms | +/-X% |
| Memory | XXmb | +/-X% |

## Verification Commands

```bash
# Regression tests
cargo test --bench regression -- --nocapture

# Budget check
./scripts/perf/budget-check.sh

# Benchmark comparison
cargo bench -- --baseline main

# Full suite
make check
```

## Exit Criteria Met

- [ ] Profiling completed and documented
- [ ] Identified bottlenecks fixed or documented as deferred
- [ ] Performance budgets enforced in CI
- [ ] Regression tests passing
- [ ] Cold run < 1s (acceptable)
- [ ] Warm run < 200ms (acceptable)
- [ ] Memory < 500MB (acceptable)
- [ ] `make check` passes
```

**Verification:**
```bash
make check
./scripts/perf/budget-check.sh
cargo test --bench regression -- --nocapture
```

## Key Implementation Details

### Profiling Methodology

1. **Baseline first** - Profile before any changes to identify real bottlenecks
2. **Measure twice** - Run profiling multiple times to ensure consistency
3. **Fix one thing** - Apply one optimization, measure, commit, repeat
4. **Document decisions** - Record why optimizations were applied or deferred

### Performance Budget Tiers

| Level | Cold Run | Warm Run | Memory | Action |
|-------|----------|----------|--------|--------|
| Target | < 500ms | < 100ms | < 100MB | Ideal |
| Acceptable | < 1s | < 200ms | < 500MB | Pass CI |
| Unacceptable | > 2s | > 500ms | > 2GB | Fail CI |

### Optimization Priority

Apply optimizations only when profiling shows clear benefit:

1. **P1 (Walker)**: If file discovery > 50% of cold run time
2. **P2 (Patterns)**: If pattern matching > 40% of check time
3. **P3 (Memory)**: If peak memory > 100MB
4. **P4 (Micro)**: Only with specific profiling evidence

### Regression Prevention

Three layers of defense:

1. **Criterion baselines** - Compare to saved baselines (10% threshold)
2. **Regression tests** - Hard limits (2s cold, 500ms warm)
3. **Budget script** - Human-readable CI output

## Verification Plan

### Phase 1 Verification
```bash
./scripts/perf/profile.sh tests/fixtures/stress-monorepo
./scripts/perf/flamegraph.sh tests/fixtures/stress-monorepo
ls reports/profiling/
```

### Phase 2 Verification
```bash
# After applying fixes, compare:
hyperfine --warmup 0 --runs 5 -i \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'
```

### Phase 3 Verification
```bash
./scripts/perf/budget-check.sh
```

### Phase 4 Verification
```bash
cargo test --bench regression -- --nocapture
```

### Phase 5 Verification
```bash
cat docs/profiling.md
grep -q "Checkpoint 17E" docs/profiling.md
```

### Phase 6 (Final) Verification
```bash
make check
./scripts/perf/budget-check.sh
cargo test --bench regression -- --nocapture
cargo bench --bench cache -- --baseline main

# Generate validation report
cat reports/checkpoint-17e-validation.md
```

## Exit Criteria

- [ ] Profiling scripts created and working
- [ ] Baseline profiling report generated (`reports/profiling/baseline-profile.md`)
- [ ] Bottleneck fixes applied (or documented as deferred with justification)
- [ ] Performance budget script passing
- [ ] Regression test suite passing
- [ ] CI workflow updated with budget check
- [ ] Documentation updated with profiling results
- [ ] Cold run < 1s on bench-medium (acceptable threshold)
- [ ] Warm run < 200ms on bench-medium (acceptable threshold)
- [ ] Cache speedup >= 3x verified
- [ ] `make check` passes
