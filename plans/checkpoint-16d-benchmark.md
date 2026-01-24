# Checkpoint 16D: Benchmark - Report Command

**Plan:** `checkpoint-16d-benchmark`
**Root Feature:** `quench-report`
**Depends On:** `checkpoint-16c-refactor` (Report Command Refactor)

## Overview

Benchmark the `quench report` command to establish performance baselines and verify it meets performance targets. The report command was implemented, validated, and refactored in prior checkpoints (16A-16C); this checkpoint measures its performance characteristics across output formats and baseline sizes.

**Key Benchmarking Goals:**
1. Measure report generation time for text, JSON, and HTML formats
2. Profile formatter implementations to identify bottlenecks
3. Compare performance against target thresholds
4. Establish benchmark baselines for regression detection

## Project Structure

```
quench/
├── crates/cli/benches/
│   ├── report.rs              # NEW: Report command benchmarks
│   └── ...                    # Existing benchmarks
├── tests/fixtures/
│   ├── report/                # Existing report fixtures
│   │   ├── with-baseline/     # Standard baseline (~10 metrics)
│   │   └── no-baseline/       # Empty baseline case
│   └── bench-report/          # NEW: Scaled baseline fixtures
│       ├── minimal/           # 1 metric
│       ├── typical/           # 5 metrics (common case)
│       ├── comprehensive/     # All metrics populated
│       └── large-escapes/     # 100+ escape patterns
└── reports/
    └── checkpoint-16d-benchmarks.md  # Output: benchmark results
```

## Dependencies

Already present in `crates/cli/Cargo.toml`:
- `criterion` with `html_reports` feature - benchmarking framework

Profiling tools (system-level):
- **macOS**: `xcrun xctrace` (Time Profiler) - built into Xcode
- **Linux**: `perf` - system profiler
- **Cross-platform**: `flamegraph` crate - `cargo install flamegraph`

Optional for statistical measurement:
- `hyperfine` - command-line benchmarking (`brew install hyperfine`)

## Implementation Phases

### Phase 1: Create Benchmark Fixtures

**Goal:** Generate scaled baseline fixtures for consistent benchmarking.

**Tasks:**

1. Create minimal baseline fixture (1 metric):
   ```json
   // tests/fixtures/bench-report/minimal/.quench/baseline.json
   {
     "version": 1,
     "updated": "2026-01-24T00:00:00Z",
     "commit": "abc123",
     "metrics": {
       "coverage": { "line": 80.5, "branch": null }
     }
   }
   ```

2. Create typical baseline fixture (5 metrics):
   ```json
   // tests/fixtures/bench-report/typical/.quench/baseline.json
   {
     "version": 1,
     "updated": "2026-01-24T00:00:00Z",
     "commit": "def456",
     "metrics": {
       "coverage": { "line": 85.2, "branch": 72.1 },
       "escapes": { "unsafe": 3, "unwrap": 12, "todo": 5 },
       "binary_size": { "quench": 4521984 },
       "build_time": { "cold_ms": 45000, "hot_ms": 1200 },
       "test_time": { "total_ms": 8500, "count": 156 }
     }
   }
   ```

3. Create comprehensive baseline fixture (all metrics populated):
   - Coverage with line and branch
   - Escapes with 10+ pattern types
   - Multiple binary sizes
   - Build and test times

4. Create large-escapes fixture (stress test):
   - 100+ unique escape patterns
   - Tests map iteration performance

5. Add minimal `quench.toml` to each fixture directory.

**Verification:**
```bash
ls tests/fixtures/bench-report/*/
# Should show minimal/, typical/, comprehensive/, large-escapes/

cargo run -p quench -- report tests/fixtures/bench-report/typical
# Should produce valid text output
```

---

### Phase 2: Create Report Benchmark Suite

**Goal:** Add criterion benchmarks for report command formatters.

**Tasks:**

