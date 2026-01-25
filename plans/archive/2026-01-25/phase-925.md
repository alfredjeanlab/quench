# Phase 925: Test Runners - Cargo Coverage

Implement Rust code coverage collection via `cargo llvm-cov` for the Cargo test runner.

## Overview

Extend `CargoRunner` to collect Rust code coverage when `ctx.collect_coverage` is true. Uses `cargo llvm-cov` with JSON output for reliable parsing of line coverage percentages and per-file coverage data.

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── cargo.rs            # Update: add coverage collection via llvm-cov
├── cargo_tests.rs      # Update: add coverage parsing tests
├── coverage.rs         # NEW: llvm-cov report parsing
├── coverage_tests.rs   # NEW: unit tests for coverage parsing
└── mod.rs              # Update: export coverage module
```

Test fixtures:
```
tests/fixtures/
└── rust-coverage/           # NEW: fixture with partial coverage
    ├── Cargo.toml
    ├── src/lib.rs           # Functions with varying coverage
    └── tests/basic.rs

tests/specs/checks/tests/
└── coverage.rs              # Update: enable Phase 925 specs
```

## Dependencies

No new crate dependencies. External tool requirement:

- `cargo-llvm-cov` - Must be installed (`cargo install cargo-llvm-cov`)
- Graceful degradation: skip coverage if not installed

## Implementation Phases

### Phase 1: Coverage Module Skeleton

Create the coverage module structure for llvm-cov JSON parsing.

**File:** `crates/cli/src/checks/tests/runners/coverage.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Coverage report parsing for cargo llvm-cov.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Deserialize;

/// Result of collecting coverage.
#[derive(Debug, Clone)]
pub struct CoverageResult {
    /// Whether coverage collection succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Collection time.
    pub duration: Duration,
    /// Overall line coverage percentage (0-100).
    pub line_coverage: Option<f64>,
    /// Per-file coverage data (path -> line coverage %).
    pub files: HashMap<String, f64>,
}

