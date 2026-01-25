# Phase 920: Test Runners - Cargo

Implement the Cargo test runner for the Quench test framework.

## Overview

Add a working `cargo` test runner that executes `cargo test --release -- --format json`, parses the JSON output to extract per-test timing, pass/fail status, and test counts, and integrates with the existing test runner framework.

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── mod.rs              # Update: replace StubRunner with CargoRunner
├── cargo.rs            # NEW: CargoRunner implementation
├── cargo_tests.rs      # NEW: Unit tests for JSON parsing
├── result.rs           # Existing: TestResult, TestRunResult types
└── stub.rs             # Existing: Stub for unimplemented runners
```

Test fixtures:
```
tests/fixtures/rust-simple/    # Existing fixture with tests
tests/specs/checks/tests/
└── runners.rs                 # Update: enable Phase 920 specs
```

## Dependencies

No new external dependencies required. Uses existing stdlib:
- `std::process::Command` - Execute cargo test
- `serde_json` - Parse JSON output (already in deps)
- `std::time::Duration` - Timing

## Implementation Phases

### Phase 1: CargoRunner Skeleton

Create the `CargoRunner` struct implementing `TestRunner` trait.

**File:** `crates/cli/src/checks/tests/runners/cargo.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cargo test runner.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{RunnerContext, TestResult, TestRunResult, TestRunner};
use crate::config::TestSuiteConfig;

/// Cargo test runner for Rust projects.
pub struct CargoRunner;

impl TestRunner for CargoRunner {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check for Cargo.toml in project root
        ctx.root.join("Cargo.toml").exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Phase 2-4 implementation
        todo!()
    }
}

#[cfg(test)]
#[path = "cargo_tests.rs"]
mod tests;
```

**Update:** `crates/cli/src/checks/tests/runners/mod.rs`

```rust
mod cargo;
pub use cargo::CargoRunner;

// In all_runners():
Arc::new(CargoRunner) // Replace StubRunner::new("cargo")
```

**Milestone:** `CargoRunner` compiles and is registered in the runner list.

### Phase 2: Command Execution

Implement `cargo test --release -- --format json` execution.

**Key details:**
- Working directory: config `path` or project root
- Arguments: `test`, `--release`, `--`, `--format`, `json`
- Handle setup command if specified
- Capture stdout (JSON output) and stderr (compiler output)

```rust
fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
    // Run setup command if specified
    if let Some(setup) = &config.setup {
        if let Err(e) = super::run_setup_command(setup, ctx.root) {
            return TestRunResult::failed(Duration::ZERO, e);
        }
    }

    let start = Instant::now();

    // Build command
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--release", "--", "--format", "json"]);

    // Set working directory
    let work_dir = config.path.as_ref()
        .map(|p| ctx.root.join(p))
        .unwrap_or_else(|| ctx.root.to_path_buf());
    cmd.current_dir(&work_dir);

    // Capture output
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => return TestRunResult::failed(start.elapsed(), format!("failed to run cargo: {e}")),
    };

    let total_time = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output (Phase 3)
    parse_cargo_json_output(&stdout, total_time)
}
```

**Milestone:** Cargo test executes and captures output.

### Phase 3: JSON Output Parsing

Parse `cargo test --format json` newline-delimited JSON output.

**JSON event types:**

```json
{"type":"suite","event":"started","test_count":3}
{"type":"test","event":"started","name":"tests::test_add"}
{"type":"test","name":"tests::test_add","event":"ok","exec_time":0.001}
{"type":"test","event":"started","name":"tests::test_sub"}
{"type":"test","name":"tests::test_sub","event":"failed","exec_time":0.002,"stdout":"assertion failed"}
{"type":"suite","event":"failed","passed":1,"failed":1,"ignored":0}
```

**Implementation:**

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CargoTestEvent {
    #[serde(rename = "type")]
    event_type: String,
    event: String,
    name: Option<String>,
    exec_time: Option<f64>,
    passed: Option<u32>,
    failed: Option<u32>,
    ignored: Option<u32>,
}

fn parse_cargo_json_output(stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut suite_passed = true;

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let event: CargoTestEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue, // Skip non-JSON lines (compiler output)
        };

        match (event.event_type.as_str(), event.event.as_str()) {
            ("test", "ok") => {
                if let (Some(name), Some(time)) = (event.name, event.exec_time) {
                    tests.push(TestResult::passed(name, Duration::from_secs_f64(time)));
                }
            }
            ("test", "failed") => {
                if let (Some(name), Some(time)) = (event.name, event.exec_time) {
                    tests.push(TestResult::failed(name, Duration::from_secs_f64(time)));
                    suite_passed = false;
                }
            }
            ("suite", "failed") => {
                suite_passed = false;
            }
            _ => {}
        }
    }

    if suite_passed {
        TestRunResult::passed(total_time).with_tests(tests)
    } else {
        TestRunResult::failed(total_time, "tests failed").with_tests(tests)
    }
}
```

**Milestone:** JSON parsing extracts test names, pass/fail, and timing.

### Phase 4: Error Handling & Edge Cases

Handle edge cases and improve robustness:

