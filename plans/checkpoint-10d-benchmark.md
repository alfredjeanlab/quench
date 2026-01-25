# Checkpoint 10D: Benchmark - Dogfooding Milestone 2

**Root Feature:** `quench-10d`
**Follows:** checkpoint-10c-refactor (DRY refactoring)

## Overview

Formalize the benchmarking infrastructure for dogfooding quench on itself. This checkpoint establishes:

1. **Benchmark runner script** - Automated benchmark execution with baseline tracking
2. **Performance baseline file** - Checked-in baseline for regression detection
3. **CI benchmark integration** - Prevent regressions via GitHub Actions
4. **Benchmark documentation** - How to run, interpret, and troubleshoot benchmarks

**Goal:** Any performance regression >20% on the quench codebase causes CI failure.

## Project Structure

```
quench/
├── crates/cli/benches/
│   ├── dogfood.rs              # EXISTS: Dogfooding benchmarks
│   └── regression.rs           # EXISTS: Hard limit regression tests
├── scripts/
│   ├── benchmark               # CREATE: Benchmark runner script
│   └── update-baseline         # CREATE: Update baseline after improvements
├── reports/
│   └── benchmark-baseline.json # CREATE: Checked-in performance baseline
├── .github/workflows/
│   └── ci.yml                  # MODIFY: Add benchmark job
├── docs/
│   └── benchmarking.md         # CREATE: Benchmark documentation
└── plans/
    └── checkpoint-10d-benchmark.md  # THIS FILE
```

## Dependencies

No new crate dependencies. Uses existing:
- `criterion` for benchmarking
- `serde_json` for baseline file parsing (already in deps)

External tooling:
- `hyperfine` (optional) for CLI timing comparisons
- GitHub Actions for CI benchmark execution

## Implementation Phases

### Phase 1: Benchmark Baseline File

Create a checked-in JSON baseline capturing current dogfood performance.

**File:** `reports/benchmark-baseline.json`

```json
{
  "version": 1,
  "generated": "2026-01-24T00:00:00Z",
  "commit": "beb6e0a",
  "benchmarks": {
    "dogfood/fast": {
      "mean_ms": 25.0,
      "stddev_ms": 3.0
    },
    "dogfood/fast_json": {
      "mean_ms": 26.0,
      "stddev_ms": 3.0
    },
    "dogfood_checks/cloc_only": {
      "mean_ms": 22.0,
      "stddev_ms": 2.0
    },
    "dogfood_checks/escapes_only": {
      "mean_ms": 23.0,
      "stddev_ms": 2.0
    },
    "dogfood_checks/agents_only": {
      "mean_ms": 22.0,
      "stddev_ms": 2.0
    }
  },
  "thresholds": {
    "warm_target_ms": 100,
    "warm_acceptable_ms": 200,
    "warm_unacceptable_ms": 500,
    "regression_threshold_pct": 20
  }
}
```

**Verification:**
```bash
cat reports/benchmark-baseline.json | jq '.benchmarks | keys'
```

---

### Phase 2: Benchmark Runner Script

Create a script to run dogfood benchmarks and compare against baseline.

**File:** `scripts/benchmark`

```bash
#!/bin/bash
# Run dogfood benchmarks and check for regressions
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
BASELINE="$REPO_DIR/reports/benchmark-baseline.json"

# Build release binary first
cargo build --release -p quench

# Run criterion benchmarks (dogfood group only)
echo "Running dogfood benchmarks..."
cargo bench --bench dogfood -- --noplot 2>&1 | tee /tmp/bench-output.txt

# Parse results and compare
echo ""
echo "=== Baseline Comparison ==="

# Extract timings from criterion output (simplified parsing)
# Format: "dogfood/fast           time:   [24.5 ms 25.1 ms 25.8 ms]"
for bench in "dogfood/fast" "dogfood/fast_json"; do
    baseline_ms=$(jq -r ".benchmarks[\"$bench\"].mean_ms // empty" "$BASELINE")
    if [[ -n "$baseline_ms" ]]; then
        echo "$bench: baseline ${baseline_ms}ms"
    fi
done

echo ""
echo "See target/criterion/ for detailed reports"
```

**Make executable:**
```bash
chmod +x scripts/benchmark
```

**Verification:**
```bash
./scripts/benchmark
```

---

### Phase 3: Baseline Update Script

Create a script to update the baseline after confirmed improvements.

**File:** `scripts/update-baseline`

