# Phase 930: Test Runners - Bats

## Overview

Implement a Bats test runner that executes shell tests using `bats --timing` and parses TAP output to extract per-test timing and pass/fail status. The runner follows the established `TestRunner` trait pattern used by `CargoRunner`.

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── mod.rs              # Register BatsRunner in all_runners()
├── bats.rs             # NEW: BatsRunner implementation
└── bats_tests.rs       # NEW: Unit tests for TAP parsing

tests/
├── specs/checks/tests/runners.rs  # Update ignored specs
└── fixtures/shell-scripts/        # Integration test fixture
    ├── quench.toml                # Add [[check.tests.suite]] runner="bats"
    ├── tests/scripts.bats         # Existing bats tests
    └── scripts/                   # Shell scripts under test
```

## Dependencies

No new external dependencies required. Uses:
- `std::process::Command` for executing `bats`
- `std::time::{Duration, Instant}` for timing
- Existing `TestRunner` trait and result types

## Implementation Phases

### Phase 1: TAP Output Parser

Create the TAP (Test Anything Protocol) parser for Bats output.

**File:** `crates/cli/src/checks/tests/runners/bats.rs`

TAP output format from `bats --timing`:

```
1..2
ok 1 build script runs successfully in 45ms
not ok 2 deploy script accepts target argument in 12ms
```

Key parsing elements:
- Test plan line: `1..N` (total test count)
- Test result lines: `ok N <description>` or `not ok N <description>`
- Timing: `in Xms` or `in X.XXs` suffix (when `--timing` used)

```rust
/// Parse TAP output from bats --timing.
fn parse_tap_output(stdout: &str, total_time: Duration) -> TestRunResult {
    let mut tests = Vec::new();
    let mut all_passed = true;

    for line in stdout.lines() {
        let line = line.trim();

        // Skip plan line (1..N) and comments (# ...)
        if line.starts_with("1..") || line.starts_with('#') {
            continue;
        }

        // Parse test result: "ok N description" or "not ok N description"
        if let Some(result) = parse_tap_line(line) {
            if !result.passed {
                all_passed = false;
            }
            tests.push(result);
        }
    }

    let mut result = if all_passed {
        TestRunResult::passed(total_time)
    } else {
        TestRunResult::failed(total_time, "tests failed")
    };
    result.tests = tests;
    result
}

/// Parse a single TAP result line.
fn parse_tap_line(line: &str) -> Option<TestResult> {
    let (passed, rest) = if line.starts_with("ok ") {
        (true, &line[3..])
    } else if line.starts_with("not ok ") {
        (false, &line[7..])
    } else {
        return None;
    };

    // Skip test number, get description
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ');

    // Extract timing if present: "description in Xms"
    let (name, duration) = extract_timing(rest);

    Some(if passed {
        TestResult::passed(name, duration)
    } else {
        TestResult::failed(name, duration)
    })
}

/// Extract timing from TAP description suffix.
fn extract_timing(desc: &str) -> (String, Duration) {
    // Pattern: "description in 45ms" or "description in 1.234s"
    if let Some(idx) = desc.rfind(" in ") {
        let timing_part = &desc[idx + 4..];
        if let Some(duration) = parse_duration(timing_part) {
            return (desc[..idx].to_string(), duration);
        }
    }
    (desc.to_string(), Duration::ZERO)
}
```

### Phase 2: BatsRunner Implementation

Implement the `TestRunner` trait for Bats.

```rust
pub struct BatsRunner;

impl TestRunner for BatsRunner {
    fn name(&self) -> &'static str {
        "bats"
    }

    fn available(&self, ctx: &RunnerContext) -> bool {
        // Check if bats is installed
        which::which("bats").is_ok()
            // And test directory exists (if specified)
            && ctx.root.join(self.test_path_or_default(None)).exists()
    }

    fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
        // Run setup if specified
        if let Some(setup) = &config.setup {
            if let Err(e) = super::run_setup_command(setup, ctx.root) {
                return TestRunResult::failed(Duration::ZERO, e);
            }
        }

        let start = Instant::now();

        // Build command: bats --timing <path>
        let mut cmd = Command::new("bats");
        cmd.arg("--timing");

        // Add test path (default: tests/)
        let test_path = config.path.as_deref().unwrap_or("tests/");
        cmd.arg(test_path);

        cmd.current_dir(ctx.root);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match cmd.output() {
            Ok(out) => out,
            Err(e) => {
                return TestRunResult::failed(
                    start.elapsed(),
                    format!("failed to run bats: {e}")
                );
            }
        };

        let total_time = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout);

        parse_tap_output(&stdout, total_time)
    }
}
```

### Phase 3: Register Runner and Update Fixtures

1. **Update `mod.rs`** to register `BatsRunner`:

```rust
// In crates/cli/src/checks/tests/runners/mod.rs

mod bats;
pub use bats::BatsRunner;