1. Create `crates/cli/benches/report.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use quench::baseline::Baseline;
use quench_cli::report;
use quench_cli::cli::ReportArgs;

fn load_fixture_baseline(name: &str) -> Baseline {
    let path = format!("../../tests/fixtures/bench-report/{name}/.quench/baseline.json");
    Baseline::load(&path).unwrap().expect("fixture must exist")
}

fn bench_text_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/text");

    for fixture in ["minimal", "typical", "comprehensive", "large-escapes"] {
        let baseline = load_fixture_baseline(fixture);
        let args = ReportArgs::default();

        group.bench_with_input(
            BenchmarkId::new("format", fixture),
            &baseline,
            |b, baseline| {
                b.iter(|| {
                    report::format_report(
                        OutputFormat::Text,
                        Some(black_box(baseline)),
                        &args,
                    )
                })
            },
        );
    }
    group.finish();
}

fn bench_json_format(c: &mut Criterion) {
    // Similar structure for JSON formatting
}

fn bench_html_format(c: &mut Criterion) {
    // Similar structure for HTML formatting
    // HTML expected to be slowest due to template rendering
}

fn bench_format_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/format-comparison");
    let baseline = load_fixture_baseline("typical");
    let args = ReportArgs::default();

    for format in [OutputFormat::Text, OutputFormat::Json, OutputFormat::Html] {
        group.bench_with_input(
            BenchmarkId::new("typical", format!("{:?}", format)),
            &format,
            |b, fmt| {
                b.iter(|| {
                    report::format_report(*fmt, Some(&baseline), &args)
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_text_format,
    bench_json_format,
    bench_html_format,
    bench_format_comparison,
);
criterion_main!(benches);
```

2. Register benchmark in `Cargo.toml`:
   ```toml
   [[bench]]
   name = "report"
   harness = false
   ```

**Verification:**
```bash
cargo bench --bench report -- --test
# Should compile and run without errors
```

---

### Phase 3: Run Criterion Benchmarks

**Goal:** Collect baseline benchmark data from criterion suite.

**Tasks:**

1. Build release binary:
   ```bash
   cargo build --release
   ```

2. Run report benchmarks:
   ```bash
   cargo bench --bench report
   ```

3. Record results from criterion output:
   - `report/text/*` - Text format times
   - `report/json/*` - JSON format times
   - `report/html/*` - HTML format times
   - `report/format-comparison/*` - Cross-format comparison

4. Save baseline for future comparison:
   ```bash
   cargo bench --bench report -- --save-baseline checkpoint-16d
   ```

**Output:** Criterion reports in `target/criterion/report/` with HTML reports.

**Verification:** Benchmark completes without errors, HTML reports generated.

---

### Phase 4: Profile Report Generation

**Goal:** Identify performance bottlenecks in formatters.

**Tasks:**

1. Profile HTML formatter (typically slowest):
   ```bash
   cargo flamegraph --bench report -- --bench 'html/format/comprehensive'
   ```

2. Examine flamegraph for:
   - String allocation overhead
   - Template rendering time
   - Metric iteration cost
   - Serialization overhead (JSON)

3. Use hyperfine for end-to-end measurement:
   ```bash
   hyperfine --warmup 3 \
     './target/release/quench report tests/fixtures/bench-report/typical' \
     './target/release/quench report tests/fixtures/bench-report/typical -o json' \
     './target/release/quench report tests/fixtures/bench-report/typical -o html'
   ```

4. Document any bottlenecks found:
   - Memory allocation patterns
   - String concatenation in HTML builder
   - JSON serialization overhead

**Key areas to examine:**
- `HtmlFormatter::format()` - Template string building
- `JsonFormatter::format()` - serde_json serialization
- `FilteredMetrics` iteration - Metric filtering overhead

**Verification:** Flamegraph generated, bottlenecks documented.

---

### Phase 5: Measure Against Performance Targets

**Goal:** Verify report command meets performance expectations.

**Performance Targets for Report Command:**

| Scenario | Target | Acceptable | Unacceptable |
|----------|--------|------------|--------------|
| Text format (typical) | < 1ms | < 5ms | > 10ms |
| JSON format (typical) | < 2ms | < 10ms | > 20ms |
| HTML format (typical) | < 5ms | < 20ms | > 50ms |
| Large baseline (100+ metrics) | < 20ms | < 50ms | > 100ms |

**Rationale:**
- Report command is I/O-bound (baseline read) not CPU-bound
- Formatting should be negligible compared to file I/O
- Even HTML with inline CSS should be sub-millisecond generation

**Tasks:**

1. Extract timing from criterion output for each scenario.

