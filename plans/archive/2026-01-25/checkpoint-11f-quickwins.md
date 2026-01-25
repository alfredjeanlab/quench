# Checkpoint 11F: Quick Wins - Tests CI Mode

## Overview

Small, targeted improvements to the tests CI mode functionality that provide value with minimal implementation effort. Follows checkpoint 11E (performance optimizations) and focuses on developer experience, better metrics, and more informative output.

**Follows:** checkpoint-11e-perf (Performance Optimizations)

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── mod.rs                          # MODIFY: Add skipped count, duration percentiles
│   └── runners/
│       ├── mod.rs                      # MODIFY: Add timeout support
│       ├── result.rs                   # MODIFY: Add skipped count, percentile metrics
│       └── cargo.rs                    # MODIFY: Parse ignored/skipped tests
├── crates/cli/src/cli.rs               # MODIFY: Add --test-timeout flag
├── crates/cli/src/config.rs            # MODIFY: Add timeout to suite config
├── crates/cli/src/check.rs             # MODIFY: Add verbose progress callbacks
├── tests/specs/checks/tests/
│   └── ci_metrics.rs                   # MODIFY: Add specs for new metrics
└── tests/fixtures/tests-ci/
    └── quench.toml                     # MODIFY: Add timeout example
```

## Dependencies

No new dependencies. Uses existing:
- `std::time::Duration` for timeout handling
- `std::process::Command` with timeout via `wait_timeout` (already available)

## Implementation Phases

### Phase 1: Track Skipped/Ignored Test Counts

**Goal:** Add `skipped_count` and `ignored_count` to test metrics for better visibility into test suite health.

Currently, only `test_count` is tracked. Adding skipped/ignored counts helps identify test debt.

**File:** `crates/cli/src/checks/tests/runners/result.rs`

```rust
/// Result from running a test suite.
#[derive(Debug, Default)]
pub struct TestRunResult {
    pub passed: bool,
    pub skipped: bool,
    pub error: Option<String>,
    pub total_time: Duration,
    pub tests: Vec<TestResult>,
    pub coverage: Option<HashMap<String, f64>>,
    pub coverage_by_package: Option<HashMap<String, f64>>,
}

impl TestRunResult {
    /// Total number of tests.
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Number of tests that passed.
    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.passed).count()
    }

    /// Number of tests that failed.
    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|t| !t.passed && !t.skipped).count()
    }

    /// Number of tests that were skipped/ignored.
    pub fn skipped_count(&self) -> usize {
        self.tests.iter().filter(|t| t.skipped).count()
    }
}

/// Individual test result.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub skipped: bool,  // NEW: Track skipped status
    pub duration: Duration,
}

impl TestResult {
    pub fn passed(name: impl Into<String>, duration: Duration) -> Self {
        Self { name: name.into(), passed: true, skipped: false, duration }
    }

    pub fn failed(name: impl Into<String>, duration: Duration) -> Self {
        Self { name: name.into(), passed: false, skipped: false, duration }
    }

    pub fn skipped(name: impl Into<String>) -> Self {
        Self { name: name.into(), passed: true, skipped: true, duration: Duration::ZERO }
    }
}
```

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

Update `parse_cargo_output` to detect ignored tests:

```rust
// Parse "test <name> ... ignored" lines
if result == "ignored" {
    tests.push(TestResult::skipped(name));
    continue;
}
```

**File:** `crates/cli/src/checks/tests/mod.rs`

Add to metrics JSON:

```rust
let mut obj = json!({
    "name": s.name,
    "runner": s.runner,
    "passed": s.passed,
    "test_count": s.test_count,
    "skipped_count": s.skipped_count,  // NEW
});
```

**Verification:**
```bash
cargo test --lib test_result
cargo test --test specs ci_metrics
```

### Phase 2: Add Duration Percentiles to Metrics

**Goal:** Add p50, p90, p99 duration percentiles to help identify slow test distributions.

**File:** `crates/cli/src/checks/tests/runners/result.rs`

```rust
impl TestRunResult {
    /// Calculate duration percentile (p50, p90, p99).
    pub fn percentile_duration(&self, p: f64) -> Option<Duration> {
        if self.tests.is_empty() {
            return None;
        }
        let mut durations: Vec<Duration> = self.tests
            .iter()
            .filter(|t| !t.skipped)
            .map(|t| t.duration)
            .collect();
        if durations.is_empty() {
            return None;
        }
        durations.sort();
        let idx = ((durations.len() as f64 * p / 100.0).ceil() as usize)
            .saturating_sub(1)
            .min(durations.len() - 1);
        Some(durations[idx])
    }
}
```

**File:** `crates/cli/src/checks/tests/mod.rs`

Add percentiles to suite metrics in CI mode:

```rust
if ctx.ci_mode {
    if let Some(p50) = result.percentile_duration(50.0) {
        obj["p50_ms"] = json!(p50.as_millis());
    }
    if let Some(p90) = result.percentile_duration(90.0) {
        obj["p90_ms"] = json!(p90.as_millis());
    }
    if let Some(p99) = result.percentile_duration(99.0) {
        obj["p99_ms"] = json!(p99.as_millis());
    }
}
```

**Verification:**
```bash
cargo test --lib percentile
cargo test --test specs ci_metrics
```

### Phase 3: Suite Timeout Configuration

**Goal:** Add configurable timeout per test suite to prevent hanging tests.

**File:** `crates/cli/src/config.rs`

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TestSuiteConfig {
    pub runner: String,
    pub name: Option<String>,
    pub path: Option<String>,
    pub setup: Option<String>,
    pub ci: bool,
    pub max_total: Option<Duration>,
    pub max_avg: Option<Duration>,
    pub max_test: Option<Duration>,
    pub timeout: Option<Duration>,  // NEW: Suite execution timeout
    // ...
}
```

