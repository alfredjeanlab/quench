# Phase 945: Tests Check - CI Mode Metrics

## Overview

Enhance the tests check CI mode output to provide comprehensive test execution metrics. Currently, suite-level metrics are collected but not aggregated into top-level summaries. This phase adds:
- **Total time aggregation** across all suites
- **Average time calculation** weighted by test count
- **Max test time tracking** with test name (across all suites)
- **Coverage aggregation** by language
- **Per-package coverage breakdown** for granular visibility

These metrics enable CI dashboards to display test health at a glance and support ratcheting on key metrics like coverage and slowest tests.

## Project Structure

```
crates/cli/src/checks/tests/
├── mod.rs                    # Update: top-level metric aggregation
├── mod_tests.rs              # Update: test metric aggregation
└── runners/
    ├── mod.rs                # Update: AggregatedCoverage per-package support
    ├── mod_tests.rs          # Update: test coverage aggregation
    ├── coverage.rs           # Update: per-package coverage extraction
    └── coverage_tests.rs     # Update: test per-package parsing

crates/cli/src/config/
└── test_config.rs            # Reference: coverage package config structure

tests/specs/checks/tests/
└── ci_metrics.rs             # NEW: behavioral tests for CI metrics

tests/fixtures/
├── test-metrics-multi-suite/ # NEW: multiple suites for aggregation testing
└── coverage-per-package/     # NEW: workspace with multiple packages
```

## Dependencies

No new external dependencies. Uses existing:
- `serde_json` for metrics serialization
- `cargo-llvm-cov` for Rust coverage (optional)
- `kcov` for shell coverage (optional)

## Implementation Phases

### Phase 1: Top-Level Time Metric Aggregation

Add aggregated timing metrics to the test suite results. Currently each suite reports its own `total_ms`, `avg_ms`, and `max_ms`. Add top-level aggregates.

**Current metrics structure:**
```json
{
  "test_count": 47,
  "total_ms": 2341,
  "suites": [...]
}
```

**Target metrics structure:**
```json
{
  "test_count": 276,
  "total_ms": 12400,
  "avg_ms": 45,
  "max_ms": 2100,
  "max_test": "tests::integration::large_file_parse",
  "suites": [...]
}
```

```rust
// crates/cli/src/checks/tests/mod.rs

impl SuiteResults {
    /// Calculate aggregated timing metrics across all suites.
    fn aggregated_metrics(&self) -> AggregatedMetrics {
        let total_tests: usize = self.suites.iter()
            .map(|s| s.test_count)
            .sum();

        let total_ms: u64 = self.suites.iter()
            .map(|s| s.total_ms)
            .sum();

        // Weighted average: sum of (suite_avg * suite_count) / total_count
        let avg_ms = if total_tests > 0 {
            let weighted_sum: u64 = self.suites.iter()
                .filter_map(|s| s.avg_ms.map(|avg| avg * s.test_count as u64))
                .sum();
            Some(weighted_sum / total_tests as u64)
        } else {
            None
        };

        // Find slowest test across all suites
        let (max_ms, max_test) = self.suites.iter()
            .filter_map(|s| s.max_ms.map(|ms| (ms, s.max_test.clone())))
            .max_by_key(|(ms, _)| *ms)
            .map(|(ms, name)| (Some(ms), name))
            .unwrap_or((None, None));

        AggregatedMetrics {
            test_count: total_tests,
            total_ms,
            avg_ms,
            max_ms,
            max_test,
        }
    }
}

#[derive(Debug)]
struct AggregatedMetrics {
    test_count: usize,
    total_ms: u64,
    avg_ms: Option<u64>,
    max_ms: Option<u64>,
    max_test: Option<String>,
}
```

**Files:**
- `crates/cli/src/checks/tests/mod.rs`
- `crates/cli/src/checks/tests/mod_tests.rs`

**Verification:** Unit tests for metric aggregation with multiple suites.

### Phase 2: Update Metrics JSON Output

Integrate aggregated metrics into the check result JSON output.

