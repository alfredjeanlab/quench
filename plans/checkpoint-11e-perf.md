# Checkpoint 11E: Performance - Tests CI Mode

## Overview

Optimize tests CI mode performance based on benchmarks established in checkpoint 11d. This checkpoint focuses on reducing overhead in test suite execution, output parsing, and metrics aggregation. Key optimizations include parallel suite execution, streaming output parsing, and early termination in fast mode.

**Follows:** checkpoint-11d-benchmark (CI Mode Benchmarks)

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── mod.rs                          # MODIFY: Parallel suite execution
│   └── runners/
│       ├── mod.rs                      # MODIFY: Add parallel execution helpers
│       ├── cargo.rs                    # MODIFY: Optimize parsing
│       └── result.rs                   # UNCHANGED
├── crates/cli/benches/
│   ├── tests_ci.rs                     # MODIFY: Add optimization benchmarks
│   └── regression.rs                   # MODIFY: Add CI mode regression test
└── reports/
    └── benchmark-baseline.json         # MODIFY: Update with CI mode baselines
```

## Dependencies

No new dependencies. Uses existing:
- `rayon` for parallel execution (already in workspace deps)
- `std::time::Instant` for timing measurements

## Implementation Phases

### Phase 1: Parallel Suite Execution

**Goal:** Execute independent test suites in parallel in CI mode.

Currently, suites run sequentially:
```rust
for suite in active_suites {
    // ... run suite
}
```

**File:** `crates/cli/src/checks/tests/mod.rs`

Modify `run_suites()` to parallelize:

```rust
use rayon::prelude::*;

impl TestsCheck {
    fn run_suites(&self, ctx: &CheckContext) -> Option<SuiteResults> {
        let suites = &ctx.config.check.tests.suite;
        if suites.is_empty() {
            return None;
        }

        let runner_ctx = RunnerContext {
            root: ctx.root,
            ci_mode: ctx.ci_mode,
            collect_coverage: ctx.ci_mode,
        };

        let active_suites = filter_suites_for_mode(suites, ctx.ci_mode);
        if active_suites.is_empty() {
            return None;
        }

        // Parallel execution in CI mode, sequential in fast mode
        let results: Vec<SuiteResult> = if ctx.ci_mode && active_suites.len() > 1 {
            active_suites
                .par_iter()
                .map(|suite| self.run_single_suite(suite, &runner_ctx))
                .collect()
        } else {
            active_suites
                .iter()
                .map(|suite| self.run_single_suite(suite, &runner_ctx))
                .collect()
        };

        let all_passed = results.iter().all(|r| r.passed || r.skipped);
        Some(SuiteResults {
            passed: all_passed,
            suites: results,
        })
    }

