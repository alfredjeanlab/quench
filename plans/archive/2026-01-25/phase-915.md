# Phase 915: Test Runners - Framework

**Reference:** `docs/specs/11-test-runners.md`
**Depends on:** Phase 910 (behavioral specs)
**Type:** Implementation

## Overview

This phase implements the test runner framework infrastructure. It provides the core abstractions and configuration parsing needed to execute test suites. Concrete runner implementations (cargo, bats, pytest, etc.) will be added in subsequent phases.

The framework includes:
1. `TestRunner` trait for pluggable test executors
2. `TestSuiteConfig` for TOML configuration parsing
3. Runner registry for name-based lookup
4. Setup command execution before tests
5. CI-mode filtering for slow suites

## Project Structure

```
crates/cli/src/
├── checks/tests/
│   ├── mod.rs              # MODIFY: Add runners module
│   ├── runners/            # NEW: Test runner framework
│   │   ├── mod.rs          # Runner trait, registry, execution
│   │   ├── result.rs       # TestRunResult type
│   │   └── stub.rs         # Stub runner for unknown types
│   └── ...
├── config/
│   ├── mod.rs              # MODIFY: Add TestSuiteConfig to TestsConfig
│   └── duration.rs         # NEW: Duration string parsing
└── ...
```

## Dependencies

No new external dependencies. Uses existing:
- `serde` for config deserialization
- `std::process::Command` for setup commands

## Implementation Phases

### Phase 1: Duration String Parsing

**Goal**: Parse duration strings like `"30s"`, `"500ms"`, `"1m30s"`.

**File**: `crates/cli/src/config/duration.rs`

```rust
//! Duration string parsing for test runner time limits.

use std::time::Duration;
use serde::{Deserialize, Deserializer};

/// Parse a duration string into a Duration.
///
/// Supports formats:
/// - "30s" → 30 seconds
/// - "500ms" → 500 milliseconds
/// - "1m30s" → 90 seconds (future)
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();

    if let Some(ms) = s.strip_suffix("ms") {
        let n: u64 = ms.trim().parse()
            .map_err(|_| format!("invalid duration: {}", s))?;
        return Ok(Duration::from_millis(n));
    }

    if let Some(secs) = s.strip_suffix('s') {
        // Handle "1.5s" format
        let n: f64 = secs.trim().parse()
            .map_err(|_| format!("invalid duration: {}", s))?;
        return Ok(Duration::from_secs_f64(n));
    }

    if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins.trim().parse()
            .map_err(|_| format!("invalid duration: {}", s))?;
        return Ok(Duration::from_secs(n * 60));
    }

    Err(format!("invalid duration format: {} (use 30s, 500ms, or 1m)", s))
}

/// Deserialize an optional duration string.
pub fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_duration(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}
```

**Verification**:
```bash
cargo test --lib -- config::duration
```

---

### Phase 2: Suite Configuration Parsing

**Goal**: Parse `[[check.tests.suite]]` from quench.toml.

**File**: `crates/cli/src/config/mod.rs` (extend `TestsConfig`)

```rust
/// Tests check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Commit message validation settings.
    #[serde(default)]
    pub commit: TestsCommitConfig,

    /// Test suites to run.
    #[serde(default)]
    pub suite: Vec<TestSuiteConfig>,

    /// Time limit checking.
    #[serde(default)]
    pub time: TestsTimeConfig,
}

/// Configuration for a single test suite.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestSuiteConfig {
    /// Runner name: "cargo", "bats", "pytest", etc.
    pub runner: String,

    /// Name for custom runners (optional, defaults to runner).
    #[serde(default)]
    pub name: Option<String>,

    /// Test directory or file pattern.
    #[serde(default)]
    pub path: Option<String>,

    /// Command to run before tests.
    #[serde(default)]
    pub setup: Option<String>,

    /// Custom command for unsupported runners.
    #[serde(default)]
    pub command: Option<String>,

    /// Coverage targets (binary names or glob patterns).
    #[serde(default)]
    pub targets: Vec<String>,

    /// Only run in CI mode.
    #[serde(default)]
    pub ci: bool,

    /// Maximum total time for this suite.
    #[serde(default, deserialize_with = "duration::deserialize_duration")]
    pub max_total: Option<Duration>,

    /// Maximum average time per test.
    #[serde(default, deserialize_with = "duration::deserialize_duration")]
    pub max_avg: Option<Duration>,

    /// Maximum time for slowest individual test.
    #[serde(default, deserialize_with = "duration::deserialize_duration")]
    pub max_test: Option<Duration>,
}

/// Time limit configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsTimeConfig {
    /// Check level: "error" | "warn" | "off"
    #[serde(default = "TestsTimeConfig::default_check")]
    pub check: String,
}

impl Default for TestsTimeConfig {
    fn default() -> Self {
        Self { check: Self::default_check() }
    }
}

impl TestsTimeConfig {
    fn default_check() -> String {
        "warn".to_string()
    }
}
```