2. Compare against targets:
   ```
   Text typical: Xμs (target: <1ms) ✓/✗
   JSON typical: Xμs (target: <2ms) ✓/✗
   HTML typical: Xμs (target: <5ms) ✓/✗
   ```

3. If any target exceeded, prioritize optimization.

**Verification:** All targets documented with measured values.

---

### Phase 6: Document Results

**Goal:** Write comprehensive benchmark report.

**Output file:** `reports/checkpoint-16d-benchmarks.md`

**Report structure:**
```markdown
# Checkpoint 16D: Report Command Benchmarks

Generated: YYYY-MM-DD

## Summary

| Format | Fixture | Target | Measured | Status |
|--------|---------|--------|----------|--------|
| Text | typical | <1ms | Xμs ± Y | ✓/✗ |
| JSON | typical | <2ms | Xμs ± Y | ✓/✗ |
| HTML | typical | <5ms | Xμs ± Y | ✓/✗ |
| HTML | large-escapes | <20ms | Xμs ± Y | ✓/✗ |

## Detailed Results

### Criterion Benchmarks
[Benchmark output summary with statistical analysis]

### Format Comparison
[Graph or table showing relative performance: text < json < html]

### Profiling Findings
[Flamegraph analysis, memory allocation patterns]

## Bottlenecks Identified

[Ordered list of any performance issues found]

1. [None expected - report is simple string formatting]

## Recommendations

[Optimization suggestions if targets not met]

## Environment

- Platform: [OS version]
- CPU: [model, cores]
- Rust version: [rustc --version]
- Build profile: release (LTO enabled)
```

**Verification:** Report written with all sections complete.

## Key Implementation Details

### Benchmark Configuration

```rust
// In benches/report.rs
use criterion::{Criterion, SamplingMode};

fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)           // 100 samples for statistical significance
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(1))
        .sampling_mode(SamplingMode::Auto)
}
```

### ReportArgs Default for Benchmarks

```rust
impl Default for ReportArgs {
    fn default() -> Self {
        Self {
            output: None,       // stdout
            cloc: false,
            no_cloc: false,
            // ... all filter flags false (include all)
        }
    }
}
```

### Expected Performance Characteristics

| Component | Expected Time | Notes |
|-----------|--------------|-------|
| Baseline load | ~1ms | File I/O dominates |
| Text format | <100μs | Simple string building |
| JSON format | <500μs | serde serialization |
| HTML format | <2ms | Template string concatenation |

### Commands Summary

```bash
# Build release binary
cargo build --release

# Run all report benchmarks
cargo bench --bench report

# Run specific benchmark
cargo bench --bench report -- 'html/format'

# Save baseline for regression detection
cargo bench --bench report -- --save-baseline checkpoint-16d

# Compare against previous baseline
cargo bench --bench report -- --baseline checkpoint-16d

# Generate flamegraph
cargo flamegraph --bench report -- --bench 'html/format/comprehensive'

# Hyperfine for CLI measurement
hyperfine --warmup 3 './target/release/quench report tests/fixtures/bench-report/typical'
```

## Verification Plan

### Per-Phase Verification

Each phase includes specific verification commands that must pass before proceeding.

### Full Verification Checklist

After all phases complete:

```bash
# 1. Fixtures exist
ls tests/fixtures/bench-report/*/

# 2. Benchmarks compile and run
cargo bench --bench report -- --test

# 3. Full benchmark run
cargo bench --bench report

# 4. Report document exists
cat reports/checkpoint-16d-benchmarks.md

# 5. All existing tests still pass
make check
```

### Success Criteria

- [ ] Benchmark fixtures created in `tests/fixtures/bench-report/`
- [ ] `benches/report.rs` benchmark suite implemented
- [ ] Criterion benchmarks run without errors
- [ ] All format times measured and documented
- [ ] Performance targets compared against measured values
- [ ] Profiling completed, bottlenecks documented (if any)
- [ ] Report written to `reports/checkpoint-16d-benchmarks.md`
- [ ] `make check` passes (no regressions)

## Deliverables

1. **Benchmark Fixtures:** `tests/fixtures/bench-report/` with scaled baselines
2. **Benchmark Suite:** `crates/cli/benches/report.rs` with criterion benchmarks
3. **Benchmark Report:** `reports/checkpoint-16d-benchmarks.md` with results
4. **Saved Baseline:** Criterion baseline for future regression detection