    fn run_single_suite(&self, suite: &TestSuiteConfig, ctx: &RunnerContext) -> SuiteResult {
        // ... extracted from current for loop body
    }
}
```

**Note:** Parallel execution only in CI mode because:
- CI mode typically has multiple suites (cargo, bats, integration tests)
- Fast mode typically runs single default suite
- Parallel overhead not worth it for single suite

**Verification:**
```bash
cargo test --lib tests_check
cargo bench --bench tests_ci -- tests_ci
```

### Phase 2: Optimize Cargo Output Parsing

**Goal:** Reduce allocations in `parse_cargo_output()`.

Current implementation does multiple string allocations per line. Optimize by:
1. Pre-allocating test vector based on estimated count
2. Using string slices where possible
3. Early termination on failure in fast mode

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

```rust
/// Parse cargo test human-readable output with optimizations.
pub fn parse_cargo_output(stdout: &str, total_time: Duration) -> TestRunResult {
    // Pre-count lines starting with "test " for capacity hint
    let test_line_count = stdout
        .lines()
        .filter(|l| l.trim_start().starts_with("test "))
        .count();

    let mut tests = Vec::with_capacity(test_line_count);
    let mut suite_passed = true;

    for line in stdout.lines() {
        let line = line.trim();

        // Parse individual test results
        if let Some(rest) = line.strip_prefix("test ") {
            if let Some((name, result)) = parse_test_line(rest) {
                let passed = result == "ok";
                tests.push(if passed {
                    TestResult::passed(name, Duration::ZERO)
                } else {
                    TestResult::failed(name, Duration::ZERO)
                    suite_passed = false;
                });
            }
        } else if line.starts_with("test result: ") && line.contains("FAILED") {
            suite_passed = false;
        }
    }

    let mut result = if suite_passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

/// Parse a test line after "test " prefix.
/// Returns (name, result) where result is "ok" or "FAILED".
#[inline]
fn parse_test_line(rest: &str) -> Option<(&str, &str)> {
    // Format: "<name> ... ok" or "<name> ... FAILED"
    let sep_pos = rest.rfind(" ... ")?;
    let name = &rest[..sep_pos];
    let result = &rest[sep_pos + 5..]; // Skip " ... "
    if result == "ok" || result == "FAILED" {
        Some((name, result))
    } else {
        None
    }
}
```

**Verification:**
```bash
cargo bench --bench tests_ci -- tests_metrics_parsing
```

### Phase 3: Fast Mode Early Termination

**Goal:** Stop test execution on first failure in fast mode (non-CI).

In fast mode, the first failure provides enough signal - no need to run remaining suites.

**File:** `crates/cli/src/checks/tests/mod.rs`

```rust
fn run_suites(&self, ctx: &CheckContext) -> Option<SuiteResults> {
    // ... setup code ...

    let mut results = Vec::with_capacity(active_suites.len());
    let mut all_passed = true;

    if ctx.ci_mode && active_suites.len() > 1 {
        // Parallel execution for CI mode
        results = active_suites
            .par_iter()
            .map(|suite| self.run_single_suite(suite, &runner_ctx))
            .collect();
        all_passed = results.iter().all(|r| r.passed || r.skipped);
    } else {
        // Sequential with early termination for fast mode
        for suite in active_suites {
            let result = self.run_single_suite(suite, &runner_ctx);
            let failed = !result.passed && !result.skipped;
            results.push(result);

            // Early termination in fast mode on first failure
            if failed && !ctx.ci_mode {
                all_passed = false;
                break;
            }
            if failed {
                all_passed = false;
            }
        }
    }

    Some(SuiteResults {
        passed: all_passed,
        suites: results,
    })
}
```

**Verification:**
```bash
cargo bench --bench tests_ci -- tests_ci/fast
```

### Phase 4: Reduce Coverage Collection Overhead

**Goal:** Defer coverage collection when not needed and optimize when it is.

Coverage is expensive because it requires:
1. Running `cargo llvm-cov` or similar
2. Parsing coverage reports

**Optimization strategies:**

1. Only collect coverage when thresholds are configured
2. Cache coverage collection command availability check
3. Use JSON output directly instead of parsing text

**File:** `crates/cli/src/checks/tests/runners/coverage.rs`

```rust
use std::sync::OnceLock;

// Cache llvm-cov availability to avoid repeated checks
static LLVM_COV_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if llvm-cov is available (cached).
pub fn llvm_cov_available() -> bool {
    *LLVM_COV_AVAILABLE.get_or_init(|| {
        Command::new("cargo")
            .args(["llvm-cov", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Collect Rust coverage with optimizations.
pub fn collect_rust_coverage(root: &Path, path: Option<&str>) -> CoverageResult {
    // Skip if llvm-cov not available
    if !llvm_cov_available() {
        return CoverageResult::unavailable("cargo-llvm-cov not installed");
    }

    // Use JSON output for faster parsing
    let mut cmd = Command::new("cargo");
    cmd.args(["llvm-cov", "--json", "--quiet"]);

    // ... rest of implementation
}
```

**Verification:**
```bash
cargo bench --bench tests_ci -- ci_overhead
```

### Phase 5: Add Regression Tests for CI Mode

**Goal:** Ensure CI mode performance doesn't regress.

**File:** `crates/cli/benches/regression.rs`

Add tighter regression tests based on established baselines:

```rust
/// CI mode on tests-ci fixture should have minimal overhead vs fast mode.
///
/// The overhead comes from:
/// - Running actual tests (vs just correlation checking)
/// - Coverage collection (if configured)
///
/// This test ensures the overhead stays bounded.
#[test]
fn tests_ci_mode_overhead_bounded() {
    let path = fixture_path("tests-ci");
    if !path.exists() {
        eprintln!("Skipping: tests-ci fixture not found");
        return;
    }

    let bin = quench_bin();
    if !bin.exists() {
        eprintln!("Skipping: release binary not found");
        return;
    }

    // Fast mode time (correlation only)
    let fast_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents"])
        .current_dir(&path)
        .output()
        .expect("fast mode should run");
    let fast_time = fast_start.elapsed();

    // CI mode time (run tests + metrics)
    let ci_start = Instant::now();
    Command::new(&bin)
        .args(["check", "--tests", "--no-cloc", "--no-escapes", "--no-agents", "--ci"])
        .current_dir(&path)
        .output()
        .expect("CI mode should run");
    let ci_time = ci_start.elapsed();

    eprintln!("Fast: {:?}, CI: {:?}", fast_time, ci_time);

    // CI overhead should be less than 200% of fast mode
    // (CI runs actual tests, so some overhead is expected)
    let overhead_pct = (ci_time.as_millis() as f64 / fast_time.as_millis().max(1) as f64) * 100.0;
    eprintln!("CI overhead: {:.1}%", overhead_pct - 100.0);

    assert!(
        ci_time < fast_time * 3,
        "CI mode overhead too high: {:?} vs {:?} ({:.1}%)",
        ci_time,
        fast_time,
        overhead_pct - 100.0
    );
}
```

**Verification:**
```bash
cargo test --bench regression -- tests_ci
```

### Phase 6: Update Baselines and Final Verification

**Goal:** Update benchmark baselines with new CI mode benchmarks and verify all optimizations.

**Steps:**

1. Run all benchmarks to establish new baselines:
```bash
cargo bench --bench tests_ci
cargo bench --bench dogfood
```

2. Update baseline file with tests CI metrics:
```bash
./scripts/update-baseline
```

3. Full verification:
```bash
# All tests
cargo test --all

# All benchmarks compile and run
cargo build --benches
cargo bench --bench tests_ci -- --test
cargo bench --bench regression -- --test

# Full check
make check
```

**Verification:**
```bash
make check
```

## Key Implementation Details

### Parallelization Strategy

| Mode | Suite Count | Strategy | Rationale |
|------|-------------|----------|-----------|
| Fast | 1 | Sequential | No parallel overhead |
| Fast | 2+ | Sequential + early termination | First failure is signal enough |
| CI | 1 | Sequential | No parallel overhead |
| CI | 2+ | Parallel (rayon) | Multiple suites benefit from concurrency |

### Expected Performance Improvements

| Optimization | Expected Impact | Measurement |
|--------------|-----------------|-------------|
| Parallel suites | 30-50% faster (2+ suites) | `tests_ci/ci` benchmark |
| Parsing optimization | 10-20% faster parsing | `tests_metrics_parsing` benchmark |
| Early termination | Variable (on failure) | Fast mode failure scenarios |
| Coverage caching | 5-10% faster repeated runs | `ci_overhead` benchmark |

### Backward Compatibility

All optimizations are internal and don't change:
- CLI flags or behavior
- Output format (text or JSON)
- Metrics structure
- Configuration options

### Thread Safety

The `TestRunner` trait is already `Send + Sync`, so parallel execution is safe. Each runner:
- Uses its own process for test execution
- Has no shared mutable state
- Collects results independently

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test --lib tests_check` | Parallel execution tests pass |
| 2 | `cargo bench --bench tests_ci -- tests_metrics_parsing` | Faster parsing (10-20%) |
| 3 | `cargo bench --bench tests_ci -- tests_ci/fast` | Fast mode benchmarks |
| 4 | `cargo bench --bench tests_ci -- ci_overhead` | Coverage overhead bounded |
| 5 | `cargo test --bench regression -- tests_ci` | Regression tests pass |
| 6 | `make check` | All checks pass |

## Completion Criteria

- [ ] Phase 1: Parallel suite execution implemented for CI mode
- [ ] Phase 2: Cargo output parsing optimized (10-20% faster)
- [ ] Phase 3: Fast mode early termination on failure
- [ ] Phase 4: Coverage collection overhead reduced
- [ ] Phase 5: CI mode regression tests added
- [ ] Phase 6: Baselines updated with new metrics
- [ ] All benchmarks run successfully
- [ ] `make check` passes
- [ ] Changes committed
- [ ] `./done` executed
