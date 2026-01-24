# Checkpoint 8D: Benchmark - Tests Correlation

**Root Feature:** `quench-03b8`

## Overview

Create performance benchmarks for the tests correlation check to establish baselines, validate against performance targets, and enable regression tracking. This follows the established benchmark checkpoint pattern (6D, 7D, etc.) with Criterion-based benchmarks and stress test fixtures.

## Project Structure

```
quench/
├── crates/cli/
│   └── benches/
│       └── tests.rs              # NEW: Criterion benchmarks
├── tests/fixtures/
│   ├── bench-tests-small/        # NEW: 10 files, basic correlation
│   ├── bench-tests-medium/       # NEW: 50 files, mixed patterns
│   ├── bench-tests-large/        # NEW: 500 files, complex scenarios
│   └── bench-tests-worst-case/   # NEW: Pathological patterns
└── reports/
    └── checkpoint-8d-benchmarks.md  # NEW: Results analysis
```

## Dependencies

Already available in the project:
- `criterion = "0.5"` - Benchmarking framework (dev-dependency)
- `globset` - Test pattern matching
- `ignore` - Parallel file walking
- Git CLI - Diff parsing

No new dependencies required.

## Implementation Phases

### Phase 1: Create Benchmark Fixtures

Create test fixtures of varying sizes for reproducible benchmarks.

**Files:**
- `tests/fixtures/bench-tests-small/` - 10 source files with test counterparts
- `tests/fixtures/bench-tests-medium/` - 50 files with mixed patterns (inline tests, separate test files)
- `tests/fixtures/bench-tests-large/` - 500 files simulating a real project
- `tests/fixtures/bench-tests-worst-case/` - Deep nesting, many test patterns

**Fixture structure:**
```
bench-tests-small/
├── quench.toml
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── parser.rs
│   ├── lexer.rs
│   └── ...
└── tests/
    ├── parser_tests.rs
    ├── lexer_tests.rs
    └── ...
```

**Verification:** Fixtures exist and `quench check` runs without errors on each.

### Phase 2: Add Criterion Benchmark Harness

Create the benchmark file with basic setup and correlation detection benchmarks.

**File:** `crates/cli/benches/tests.rs`

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;