**Verification**:
```bash
cargo test --lib -- config::tests_config
# Also verify TOML parsing with:
cargo test --test specs -- config::validation --ignored
```

---

### Phase 3: Runner Trait Definition

**Goal**: Define the `TestRunner` trait for pluggable test execution.

**File**: `crates/cli/src/checks/tests/runners/mod.rs`

```rust
//! Test runner framework.
//!
//! Provides abstractions for executing test suites and collecting metrics.

mod result;
mod stub;

pub use result::{TestRunResult, TestResult};
pub use stub::StubRunner;

use std::path::Path;
use std::time::Duration;

use crate::config::TestSuiteConfig;

/// Context passed to test runners during execution.
pub struct RunnerContext<'a> {
    /// Project root directory.
    pub root: &'a Path,
    /// Whether running in CI mode.
    pub ci_mode: bool,
    /// Whether to collect coverage.
    pub collect_coverage: bool,
}

/// Trait for pluggable test runners.
///
/// Implementors execute tests and return timing/coverage metrics.
pub trait TestRunner: Send + Sync {
    /// Runner name (e.g., "cargo", "bats").
    fn name(&self) -> &'static str;

    /// Check if this runner can handle the given configuration.
    ///
    /// Returns false if required tools are not installed.
    fn available(&self, ctx: &RunnerContext) -> bool;

    /// Execute the test suite and return results.
    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult;
}
```

**File**: `crates/cli/src/checks/tests/runners/result.rs`

```rust
//! Test run result types.

use std::time::Duration;
use serde::Serialize;

/// Result of running a single test.
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    /// Test name.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Test duration.
    pub duration: Duration,
}

/// Result of running an entire test suite.
#[derive(Debug, Clone)]
pub struct TestRunResult {
    /// Whether all tests passed.
    pub passed: bool,
    /// Whether the suite was skipped (runner unavailable).
    pub skipped: bool,
    /// Error message if skipped.
    pub error: Option<String>,
    /// Total wall-clock time.
    pub total_time: Duration,
    /// Individual test results (if available).
    pub tests: Vec<TestResult>,
    /// Coverage percentage (0-100) by language.
    pub coverage: Option<std::collections::HashMap<String, f64>>,
}

impl TestRunResult {
    /// Create a successful result with no tests.
    pub fn passed(total_time: Duration) -> Self {
        Self {
            passed: true,
            skipped: false,
            error: None,
            total_time,
            tests: Vec::new(),
            coverage: None,
        }
    }

    /// Create a failed result.
    pub fn failed(total_time: Duration, tests: Vec<TestResult>) -> Self {
        Self {
            passed: false,
            skipped: false,
            error: None,
            total_time,
            tests,
            coverage: None,
        }
    }

    /// Create a skipped result (runner unavailable).
    pub fn skipped(error: impl Into<String>) -> Self {
        Self {
            passed: false,
            skipped: true,
            error: Some(error.into()),
            total_time: Duration::ZERO,
            tests: Vec::new(),
            coverage: None,
        }
    }

    /// Add test results.
    pub fn with_tests(mut self, tests: Vec<TestResult>) -> Self {
        self.tests = tests;
        self
    }

    /// Add coverage data.
    pub fn with_coverage(mut self, coverage: std::collections::HashMap<String, f64>) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Get test count.
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Get average test duration (if tests available).
    pub fn avg_duration(&self) -> Option<Duration> {
        if self.tests.is_empty() {
            return None;
        }
        let total: Duration = self.tests.iter().map(|t| t.duration).sum();
        Some(total / self.tests.len() as u32)
    }

    /// Get slowest test (if tests available).
    pub fn slowest_test(&self) -> Option<&TestResult> {
        self.tests.iter().max_by_key(|t| t.duration)
    }
}
```

**Verification**:
```bash
cargo build --lib
cargo test --lib -- checks::tests::runners
```

---

### Phase 4: Runner Registry

**Goal**: Register runners and select by name.

**File**: `crates/cli/src/checks/tests/runners/mod.rs` (extend)