impl CoverageResult {
    pub fn failed(duration: Duration, error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            duration,
            line_coverage: None,
            files: HashMap::new(),
        }
    }

    pub fn skipped() -> Self {
        Self {
            success: true,
            error: None,
            duration: Duration::ZERO,
            line_coverage: None,
            files: HashMap::new(),
        }
    }
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
```

**Update:** `crates/cli/src/checks/tests/runners/mod.rs`

```rust
mod coverage;
pub use coverage::CoverageResult;
```

**Milestone:** Coverage module compiles with result types.

### Phase 2: llvm-cov Availability Check

Implement detection for `cargo-llvm-cov` installation.

**File:** `crates/cli/src/checks/tests/runners/coverage.rs`

```rust
/// Check if cargo-llvm-cov is available.
pub fn llvm_cov_available() -> bool {
    Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

**Milestone:** Can detect if llvm-cov is installed.

### Phase 3: Execute cargo llvm-cov

Run `cargo llvm-cov --json` and capture output.

```rust
/// Collect coverage for a Rust project.
pub fn collect_rust_coverage(root: &Path, path: Option<&str>) -> CoverageResult {
    use std::time::Instant;

    if !llvm_cov_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();

    let mut cmd = Command::new("cargo");
    cmd.args(["llvm-cov", "--json", "--release"]);

    // Set working directory
    let work_dir = path
        .map(|p| root.join(p))
        .unwrap_or_else(|| root.to_path_buf());
    cmd.current_dir(&work_dir);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            return CoverageResult::failed(
                start.elapsed(),
                format!("failed to run cargo llvm-cov: {e}"),
            );
        }
    };

    let duration = start.elapsed();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
        return CoverageResult::failed(duration, format!("cargo llvm-cov failed:\n{msg}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_llvm_cov_json(&stdout, duration)
}
```

**Milestone:** Can execute `cargo llvm-cov --json` and capture output.

### Phase 4: Parse llvm-cov JSON Report

Parse the JSON output format from `cargo llvm-cov --json`.

**llvm-cov JSON format (LLVM export format):**

```json
{
  "data": [
    {
      "totals": {
        "lines": { "count": 100, "covered": 80, "percent": 80.0 }
      },
      "files": [
        {
          "filename": "/path/to/src/lib.rs",
          "summary": {
            "lines": { "count": 50, "covered": 40, "percent": 80.0 }
          }
        }
      ]
    }
  ],
  "type": "llvm.coverage.json.export",
  "version": "2.0.1"
}
```

**Implementation:**

```rust
#[derive(Debug, Deserialize)]
struct LlvmCovReport {
    data: Vec<LlvmCovData>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovData {
    totals: LlvmCovSummary,
    files: Vec<LlvmCovFile>,
}

#[derive(Debug, Deserialize)]
struct LlvmCovSummary {
    lines: LlvmCovLines,
}

#[derive(Debug, Deserialize)]
struct LlvmCovLines {
    count: u64,
    covered: u64,
    percent: f64,
}

#[derive(Debug, Deserialize)]
struct LlvmCovFile {
    filename: String,
    summary: LlvmCovSummary,
}

fn parse_llvm_cov_json(json: &str, duration: Duration) -> CoverageResult {
    let report: LlvmCovReport = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => {
            return CoverageResult::failed(duration, format!("failed to parse coverage JSON: {e}"));
        }
    };

    // Get first data entry (typically only one)
    let Some(data) = report.data.first() else {
        return CoverageResult::failed(duration, "no coverage data in report");
    };

    // Extract overall line coverage
    let line_coverage = data.totals.lines.percent;

    // Extract per-file coverage
    let mut files = HashMap::new();
    for file in &data.files {
        // Normalize path: remove workspace prefix, keep relative
        let path = normalize_coverage_path(&file.filename);
        files.insert(path, file.summary.lines.percent);
    }

    CoverageResult {
        success: true,
        error: None,
        duration,
        line_coverage: Some(line_coverage),
        files,
    }
}

/// Normalize coverage paths to workspace-relative.
fn normalize_coverage_path(path: &str) -> String {
    // llvm-cov reports absolute paths; extract relative portion
    // Heuristic: find "src/" or "tests/" and keep from there
    for marker in ["src/", "tests/"] {
        if let Some(idx) = path.find(marker) {
            return path[idx..].to_string();
        }
    }
    // Fallback: use filename only
    std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}
```

**Milestone:** Can parse llvm-cov JSON and extract coverage percentages.

### Phase 5: Integrate into CargoRunner

Update `CargoRunner::run()` to collect coverage when requested.

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

```rust
use super::coverage::collect_rust_coverage;

impl TestRunner for CargoRunner {
    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // ... existing test execution code ...
        let mut result = parse_cargo_output(&stdout, total_time);

        // Collect coverage if requested
        if ctx.collect_coverage {
            let coverage = collect_rust_coverage(ctx.root, config.path.as_deref());
            if coverage.success && coverage.line_coverage.is_some() {
                let mut cov_map = HashMap::new();
                cov_map.insert("rust".to_string(), coverage.line_coverage.unwrap());
                result = result.with_coverage(cov_map);

                // Store per-file data in extended metrics (future use)
                // result.coverage_files = Some(coverage.files);
            }
        }

        result
    }
}
```

**Note:** The `RunnerContext.collect_coverage` flag is already defined in the codebase. The tests check needs to set this based on CI mode or explicit configuration.

**Milestone:** CargoRunner collects and reports coverage when enabled.

### Phase 6: Unit Tests

Create comprehensive unit tests for coverage parsing.

**File:** `crates/cli/src/checks/tests/runners/coverage_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_llvm_cov_json_report() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 100, "covered": 75, "percent": 75.0 } },
            "files": [
                {
                    "filename": "/home/user/project/src/lib.rs",
                    "summary": { "lines": { "count": 60, "covered": 50, "percent": 83.33 } }
                },
                {
                    "filename": "/home/user/project/src/utils.rs",
                    "summary": { "lines": { "count": 40, "covered": 25, "percent": 62.5 } }
                }
            ]
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_secs(1));

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(75.0));
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files.get("src/lib.rs"), Some(&83.33));
    assert_eq!(result.files.get("src/utils.rs"), Some(&62.5));
}

#[test]
fn handles_empty_coverage_data() {
    let json = r#"{ "data": [], "type": "llvm.coverage.json.export", "version": "2.0.1" }"#;
    let result = parse_llvm_cov_json(json, Duration::from_secs(0));

    assert!(!result.success);
    assert!(result.error.is_some());
}

#[test]
fn handles_malformed_json() {
    let result = parse_llvm_cov_json("not json", Duration::from_secs(0));

    assert!(!result.success);
    assert!(result.error.unwrap().contains("failed to parse"));
}

#[test]
fn normalizes_coverage_paths() {
    assert_eq!(
        normalize_coverage_path("/home/user/project/src/lib.rs"),
        "src/lib.rs"
    );
    assert_eq!(
        normalize_coverage_path("/workspace/tests/basic.rs"),
        "tests/basic.rs"
    );
    assert_eq!(
        normalize_coverage_path("/unknown/path/file.rs"),
        "file.rs"
    );
}

#[test]
fn extracts_overall_line_coverage() {
    let json = r#"{
        "data": [{
            "totals": { "lines": { "count": 200, "covered": 180, "percent": 90.0 } },
            "files": []
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;

    let result = parse_llvm_cov_json(json, Duration::from_millis(500));

    assert!(result.success);
    assert_eq!(result.line_coverage, Some(90.0));
    assert!(result.files.is_empty());
}
```

**Milestone:** Unit tests pass for all coverage parsing scenarios.

### Phase 7: Test Fixture and Behavioral Specs

Create fixture and enable behavioral specs.

**File:** `tests/fixtures/rust-coverage/Cargo.toml`

```toml
[package]
name = "rust_coverage"
version = "0.1.0"
edition = "2021"

[lib]
name = "rust_coverage"
path = "src/lib.rs"
```

**File:** `tests/fixtures/rust-coverage/src/lib.rs`

```rust
/// Covered function - called by test.
pub fn covered_function() -> i32 {
    42
}

/// Uncovered function - not called by any test.
pub fn uncovered_function() -> i32 {
    0
}
```

**File:** `tests/fixtures/rust-coverage/tests/basic.rs`

```rust
#[test]
fn test_covered() {
    assert_eq!(rust_coverage::covered_function(), 42);
}
```

**Update:** `tests/specs/checks/tests/coverage.rs`

```rust
#[test]
// Remove: #[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_collects_rust_coverage() {
    // ... existing test
}
```

**Note:** The behavioral spec `cargo_runner_collects_rust_coverage()` already exists and tests the expected behavior. It creates a temp project with partial coverage and verifies the metrics include a `coverage.rust` value around 50%.

**Milestone:** Behavioral specs pass when llvm-cov is installed.

## Key Implementation Details

### cargo llvm-cov Command

```bash
cargo llvm-cov --json --release
```

Flags:
- `--json` - Output LLVM JSON export format (machine-readable)
- `--release` - Use release profile (consistent with test execution)

Alternative: `--lcov` outputs LCOV format if JSON parsing proves problematic.

### JSON Report Structure

The `llvm.coverage.json.export` format is the LLVM coverage export format:

| Path | Type | Description |
|------|------|-------------|
| `data[0].totals.lines.percent` | f64 | Overall line coverage % |
| `data[0].files[].filename` | string | Absolute file path |
| `data[0].files[].summary.lines.percent` | f64 | Per-file line coverage % |

### Path Normalization

llvm-cov reports absolute paths. Normalize by finding `src/` or `tests/` markers:

```
/home/user/project/src/lib.rs -> src/lib.rs
/workspace/tests/unit.rs -> tests/unit.rs
```

### Graceful Degradation

If `cargo-llvm-cov` is not installed:
- `llvm_cov_available()` returns false
- `collect_rust_coverage()` returns `CoverageResult::skipped()`
- Tests still run and report timing, just without coverage data

### Coverage Data Flow

```
CargoRunner::run()
    |
    +-> cargo test --release (timing + pass/fail)
    |
    +-> if ctx.collect_coverage:
            collect_rust_coverage()
                |
                +-> cargo llvm-cov --json --release
                |
                +-> parse_llvm_cov_json()
                |
                +-> CoverageResult { line_coverage, files }
    |
    +-> TestRunResult.with_coverage({"rust": line_coverage})
```

### Metrics Aggregation

In `TestsCheck`, coverage is reported in metrics:

```json
{
  "metrics": {
    "test_count": 10,
    "total_ms": 1234,
    "coverage": {
      "rust": 75.5
    }
  }
}
```

Per-file coverage data is available for detailed reporting but not included in summary metrics.

## Verification Plan

1. **Unit tests:** Run coverage parsing tests
   ```bash
   cargo test -p quench -- coverage
   ```

2. **Behavioral specs:** Remove `#[ignore]` and run (requires `cargo-llvm-cov` installed)
   ```bash
   cargo install cargo-llvm-cov  # If not installed
   cargo test --test specs -- cargo_runner_collects_rust_coverage
   ```

3. **Manual verification:** Run on fixture
   ```bash
   cd tests/fixtures/rust-coverage
   cargo llvm-cov --json  # Verify tool works
   cargo run -- check tests --ci  # Should show coverage in metrics
   ```

4. **Full check suite:**
   ```bash
   make check
   ```

## Checklist

- [ ] Phase 1: Coverage module skeleton compiles
- [ ] Phase 2: llvm-cov availability detection works
- [ ] Phase 3: `cargo llvm-cov --json` executes
- [ ] Phase 4: JSON report parsing extracts coverage
- [ ] Phase 5: CargoRunner integrates coverage collection
- [ ] Phase 6: Unit tests in `coverage_tests.rs` pass
- [ ] Phase 7: Behavioral specs pass (with llvm-cov installed)
- [ ] `make check` passes