```rust
// crates/cli/src/checks/tests/mod.rs

fn run_test_suites(&self, ctx: &CheckContext) -> CheckResult {
    let suite_results = match self.run_suites(ctx) {
        Some(r) => r,
        None => return CheckResult::passed(self.name()),
    };

    // Calculate aggregated metrics
    let agg = suite_results.aggregated_metrics();

    // ... existing coverage aggregation ...

    // Build metrics JSON with top-level aggregates
    let mut metrics = json!({
        "test_count": agg.test_count,
        "total_ms": agg.total_ms,
        "suites": suite_results.suites.iter().map(|s| {
            // ... existing per-suite serialization ...
        }).collect::<Vec<_>>(),
    });

    // Add optional aggregated metrics
    if let Some(avg) = agg.avg_ms {
        metrics["avg_ms"] = json!(avg);
    }
    if let Some(max) = agg.max_ms {
        metrics["max_ms"] = json!(max);
    }
    if let Some(ref test) = agg.max_test {
        metrics["max_test"] = json!(test);
    }

    // ... rest of result building ...
}
```

**Verification:** Behavioral test verifying JSON output contains all aggregated fields.

### Phase 3: Per-Package Coverage Extraction

Extract per-package coverage from llvm-cov output. The llvm-cov JSON output includes file paths which can be grouped by package.

**llvm-cov JSON structure (relevant fields):**
```json
{
  "data": [{
    "totals": { "lines": { "percent": 82.3 } },
    "files": [
      { "filename": "/path/to/crates/core/src/lib.rs", "summary": { "lines": { "percent": 90.1 } } },
      { "filename": "/path/to/crates/cli/src/main.rs", "summary": { "lines": { "percent": 75.0 } } }
    ]
  }]
}
```

```rust
// crates/cli/src/checks/tests/runners/coverage.rs

/// Coverage result with per-package breakdown.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    pub success: bool,
    pub error: Option<String>,
    pub duration: Duration,
    pub line_coverage: Option<f64>,
    pub files: HashMap<String, f64>,
    /// Per-package coverage (package name -> line coverage %)
    pub packages: HashMap<String, f64>,
}

fn parse_llvm_cov_json(json: &str, duration: Duration) -> CoverageResult {
    // ... existing parsing ...

    // Group files by package
    let mut package_files: HashMap<String, Vec<f64>> = HashMap::new();
    for file in &data.files {
        let package = extract_package_name(&file.filename);
        package_files
            .entry(package)
            .or_default()
            .push(file.summary.lines.percent);
    }

    // Calculate per-package averages
    let packages: HashMap<String, f64> = package_files
        .into_iter()
        .map(|(pkg, coverages)| {
            let avg = coverages.iter().sum::<f64>() / coverages.len() as f64;
            (pkg, avg)
        })
        .collect();

    CoverageResult {
        success: true,
        error: None,
        duration,
        line_coverage: Some(line_coverage),
        files,
        packages,
    }
}

/// Extract package name from file path.
///
/// Heuristics:
/// - Cargo workspace: look for "crates/<name>/" pattern
/// - Single package: use "root" or package name from Cargo.toml
fn extract_package_name(path: &str) -> String {
    // Check for "crates/<name>/" pattern (workspace)
    if let Some(idx) = path.find("/crates/") {
        let rest = &path[idx + 8..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
    }

    // Check for "packages/<name>/" pattern (monorepo)
    if let Some(idx) = path.find("/packages/") {
        let rest = &path[idx + 10..];
        if let Some(end) = rest.find('/') {
            return rest[..end].to_string();
        }
    }

    // Fallback to "root"
    "root".to_string()
}
```

**Files:**
- `crates/cli/src/checks/tests/runners/coverage.rs`
- `crates/cli/src/checks/tests/runners/coverage_tests.rs`

**Verification:** Unit tests for package name extraction and grouping.

### Phase 4: Per-Package Coverage in Metrics Output

Add per-package coverage breakdown to the metrics JSON and support config-based thresholds.

**Config reference (existing in spec):**
```toml
[check.tests.coverage]
min = 75

[check.tests.coverage.package.core]
min = 90

[check.tests.coverage.package.cli]
min = 70
```

**Target metrics structure:**
```json
{
  "coverage": {
    "rust": 82.3,
    "shell": 71.2
  },
  "coverage_by_package": {
    "core": 90.1,
    "cli": 75.0,
    "utils": 68.5
  }
}
```