```rust
use std::sync::Arc;

/// List of known runner names.
pub const RUNNER_NAMES: &[&str] = &[
    "cargo", "go", "pytest", "vitest", "bun", "jest", "bats", "custom",
];

/// Get all available runners.
pub fn all_runners() -> Vec<Arc<dyn TestRunner>> {
    vec![
        // Stub implementations for now - concrete runners in later phases
        Arc::new(StubRunner::new("cargo")),
        Arc::new(StubRunner::new("go")),
        Arc::new(StubRunner::new("pytest")),
        Arc::new(StubRunner::new("vitest")),
        Arc::new(StubRunner::new("bun")),
        Arc::new(StubRunner::new("jest")),
        Arc::new(StubRunner::new("bats")),
    ]
}

/// Get a runner by name.
pub fn get_runner(name: &str) -> Option<Arc<dyn TestRunner>> {
    all_runners().into_iter().find(|r| r.name() == name)
}

/// Filter suites based on CI mode.
///
/// In fast mode: skip suites with `ci = true`
/// In CI mode: run all suites
pub fn filter_suites_for_mode(
    suites: &[TestSuiteConfig],
    ci_mode: bool,
) -> Vec<&TestSuiteConfig> {
    suites
        .iter()
        .filter(|s| ci_mode || !s.ci)
        .collect()
}
```

**File**: `crates/cli/src/checks/tests/runners/stub.rs`

```rust
//! Stub runner for unimplemented runner types.

use std::time::Duration;

use super::{RunnerContext, TestRunResult, TestRunner};
use crate::config::TestSuiteConfig;

/// Stub runner that always skips.
pub struct StubRunner {
    name: &'static str,
}

impl StubRunner {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl TestRunner for StubRunner {
    fn name(&self) -> &'static str {
        self.name
    }

    fn available(&self, _ctx: &RunnerContext) -> bool {
        false
    }

    fn run(&self, _config: &TestSuiteConfig, _ctx: &RunnerContext) -> TestRunResult {
        TestRunResult::skipped(format!("{} runner not yet implemented", self.name))
    }
}
```

**Verification**:
```bash
cargo test --lib -- checks::tests::runners::registry
```

---

### Phase 5: Setup Command Execution

**Goal**: Execute setup commands before running tests.

**File**: `crates/cli/src/checks/tests/runners/mod.rs` (extend)

```rust
use std::process::{Command, Stdio};

/// Execute a setup command before running tests.
///
/// Returns Ok(()) on success, Err(message) on failure.
pub fn run_setup_command(
    setup: &str,
    root: &Path,
) -> Result<(), String> {
    // Use shell to handle complex commands
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", setup])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    } else {
        Command::new("sh")
            .args(["-c", setup])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    };

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!(
                "setup command failed: {}\n{}",
                setup,
                stderr.lines().take(5).collect::<Vec<_>>().join("\n")
            ))
        }
        Err(e) => Err(format!("failed to execute setup: {}", e)),
    }
}
```

**Verification**:
```bash
cargo test --lib -- checks::tests::runners::setup
```

---

### Phase 6: Integration with Tests Check

**Goal**: Wire runners into the existing tests check.

**File**: `crates/cli/src/checks/tests/mod.rs` (extend)

```rust
pub mod runners;

use runners::{RunnerContext, filter_suites_for_mode, get_runner, run_setup_command};

impl TestsCheck {
    /// Run configured test suites.
    fn run_suites(&self, ctx: &CheckContext) -> Option<SuiteResults> {
        let suites = &ctx.config.check.tests.suite;
        if suites.is_empty() {
            return None;
        }

        let runner_ctx = RunnerContext {
            root: ctx.root,
            ci_mode: ctx.ci_mode,
            collect_coverage: ctx.ci_mode, // Coverage only in CI
        };

        // Filter suites for current mode
        let active_suites = filter_suites_for_mode(suites, ctx.ci_mode);
        if active_suites.is_empty() {
            return None;
        }

        let mut results = Vec::new();
        let mut all_passed = true;

        for suite in active_suites {
            // Run setup command if configured
            if let Some(ref setup) = suite.setup {
                if let Err(e) = run_setup_command(setup, ctx.root) {
                    // Setup failure skips the suite
                    results.push(SuiteResult {
                        name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                        runner: suite.runner.clone(),
                        skipped: true,
                        error: Some(e),
                        ..Default::default()
                    });
                    continue;
                }
            }

            // Get runner for this suite
            let runner = match get_runner(&suite.runner) {
                Some(r) => r,
                None => {
                    results.push(SuiteResult {
                        name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                        runner: suite.runner.clone(),
                        skipped: true,
                        error: Some(format!("unknown runner: {}", suite.runner)),
                        ..Default::default()
                    });
                    continue;
                }
            };

            // Check runner availability
            if !runner.available(&runner_ctx) {
                results.push(SuiteResult {
                    name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                    runner: suite.runner.clone(),
                    skipped: true,
                    error: Some(format!("{} not available", suite.runner)),
                    ..Default::default()
                });
                continue;
            }

            // Execute the runner
            let run_result = runner.run(suite, &runner_ctx);
            all_passed = all_passed && run_result.passed;

            results.push(SuiteResult {
                name: suite.name.clone().unwrap_or_else(|| suite.runner.clone()),
                runner: suite.runner.clone(),
                passed: run_result.passed,
                skipped: run_result.skipped,
                error: run_result.error,
                test_count: run_result.test_count(),
                total_ms: run_result.total_time.as_millis() as u64,
                avg_ms: run_result.avg_duration().map(|d| d.as_millis() as u64),
                max_ms: run_result.slowest_test().map(|t| t.duration.as_millis() as u64),
                max_test: run_result.slowest_test().map(|t| t.name.clone()),
            });
        }

        Some(SuiteResults {
            passed: all_passed,
            suites: results,
        })
    }
}

/// Aggregated results from all test suites.
#[derive(Debug, Default)]
struct SuiteResults {
    passed: bool,
    suites: Vec<SuiteResult>,
}

/// Result from a single test suite.
#[derive(Debug, Default)]
struct SuiteResult {
    name: String,
    runner: String,
    passed: bool,
    skipped: bool,
    error: Option<String>,
    test_count: usize,
    total_ms: u64,
    avg_ms: Option<u64>,
    max_ms: Option<u64>,
    max_test: Option<String>,
}
```