fn bench_correlation_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-correlation");

    for (name, fixture) in [
        ("small", "bench-tests-small"),
        ("medium", "bench-tests-medium"),
        ("large", "bench-tests-large"),
        ("worst-case", "bench-tests-worst-case"),
    ] {
        let path = PathBuf::from(format!("tests/fixtures/{fixture}"));
        group.throughput(Throughput::Elements(file_count(&path)));
        group.bench_with_input(
            BenchmarkId::new("detect", name),
            &path,
            |b, path| {
                b.iter(|| {
                    // Run correlation detection
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_correlation_detection);
criterion_main!(benches);
```

**Update:** Add to `crates/cli/Cargo.toml`:
```toml
[[bench]]
name = "tests"
harness = false
```

**Verification:** `cargo bench --bench tests -- --list` shows benchmarks.

### Phase 3: Benchmark Core Operations

Add benchmarks for individual operations to identify bottlenecks.

**Operations to benchmark:**

1. **Candidate path generation** - `candidate_test_paths()` for various source files
2. **Glob pattern matching** - Finding test files using globset
3. **Inline test detection** - Parsing `#[cfg(test)]` blocks in Rust files
4. **Placeholder detection** - Finding `#[ignore = "TODO"]` patterns
5. **Git diff parsing** - Processing `git diff` output for change detection

```rust
fn bench_candidate_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-candidate-paths");

    // Various path patterns to exercise matching logic
    let paths = [
        "src/lib.rs",
        "src/parser.rs",
        "src/checks/tests/correlation.rs",
        "crates/cli/src/main.rs",
    ];

    for path in paths {
        group.bench_function(path, |b| {
            b.iter(|| candidate_test_paths(Path::new(path)));
        });
    }
    group.finish();
}

fn bench_inline_test_detection(c: &mut Criterion) {
    // Benchmark detecting #[cfg(test)] in files of various sizes
}

fn bench_diff_parsing(c: &mut Criterion) {
    // Benchmark parsing git diff output
}
```

**Verification:** All benchmark functions run without errors.

### Phase 4: End-to-End CLI Benchmarks

Benchmark the full `quench check` flow with test correlation enabled.

**Scenarios:**
1. Clean state (no changes) - baseline
2. Single file change - typical case
3. Many file changes - stress case
4. Branch scope vs commit scope

```rust
fn bench_cli_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("tests-cli");
    group.sample_size(20); // Fewer samples for slower benchmarks

    for (name, fixture) in fixtures() {
        // Cold run (no cache)
        group.bench_function(BenchmarkId::new("cold", name), |b| {
            b.iter_custom(|iters| {
                // Clear cache, run check, measure wall time
            });
        });

        // Warm run (cached)
        group.bench_function(BenchmarkId::new("warm", name), |b| {
            b.iter(|| {
                // Run check with existing cache
            });
        });
    }
    group.finish();
}
```

**Verification:** CLI benchmarks complete and produce timing data.

### Phase 5: Validate Performance Targets

Run benchmarks and compare against targets from `docs/specs/20-performance.md`:
- Fast check (cold): < 500ms
- Fast check (warm): < 100ms
- CI check: < 5s

**Process:**
1. Run full benchmark suite: `cargo bench --bench tests`
2. Capture baseline metrics
3. Compare against targets
4. Identify any regressions or bottlenecks

**File:** `reports/checkpoint-8d-benchmarks.md`

```markdown
# Benchmark Results: Tests Correlation Check

## Summary

| Fixture | Cold (ms) | Warm (ms) | Target | Status |
|---------|-----------|-----------|--------|--------|
| small   | XX        | XX        | <100ms | PASS   |
| medium  | XX        | XX        | <100ms | PASS   |
| large   | XX        | XX        | <500ms | PASS   |

## Detailed Analysis

### Correlation Detection
...

### Git Integration
...

### Bottlenecks Identified
...
```

**Verification:** Report complete with all metrics captured.

### Phase 6: CI Integration

Ensure benchmarks run in CI for regression tracking.

**Files to verify/update:**
- `.github/workflows/bench.yml` - Add tests benchmark
- `scripts/bench-ci` - Include tests in benchmark runner

**Verification:** `make check` passes, CI workflow includes tests benchmark.

## Key Implementation Details

### Fixture Generation Strategy

Fixtures simulate realistic project structures:
- **Small:** Single crate, basic src/tests layout
- **Medium:** Multi-module, mix of inline and separate tests
- **Large:** Workspace-like structure, deep nesting
- **Worst-case:** Many glob patterns to match, edge cases

### Benchmark Isolation

Each benchmark run should:
1. Use `iter_custom()` for I/O-heavy operations
2. Clear caches between cold runs
3. Use consistent git state (committed fixtures)
4. Avoid filesystem noise from concurrent processes

### Metrics to Track

1. **Time:** Wall clock duration for operations
2. **Throughput:** Files processed per second
3. **Memory:** Allocations during correlation detection (optional)
4. **Git calls:** Number of subprocess invocations

## Verification Plan

1. **Phase 1:** Run `ls tests/fixtures/bench-tests-*` to confirm fixture creation
2. **Phase 2:** Run `cargo bench --bench tests -- --list` to verify benchmark registration
3. **Phase 3:** Run `cargo bench --bench tests -- bench_candidate_paths` to test core ops
4. **Phase 4:** Run `cargo bench --bench tests -- bench_cli` to test end-to-end
5. **Phase 5:** Review `reports/checkpoint-8d-benchmarks.md` for completeness
6. **Phase 6:** Run `make check` to verify CI compatibility

**Final verification:**
```bash
cargo bench --bench tests
# Review output, all benchmarks complete without error
# Check reports/checkpoint-8d-benchmarks.md has baseline metrics
```