```rust
// crates/cli/src/checks/tests/mod.rs

fn run_test_suites(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing code ...

    // Aggregate per-package coverage
    let mut packages_coverage: HashMap<String, f64> = HashMap::new();
    for suite in &suite_results.suites {
        if let Some(ref cov) = suite.coverage_result {
            for (pkg, pct) in &cov.packages {
                packages_coverage
                    .entry(pkg.clone())
                    .and_modify(|existing| *existing = existing.max(*pct))
                    .or_insert(*pct);
            }
        }
    }

    // Add to metrics
    if !packages_coverage.is_empty() {
        metrics["coverage_by_package"] = json!(packages_coverage);
    }

    // ... rest of result building ...
}
```

**Verification:** Behavioral test verifying per-package coverage in output.

### Phase 5: Coverage Threshold Violations

Generate violations when per-package coverage falls below thresholds.

```rust
// crates/cli/src/checks/tests/mod.rs

fn check_coverage_thresholds(
    &self,
    ctx: &CheckContext,
    aggregated_coverage: &HashMap<String, f64>,
    packages_coverage: &HashMap<String, f64>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let coverage_config = &ctx.config.check.tests.coverage;

    // Check global minimum
    if let Some(min) = coverage_config.min {
        for (lang, pct) in aggregated_coverage {
            if *pct < min as f64 {
                violations.push(Violation::threshold(
                    format!("<coverage:{}>", lang),
                    "coverage_below_threshold",
                    format!("{}% coverage below minimum {}%", pct.round(), min),
                    *pct as i64,
                    min as i64,
                ));
            }
        }
    }

    // Check per-package minimums
    for (pkg, pct) in packages_coverage {
        if let Some(pkg_config) = coverage_config.package.get(pkg) {
            if let Some(pkg_min) = pkg_config.min {
                if *pct < pkg_min as f64 {
                    violations.push(Violation::threshold(
                        format!("<coverage:package:{}>", pkg),
                        "package_coverage_below_threshold",
                        format!("Package '{}' has {}% coverage, minimum is {}%", pkg, pct.round(), pkg_min),
                        *pct as i64,
                        pkg_min as i64,
                    ));
                }
            }
        }
    }

    violations
}
```

**Verification:** Behavioral test with per-package thresholds.

### Phase 6: Behavioral Tests and Fixtures

Create comprehensive behavioral tests for CI mode metrics.