**Verification**:
```bash
cargo build --lib
cargo test --lib -- checks::tests
```

---

## Key Implementation Details

### Object-Safe Trait Design

The `TestRunner` trait is object-safe (`Send + Sync`) to allow:
- Dynamic dispatch via `Arc<dyn TestRunner>`
- Parallel execution across runners (future)
- Easy extension with new runners

### CI-Mode Filtering Pattern

Following the existing pattern from `CheckContext::ci_mode`:

```rust
// In suite config:
ci = true  // Only run in CI mode

// In filtering:
if !ctx.ci_mode && suite.ci {
    continue; // Skip slow suite in fast mode
}
```

### Duration Parsing

Follows the spec format:
- `"30s"` → 30 seconds
- `"500ms"` → 500 milliseconds
- `"1m"` → 1 minute

### Error Handling

Setup and runner errors produce `skipped` results (not hard failures):
- Allows remaining suites to run
- Reports which suites were skipped and why
- Matches existing `CheckResult::skipped()` pattern

### Metrics Structure

Suite metrics aggregate into the tests check metrics:

```json
{
  "name": "tests",
  "metrics": {
    "suites": [
      {
        "name": "cargo",
        "runner": "cargo",
        "passed": true,
        "test_count": 42,
        "total_ms": 1234,
        "avg_ms": 29,
        "max_ms": 156,
        "max_test": "integration::large_file"
      }
    ],
    "total_tests": 42,
    "total_ms": 1234
  }
}
```

---

## Verification Plan

### Per-Phase Verification

| Phase | Command | Expected |
|-------|---------|----------|
| 1 | `cargo test --lib -- config::duration` | Duration parsing works |
| 2 | `cargo build --lib` | Config compiles |
| 3 | `cargo build --lib` | Trait compiles |
| 4 | `cargo test --lib -- runners::registry` | Registry works |
| 5 | `cargo test --lib -- runners::setup` | Setup works |
| 6 | `cargo build --lib` | Integration compiles |

### Final Verification

```bash
# Unit tests pass
cargo test --lib

# Config parsing works for suites
cargo test --lib -- config::tests_suite

# Full check
make check
```

### Behavioral Spec Status

After this phase, the following Phase 910 specs should remain ignored (awaiting concrete runner implementations):

```bash
cargo test --test specs -- tests::runners --ignored
# cargo_runner_executes_cargo_test: still ignored (needs cargo runner)
# bats_runner_executes_bats_with_timing: still ignored (needs bats runner)
```

---

## Completion Criteria

- [ ] Phase 1: Duration parsing module with tests
- [ ] Phase 2: `TestSuiteConfig` added to `TestsConfig`
- [ ] Phase 3: `TestRunner` trait and result types defined
- [ ] Phase 4: Runner registry with stub implementations
- [ ] Phase 5: Setup command execution with error handling
- [ ] Phase 6: Integration with tests check (calling runners)
- [ ] All unit tests pass (`cargo test --lib`)
- [ ] `make check` passes
- [ ] `./done` executed successfully

---

## Future Phases

This framework enables subsequent implementation phases:

- **Phase 916**: Cargo runner implementation
- **Phase 917**: Bats runner implementation
- **Phase 918**: Coverage collection (llvm-cov, kcov)
- **Phase 919**: Time limit violations and thresholds
- **Phase 920**: Other runners (pytest, vitest, go, jest, bun)