**File:** `crates/cli/src/checks/tests/runners/mod.rs`

Add timeout handling to runner execution:

```rust
use std::time::Duration;
use wait_timeout::ChildExt;

/// Run a command with optional timeout.
pub fn run_with_timeout(
    mut cmd: Command,
    timeout: Option<Duration>,
) -> std::io::Result<std::process::Output> {
    let mut child = cmd.spawn()?;

    match timeout {
        Some(t) => {
            match child.wait_timeout(t)? {
                Some(status) => {
                    let stdout = child.stdout.take().map(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    let stderr = child.stderr.take().map(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok();
                        buf
                    }).unwrap_or_default();
                    Ok(std::process::Output { status, stdout, stderr })
                }
                None => {
                    // Timeout - kill the process
                    child.kill()?;
                    Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("command timed out after {:?}", t),
                    ))
                }
            }
        }
        None => child.wait_with_output(),
    }
}
```

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

Use timeout in cargo runner:

```rust
impl TestRunner for CargoRunner {
    fn run(&self, suite: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        let mut cmd = Command::new("cargo");
        cmd.args(["test", "--", "--test-threads=1"]);
        // ... existing setup ...

        let start = Instant::now();
        let output = match run_with_timeout(cmd, suite.timeout) {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("timed out after {:?}", suite.timeout.unwrap()),
                );
            }
            Err(e) => {
                return TestRunResult::failed(start.elapsed(), e.to_string());
            }
        };
        // ... rest of parsing ...
    }
}
```

**Verification:**
```bash
cargo test --lib timeout
```

### Phase 4: Improve Error Messages for Common Failures

**Goal:** Provide more actionable error messages for common test failures.

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

Add better error detection:

```rust
/// Categorize cargo test error for better messaging.
fn categorize_cargo_error(stderr: &str, exit_code: Option<i32>) -> String {
    // Compilation error
    if stderr.contains("error[E") || stderr.contains("could not compile") {
        return "compilation failed - fix build errors first".to_string();
    }

    // Missing test binary
    if stderr.contains("no test target") || stderr.contains("can't find") {
        return "no tests found - check test file paths".to_string();
    }

    // Timeout (from signal)
    if exit_code == Some(137) || exit_code == Some(124) {
        return "test timed out - check for infinite loops or deadlocks".to_string();
    }

    // Out of memory
    if stderr.contains("out of memory") || exit_code == Some(139) {
        return "out of memory - reduce test parallelism or resource usage".to_string();
    }

    // Generic failure
    "tests failed".to_string()
}
```

**File:** `crates/cli/src/checks/tests/mod.rs`

Use categorized errors in violation messages:

```rust
let advice = categorize_cargo_error(&run_result.stderr, run_result.exit_code);
Violation::file_only(format!("<suite:{}>", s.name), "test_suite_failed", advice)
```

**Verification:**
```bash
cargo test --lib error_messages
cargo test --test specs error_scenarios
```

### Phase 5: Add Verbose Mode Suite Progress

**Goal:** In verbose mode, show which suite is currently running and its progress.

**File:** `crates/cli/src/checks/tests/mod.rs`

Add progress reporting:

```rust
impl TestsCheck {
    fn run_single_suite(
        suite: &TestSuiteConfig,
        runner_ctx: &RunnerContext,
        verbose: bool,
    ) -> SuiteResult {
        let suite_name = suite.name.as_deref().unwrap_or(&suite.runner);

        if verbose {
            eprintln!("  Running suite: {} ({})", suite_name, suite.runner);
        }

        // ... existing execution ...

        if verbose {
            let status = if result.passed { "PASS" } else { "FAIL" };
            eprintln!(
                "  {} {} ({} tests, {:?})",
                status,
                suite_name,
                result.test_count,
                Duration::from_millis(result.total_ms),
            );
        }

        result
    }
}
```

**Note:** This requires threading verbose flag through the check context, which is already available via `ctx.verbose`.

**Verification:**
```bash
cargo test --lib verbose
quench check --tests --ci --verbose  # Manual verification
```

### Phase 6: Add Specs and Update Documentation

**Goal:** Add behavioral specs for new metrics and update fixture configuration.

**File:** `tests/specs/checks/tests/ci_metrics.rs`

Add specs for new metrics:

```rust
/// Spec: docs/specs/checks/tests.md#skipped-metrics
#[test]
fn ci_mode_reports_skipped_count() {
    cli()
        .args(["check", "--tests", "--ci", "-o", "json"])
        .on("tests-ci")
        .succeeds()
        .stdout_has("skipped_count");
}

/// Spec: docs/specs/checks/tests.md#percentile-metrics
#[test]
fn ci_mode_reports_percentiles() {
    cli()
        .args(["check", "--tests", "--ci", "-o", "json"])
        .on("tests-ci")
        .succeeds()
        .stdout_has("p50_ms")
        .stdout_has("p90_ms");
}

/// Spec: docs/specs/checks/tests.md#timeout
#[test]
fn suite_timeout_kills_slow_tests() {
    cli()
        .args(["check", "--tests", "--ci"])
        .on("tests-timeout")
        .fails()
        .stdout_has("timed out");
}
```

**File:** `tests/fixtures/tests-ci/quench.toml`

Add timeout example to fixture:

```toml
[[check.tests.suite]]
runner = "cargo"
timeout = "60s"  # Prevent runaway tests
```

**Verification:**
```bash
cargo test --test specs ci_metrics
make check
```

## Key Implementation Details

### Metrics Addition Summary

| Metric | Type | Description |
|--------|------|-------------|
| `skipped_count` | integer | Number of ignored/skipped tests |
| `p50_ms` | integer | 50th percentile test duration |
| `p90_ms` | integer | 90th percentile test duration |
| `p99_ms` | integer | 99th percentile test duration |

### JSON Output Changes

Before:
```json
{
  "suites": [{
    "name": "cargo",
    "test_count": 100,
    "total_ms": 5000,
    "avg_ms": 50,
    "max_ms": 200
  }]
}
```

After:
```json
{
  "suites": [{
    "name": "cargo",
    "test_count": 100,
    "skipped_count": 3,
    "total_ms": 5000,
    "avg_ms": 50,
    "max_ms": 200,
    "p50_ms": 45,
    "p90_ms": 120,
    "p99_ms": 180
  }]
}
```

### Backward Compatibility

All changes are additive:
- New JSON fields don't break existing consumers
- New config options have sensible defaults (no timeout = unlimited)
- Verbose output is opt-in

### Error Message Categories

| Category | Detection | Message |
|----------|-----------|---------|
| Compilation | `error[E` in stderr | "compilation failed - fix build errors first" |
| No tests | "no test target" | "no tests found - check test file paths" |
| Timeout | exit code 137/124 | "test timed out - check for infinite loops" |
| OOM | exit code 139 | "out of memory - reduce test parallelism" |
| Generic | other | "tests failed" |

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test --lib test_result` | Skipped count tests pass |
| 2 | `cargo test --lib percentile` | Percentile calculation tests pass |
| 3 | `cargo test --lib timeout` | Timeout handling tests pass |
| 4 | `cargo test --lib error_messages` | Error categorization tests pass |
| 5 | `quench check --tests --ci -v` | Progress output visible |
| 6 | `cargo test --test specs ci_metrics` | All CI metrics specs pass |
| All | `make check` | Full CI passes |

## Completion Criteria

- [ ] Phase 1: Skipped/ignored test count in metrics
- [ ] Phase 2: Duration percentiles (p50, p90, p99) in metrics
- [ ] Phase 3: Suite timeout configuration
- [ ] Phase 4: Better error messages for common failures
- [ ] Phase 5: Verbose mode suite progress
- [ ] Phase 6: Specs and documentation updated
- [ ] All tests pass
- [ ] `make check` passes
- [ ] Changes committed
- [ ] `./done` executed