**Fixture: test-metrics-multi-suite/**
```
test-metrics-multi-suite/
├── Cargo.toml
├── quench.toml
├── src/
│   └── lib.rs
├── tests/
│   ├── unit_tests.rs
│   └── cli/
│       └── basic.bats
```

```toml
# quench.toml
[[check.tests.suite]]
runner = "cargo"

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
```

**Test: ci_metrics.rs**
```rust
// tests/specs/checks/tests/ci_metrics.rs

use crate::prelude::*;

#[test]
fn ci_mode_reports_aggregated_timing_metrics() {
    cli()
        .on("test-metrics-multi-suite")
        .args(["--ci", "-o", "json"])
        .stdout_json(json!({
            "checks": [{
                "name": "tests",
                "metrics": {
                    "test_count": 10,
                    "total_ms": any::<u64>(),
                    "avg_ms": any::<u64>(),
                    "max_ms": any::<u64>(),
                    "max_test": any::<String>(),
                }
            }]
        }));
}

#[test]
fn ci_mode_reports_per_package_coverage() {
    cli()
        .on("coverage-per-package")
        .args(["--ci", "-o", "json"])
        .stdout_json(json!({
            "checks": [{
                "name": "tests",
                "metrics": {
                    "coverage": {
                        "rust": any::<f64>()
                    },
                    "coverage_by_package": {
                        "core": any::<f64>(),
                        "cli": any::<f64>()
                    }
                }
            }]
        }));
}

#[test]
fn ci_mode_fails_on_package_coverage_threshold() {
    cli()
        .on("coverage-threshold-fail")
        .args(["--ci"])
        .exits(1)
        .stdout_has("package_coverage_below_threshold");
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`
- `tests/fixtures/test-metrics-multi-suite/`
- `tests/fixtures/coverage-per-package/`
- `tests/fixtures/coverage-threshold-fail/`

**Verification:** `cargo test --test specs`

## Key Implementation Details

### Metric Aggregation Flow

```
Suite 1: cargo (47 tests, 2341ms, avg 50ms, max 245ms "test_a")
Suite 2: bats (12 tests, 1200ms, avg 100ms, max 350ms "test_b")
                    ↓
              Aggregation
                    ↓
Totals: 59 tests, 3541ms, avg 60ms, max 350ms "test_b"
```

### Weighted Average Calculation

```rust
// Weighted average = sum(suite_avg * suite_count) / total_count
//
// Example:
// Suite 1: 47 tests, avg 50ms → 47 * 50 = 2350
// Suite 2: 12 tests, avg 100ms → 12 * 100 = 1200
// Total: 59 tests
// Weighted avg: (2350 + 1200) / 59 = 60ms
```

### Package Name Extraction Patterns

| Path Pattern | Package Name |
|--------------|--------------|
| `/project/crates/core/src/lib.rs` | `core` |
| `/project/crates/cli/src/main.rs` | `cli` |
| `/project/packages/utils/index.ts` | `utils` |
| `/project/src/lib.rs` | `root` |

### Coverage Threshold Precedence

1. Per-package threshold (`[check.tests.coverage.package.<name>]`)
2. Global threshold (`[check.tests.coverage].min`)
3. No threshold (pass regardless of coverage)

### Metrics JSON Schema Update

```json
{
  "metrics": {
    "test_count": 276,
    "total_ms": 12400,
    "avg_ms": 45,
    "max_ms": 2100,
    "max_test": "tests::integration::large_file_parse",
    "suites": [
      {
        "name": "cargo",
        "runner": "cargo",
        "passed": true,
        "test_count": 264,
        "total_ms": 10500,
        "avg_ms": 40,
        "max_ms": 2100,
        "max_test": "tests::integration::large_file_parse"
      },
      {
        "name": "bats",
        "runner": "bats",
        "passed": true,
        "test_count": 12,
        "total_ms": 1900,
        "avg_ms": 158,
        "max_ms": 450,
        "max_test": "test_complex_scenario"
      }
    ],
    "coverage": {
      "rust": 82.3,
      "shell": 71.2
    },
    "coverage_by_package": {
      "core": 90.1,
      "cli": 75.0,
      "utils": 68.5
    }
  }
}
```

## Verification Plan

### Unit Tests

1. **Metric Aggregation** (`mod_tests.rs`)
   - Multiple suites with different test counts
   - Weighted average calculation accuracy
   - Max test selection across suites
   - Empty suites handled gracefully

2. **Package Extraction** (`coverage_tests.rs`)
   - Cargo workspace paths
   - Monorepo paths
   - Single-package fallback

3. **Coverage Merging** (`mod_tests.rs`)
   - Per-package coverage aggregation
   - Same package from multiple suites

### Behavioral Tests

1. `ci_mode_reports_aggregated_timing_metrics` - Top-level timing
2. `ci_mode_reports_per_suite_timing` - Per-suite timing breakdown
3. `ci_mode_reports_max_test_name` - Slowest test identification
4. `ci_mode_reports_per_package_coverage` - Package coverage breakdown
5. `ci_mode_fails_on_package_coverage_threshold` - Threshold violation
6. `ci_mode_merges_coverage_from_multiple_suites` - Coverage aggregation

### Manual Verification

```bash
# Multi-suite metrics
quench check --ci -o json | jq '.checks[] | select(.name == "tests") | .metrics'

# Expected output includes:
# - test_count (total)
# - total_ms (aggregated)
# - avg_ms (weighted average)
# - max_ms and max_test (slowest across all suites)
# - coverage_by_package (per-package breakdown)
```

## Commit Strategy

1. `feat(tests): add top-level timing metric aggregation`
2. `feat(tests): extract per-package coverage from llvm-cov`
3. `feat(tests): add coverage threshold violations`
4. `test(tests): add CI metrics behavioral tests`