1. **Compilation errors:** Detect when cargo test fails to compile
2. **No tests found:** Handle empty test suites
3. **Timeout handling:** Future work (config.max_total)
4. **Ignored tests:** Track ignored test count

```rust
fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
    // ... command execution ...

    // Check if cargo command itself failed (compilation error)
    if !output.status.success() && tests.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
        return TestRunResult::failed(total_time, format!("cargo test failed:\n{msg}"));
    }

    // ... rest of implementation
}
```

**Milestone:** Robust error handling for all failure modes.

### Phase 5: Unit Tests

Create comprehensive unit tests for JSON parsing.

**File:** `crates/cli/src/checks/tests/runners/cargo_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_passing_test() {
    let json = r#"
{"type":"suite","event":"started","test_count":1}
{"type":"test","event":"started","name":"tests::add"}
{"type":"test","name":"tests::add","event":"ok","exec_time":0.001}
{"type":"suite","event":"ok","passed":1,"failed":0,"ignored":0}
"#;
    let result = parse_cargo_json_output(json, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(result.tests[0].passed);
    assert_eq!(result.tests[0].name, "tests::add");
}

#[test]
fn parses_failing_test() {
    let json = r#"
{"type":"suite","event":"started","test_count":1}
{"type":"test","event":"started","name":"tests::fail"}
{"type":"test","name":"tests::fail","event":"failed","exec_time":0.002}
{"type":"suite","event":"failed","passed":0,"failed":1,"ignored":0}
"#;
    let result = parse_cargo_json_output(json, Duration::from_secs(1));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(!result.tests[0].passed);
}

#[test]
fn handles_mixed_output() {
    // Cargo sometimes emits non-JSON lines (compiler warnings)
    let json = r#"
   Compiling test_project v0.1.0
    Finished release target(s) in 0.1s
     Running tests
{"type":"suite","event":"started","test_count":2}
{"type":"test","event":"started","name":"tests::a"}
{"type":"test","name":"tests::a","event":"ok","exec_time":0.001}
{"type":"test","event":"started","name":"tests::b"}
{"type":"test","name":"tests::b","event":"ok","exec_time":0.002}
{"type":"suite","event":"ok","passed":2,"failed":0,"ignored":0}
"#;
    let result = parse_cargo_json_output(json, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
}

#[test]
fn handles_empty_output() {
    let result = parse_cargo_json_output("", Duration::from_secs(0));
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn extracts_timing() {
    let json = r#"{"type":"test","name":"slow_test","event":"ok","exec_time":1.234}"#;
    let result = parse_cargo_json_output(json, Duration::from_secs(2));

    assert_eq!(result.tests.len(), 1);
    assert_eq!(result.tests[0].duration, Duration::from_millis(1234));
}
```

**Milestone:** Unit tests pass for all parsing scenarios.

### Phase 6: Integration Specs

Update behavioral specs and verify against fixture.

**File:** `tests/specs/checks/tests/runners.rs` - Update ignore tags:

```rust
#[test]
// Remove: #[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_executes_cargo_test() {
    // ... existing test
}

#[test]
// Remove: #[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_reports_test_count() {
    // ... existing test
}
```

Also add a spec that runs against `rust-simple` fixture:

```rust
/// Spec: Integration test on fixtures/rust-simple
#[test]
fn cargo_runner_on_rust_simple_fixture() {
    check("tests")
        .on("rust-simple")
        .with_config(r#"
[[check.tests.suite]]
runner = "cargo"
"#)
        .passes();
}
```

**Milestone:** All cargo runner specs pass.

## Key Implementation Details

### Cargo JSON Format

Cargo's `--format json` outputs newline-delimited JSON (JSONL). Each line is a separate JSON object. Key event types:

| Type | Event | Fields |
|------|-------|--------|
| `suite` | `started` | `test_count` |
| `suite` | `ok`/`failed` | `passed`, `failed`, `ignored` |
| `test` | `started` | `name` |
| `test` | `ok`/`failed` | `name`, `exec_time` |
| `test` | `ignored` | `name` |

### Timing Precision

`exec_time` is a float in seconds. Convert to `Duration`:
```rust
Duration::from_secs_f64(exec_time)
```

### Mixed Output Handling

Cargo may emit non-JSON lines (compilation status, warnings). The parser should skip lines that don't parse as JSON.

### Test Name Format

Test names follow the format: `module::submodule::test_fn` or `crate::module::test_fn`.

## Verification Plan

1. **Unit tests:** Run `cargo test` in quench workspace
   ```bash
   cargo test -p quench -- cargo
   ```

2. **Behavioral specs:** Remove `#[ignore]` and run
   ```bash
   cargo test --test specs -- cargo
   ```

3. **Manual verification:** Run on fixture
   ```bash
   cd tests/fixtures/rust-simple
   cargo run -- check tests
   ```

4. **Full check suite:**
   ```bash
   make check
   ```

## Checklist

- [ ] Phase 1: CargoRunner skeleton compiles
- [ ] Phase 2: `cargo test` command executes
- [ ] Phase 3: JSON output parsing works
- [ ] Phase 4: Error handling complete
- [ ] Phase 5: Unit tests in `cargo_tests.rs`
- [ ] Phase 6: Behavioral specs pass
- [ ] `make check` passes