pub fn all_runners() -> Vec<Arc<dyn TestRunner>> {
    vec![
        Arc::new(CargoRunner),
        Arc::new(BatsRunner),  // Add concrete implementation
        // Stub implementations...
        Arc::new(StubRunner::new("go")),
        // ...
    ]
}
```

2. **Update fixture** `tests/fixtures/shell-scripts/quench.toml`:

```toml
version = 1

[project]
name = "shell-scripts"

[[check.tests.suite]]
runner = "bats"
path = "tests/"
```

### Phase 4: Unit Tests

**File:** `crates/cli/src/checks/tests/runners/bats_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::time::Duration;

#[test]
fn parses_passing_test() {
    let output = "1..1\nok 1 example test in 45ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(result.tests[0].passed);
    assert_eq!(result.tests[0].name, "example test");
    assert_eq!(result.tests[0].duration, Duration::from_millis(45));
}

#[test]
fn parses_failing_test() {
    let output = "1..1\nnot ok 1 should pass in 12ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(!result.tests[0].passed);
}

#[test]
fn parses_multiple_tests() {
    let output = r#"1..3
ok 1 first test in 10ms
ok 2 second test in 20ms
not ok 3 third test in 30ms
"#;
    let result = parse_tap_output(output, Duration::from_millis(100));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 3);
    assert_eq!(result.passed_count(), 2);
    assert_eq!(result.failed_count(), 1);
}

#[test]
fn extracts_timing_in_milliseconds() {
    let (name, duration) = extract_timing("test name in 123ms");
    assert_eq!(name, "test name");
    assert_eq!(duration, Duration::from_millis(123));
}

#[test]
fn extracts_timing_in_seconds() {
    let (name, duration) = extract_timing("test name in 1.5s");
    assert_eq!(name, "test name");
    assert_eq!(duration, Duration::from_millis(1500));
}

#[test]
fn handles_missing_timing() {
    let (name, duration) = extract_timing("test without timing");
    assert_eq!(name, "test without timing");
    assert_eq!(duration, Duration::ZERO);
}

#[test]
fn ignores_tap_comments() {
    let output = "1..1\n# diagnostic message\nok 1 test in 10ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert_eq!(result.tests.len(), 1);
}
```

### Phase 5: Enable Behavioral Specs

Update `tests/specs/checks/tests/runners.rs`:

```rust
// Remove #[ignore] from these tests:

#[test]
fn bats_runner_executes_bats_with_timing() { ... }

#[test]
fn bats_runner_parses_tap_timing() { ... }
```

### Phase 6: Integration Test

Add integration spec using the `shell-scripts` fixture:

```rust
/// Spec: Integration test on fixtures/shell-scripts
#[test]
fn bats_runner_on_shell_scripts_fixture() {
    check("tests").on("shell-scripts").passes();
}
```

## Key Implementation Details

### TAP Format Variations

Bats TAP output varies slightly by version. Key patterns to handle:

| Pattern | Example | Notes |
|---------|---------|-------|
| Plan first | `1..5` | Test count declaration |
| Plan last | `1..5` | Some versions output at end |
| Passing | `ok 1 description` | Simple pass |
| Failing | `not ok 1 description` | Simple fail |
| Timing | `... in 45ms` | `--timing` flag suffix |
| Comment | `# message` | Diagnostic, skip |
| TODO | `ok 1 # TODO reason` | Skip (not implemented) |
| SKIP | `ok 1 # SKIP reason` | Skip (conditional) |

### Duration Parsing

```rust
fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(secs) = s.strip_suffix('s') {
        secs.parse::<f64>().ok().map(|s| Duration::from_secs_f64(s))
    } else {
        None
    }
}
```

### Availability Check

The runner checks both:
1. `bats` is installed (via `which::which`)
2. Test directory exists (config `path` or default `tests/`)

If bats is not installed, the suite is skipped with a clear message.

## Verification Plan

### Unit Tests (Phase 4)

```bash
cargo test --package quench -- runners::bats
```

Verifies:
- TAP output parsing (ok/not ok)
- Per-test timing extraction (ms/s formats)
- Multiple test aggregation
- Pass/fail determination

### Behavioral Specs (Phase 5)

```bash
cargo test --test specs -- bats
```

Verifies:
- Runner executes `bats --timing`
- Test count extracted from TAP
- Integration with check framework

### Integration Test (Phase 6)

```bash
cargo test --test specs -- shell_scripts
```

Verifies:
- End-to-end on real fixture
- Setup command execution
- Timing metrics collection

### Full Verification

```bash
make check
```

Runs all checks per project conventions.

## Files Modified

| File | Change |
|------|--------|
| `crates/cli/src/checks/tests/runners/mod.rs` | Register BatsRunner |
| `crates/cli/src/checks/tests/runners/bats.rs` | NEW: Runner implementation |
| `crates/cli/src/checks/tests/runners/bats_tests.rs` | NEW: Unit tests |
| `tests/specs/checks/tests/runners.rs` | Enable bats specs |
| `tests/fixtures/shell-scripts/quench.toml` | Add test suite config |

## Dependencies

- `which` crate (already in dependencies for tool detection)