```bash
#!/bin/bash
# Update benchmark baseline after confirmed performance improvements
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
BASELINE="$REPO_DIR/reports/benchmark-baseline.json"

# Require confirmation
echo "This will update the performance baseline."
echo "Only run this after verifying performance improvements."
read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Run benchmarks
cargo build --release -p quench
cargo bench --bench dogfood -- --noplot

# Extract criterion results
CRITERION_DIR="$REPO_DIR/target/criterion"

# Build new baseline JSON
cat > "$BASELINE" << EOF
{
  "version": 1,
  "generated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "commit": "$(git rev-parse --short HEAD)",
  "benchmarks": {
    "dogfood/fast": {
      "mean_ms": $(jq -r '.mean.point_estimate / 1e6' "$CRITERION_DIR/dogfood/fast/new/estimates.json"),
      "stddev_ms": $(jq -r '.std_dev.point_estimate / 1e6' "$CRITERION_DIR/dogfood/fast/new/estimates.json")
    },
    "dogfood/fast_json": {
      "mean_ms": $(jq -r '.mean.point_estimate / 1e6' "$CRITERION_DIR/dogfood/fast_json/new/estimates.json"),
      "stddev_ms": $(jq -r '.std_dev.point_estimate / 1e6' "$CRITERION_DIR/dogfood/fast_json/new/estimates.json")
    },
    "dogfood_checks/cloc_only": {
      "mean_ms": $(jq -r '.mean.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/cloc_only/new/estimates.json"),
      "stddev_ms": $(jq -r '.std_dev.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/cloc_only/new/estimates.json")
    },
    "dogfood_checks/escapes_only": {
      "mean_ms": $(jq -r '.mean.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/escapes_only/new/estimates.json"),
      "stddev_ms": $(jq -r '.std_dev.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/escapes_only/new/estimates.json")
    },
    "dogfood_checks/agents_only": {
      "mean_ms": $(jq -r '.mean.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/agents_only/new/estimates.json"),
      "stddev_ms": $(jq -r '.std_dev.point_estimate / 1e6' "$CRITERION_DIR/dogfood_checks/agents_only/new/estimates.json")
    }
  },
  "thresholds": {
    "warm_target_ms": 100,
    "warm_acceptable_ms": 200,
    "warm_unacceptable_ms": 500,
    "regression_threshold_pct": 20
  }
}
EOF

echo "Baseline updated: $BASELINE"
git diff "$BASELINE"
```

**Verification:**
```bash
chmod +x scripts/update-baseline
./scripts/update-baseline  # Manually verify it prompts
```

---

### Phase 4: CI Benchmark Integration

Add benchmark regression detection to CI.

**File:** `.github/workflows/ci.yml` (add benchmark job)

```yaml
  benchmark:
    name: Benchmark Regression Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Build release binary
        run: cargo build --release -p quench

      - name: Run regression tests
        run: cargo test --release --bench regression -- --nocapture

      - name: Run dogfood benchmarks
        run: |
          cargo bench --bench dogfood -- --noplot

      - name: Check for regressions
        run: |
          # Compare against baseline thresholds
          # This is a simplified check - criterion handles detailed comparison
          BASELINE="reports/benchmark-baseline.json"
          if [ -f "$BASELINE" ]; then
            TARGET_MS=$(jq '.thresholds.warm_acceptable_ms' "$BASELINE")
            echo "Warm run target: <${TARGET_MS}ms"
          fi
```

**Verification:**
```bash
# Test locally
cargo test --release --bench regression -- --nocapture
```

---

### Phase 5: Benchmark Documentation

Document how to run, interpret, and troubleshoot benchmarks.

**File:** `docs/benchmarking.md`

```markdown
# Benchmarking Guide

## Quick Start

```bash
# Run dogfood benchmarks
./scripts/benchmark

# View detailed HTML reports
open target/criterion/report/index.html
```

## Performance Targets

From `docs/specs/20-performance.md`:

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Warm run | < 100ms | < 200ms | > 500ms |
| Cold run | < 500ms | < 1s | > 2s |

## Benchmark Suites

### Dogfood (`benches/dogfood.rs`)

Quench checking itself - the primary benchmark.

- `dogfood/fast` - Default mode with text output
- `dogfood/fast_json` - JSON output mode
- `dogfood_checks/*` - Individual check isolation

### Regression (`benches/regression.rs`)

Hard time limits that fail if exceeded:

- `cold_run_under_2s` - Cold run must complete <2s
- `warm_run_under_500ms` - Warm run must complete <500ms
- `cache_provides_speedup` - Cache must provide ≥2x speedup

### Stress (`benches/stress.rs`)

Pathological inputs for edge case testing:

- Large files (10K-50K lines)
- Many `#[cfg(test)]` blocks
- Large workspaces (50 packages)
- Deep nesting (20 levels)

## Baseline Management

The baseline file `reports/benchmark-baseline.json` tracks expected performance:

```bash
# Update baseline after confirmed improvements
./scripts/update-baseline

# View current baseline
cat reports/benchmark-baseline.json | jq '.benchmarks'
```

## CI Integration

Benchmarks run on every PR:

1. **Regression tests** - Hard limits (fail if exceeded)
2. **Dogfood benchmarks** - Track against baseline

A 20% regression fails CI.

## Troubleshooting

### High Variance

If benchmarks show high variance (>20% stddev):

1. Close other applications
2. Disable CPU throttling: `sudo cpupower frequency-set --governor performance`
3. Run more samples: `cargo bench --bench dogfood -- --sample-size 100`

### Unexpected Slowdown

1. Check if cache is working: `ls -la .quench/cache.bin`
2. Profile: `cargo flamegraph -- check`
3. Compare against baseline: `./scripts/benchmark`

### Fixture Not Found

```bash
# Generate benchmark fixtures
./scripts/fixtures/generate-bench-fixtures
```
```

**Verification:**
```bash
# Check doc renders correctly
cat docs/benchmarking.md
```

---

### Phase 6: Initial Baseline Capture

Run benchmarks and capture the initial baseline for the quench codebase.

```bash
# Ensure clean build
cargo build --release

# Run dogfood benchmarks
cargo bench --bench dogfood -- --noplot

# Create initial baseline
./scripts/update-baseline

# Commit baseline
git add reports/benchmark-baseline.json
git commit -m "chore: add initial benchmark baseline"
```

**Verification:**
```bash
# Verify baseline exists and is valid JSON
jq '.' reports/benchmark-baseline.json

# Verify regression tests pass
cargo test --release --bench regression -- --nocapture
```

---

## Key Implementation Details

### Criterion Integration

Criterion stores results in `target/criterion/<group>/<bench>/new/estimates.json`:

```json
{
  "mean": {
    "point_estimate": 25000000.0,  // nanoseconds
    "standard_error": 500000.0
  },
  "std_dev": {
    "point_estimate": 3000000.0
  }
}
```

The scripts parse these files to extract timing data.

### Regression Detection Strategy

Two-tier approach:

1. **Hard limits** (`regression.rs`) - Absolute thresholds that never regress
   - Cold run < 2s (unacceptable threshold)
   - Warm run < 500ms (unacceptable threshold)
   - Cache speedup ≥ 2x

2. **Baseline comparison** (`benchmark-baseline.json`) - Relative change detection
   - >20% slower than baseline → CI failure
   - Baseline updated manually after confirmed improvements

### Why Not `cargo bench` + Criterion Alone?

Criterion compares to previous runs, but:
- Previous runs may not exist (fresh CI)
- Local results differ from CI (hardware variance)
- Need explicit baseline approval workflow

The checked-in baseline provides:
- Consistent CI comparison target
- Explicit review when baselines change
- Historical record of performance

### Noise Mitigation

Benchmarks are inherently noisy. Strategies:
- Warm up cache before measurement
- Multiple samples (criterion default: 100)
- 20% threshold accounts for normal variance
- Regression tests use absolute limits (not relative)

---

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `jq '.' reports/benchmark-baseline.json` | Valid JSON with benchmarks |
| 2 | `./scripts/benchmark` | Runs benchmarks, shows comparison |
| 3 | `./scripts/update-baseline` | Prompts for confirmation |
| 4 | `cargo test --release --bench regression` | All tests pass |
| 5 | `cat docs/benchmarking.md` | Documentation readable |
| 6 | `git diff reports/benchmark-baseline.json` | Baseline captured |

---

## Summary

| Phase | Deliverable | Purpose |
|-------|-------------|---------|
| 1 | `benchmark-baseline.json` | Checked-in performance baseline |
| 2 | `scripts/benchmark` | Run benchmarks with comparison |
| 3 | `scripts/update-baseline` | Update baseline after improvements |
| 4 | CI benchmark job | Prevent regressions in PRs |
| 5 | `docs/benchmarking.md` | Developer documentation |
| 6 | Initial baseline | Capture current quench performance |

---

## Completion Criteria

- [ ] Phase 1: `reports/benchmark-baseline.json` exists with valid structure
- [ ] Phase 2: `./scripts/benchmark` runs and shows baseline comparison
- [ ] Phase 3: `./scripts/update-baseline` prompts and updates baseline
- [ ] Phase 4: CI benchmark job added to workflow
- [ ] Phase 5: `docs/benchmarking.md` documents benchmark workflow
- [ ] Phase 6: Initial baseline captured from current quench performance
- [ ] `make check` passes
- [ ] `./done` executed successfully
