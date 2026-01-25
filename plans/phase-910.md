# Phase 910: Test Runners - Specs

**Reference:** `docs/specs/11-test-runners.md`
**Type:** Behavioral Specs (black-box tests)

## Overview

This phase writes behavioral specifications for the test runners feature. Test runners execute test suites and collect timing and coverage metrics. The specs will test CLI behavior as a black box, verifying:

1. **Cargo runner** - Executes `cargo test`, extracts per-test timing
2. **Bats runner** - Executes `bats --timing`, parses TAP output
3. **Rust coverage** - Collects coverage via llvm-cov
4. **Shell coverage** - Collects coverage via kcov
5. **Coverage merging** - Aggregates coverage from multiple suites

All specs will be marked with `#[ignore = "TODO: Phase XXX - description"]` until the feature is implemented.

## Project Structure

```
tests/specs/checks/tests/
├── mod.rs              # MODIFY: Add new modules
├── correlation.rs      # (existing)
├── output.rs           # (existing)
├── runners.rs          # NEW: Runner execution specs
├── timing.rs           # NEW: Per-test timing extraction specs
└── coverage.rs         # NEW: Coverage collection and merging specs
tests/fixtures/
└── tests/              # NEW: Test runner fixtures
    ├── cargo-basic/    # Minimal Rust project with tests
    ├── cargo-timing/   # Rust project with varied test durations
    ├── bats-basic/     # Shell tests using bats
    ├── bats-timing/    # Bats tests with timing
    └── multi-suite/    # Project with multiple test suites
```

## Dependencies

No new dependencies. Specs use existing test helpers from `tests/specs/prelude.rs`.

**Note:** Fixtures may need actual test files that can be executed. Consider whether to:
- Use minimal real projects (slower but realistic)
- Mock runner output (faster but less integration)

The specs should test CLI behavior, so mocking internal runner execution may be appropriate for speed.

## Implementation Phases

### Phase 1: Create Runner Execution Specs

**Goal**: Specs for basic runner execution (cargo and bats).

**File**: `tests/specs/checks/tests/runners.rs`

```rust
//! Behavioral specs for test runner execution.
//!
//! Reference: docs/specs/11-test-runners.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CARGO RUNNER SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#cargo
///
/// > cargo test --release -- --format json
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_executes_cargo_test() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file("tests/basic.rs", r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#);

    // Runner should execute cargo test and report results
    check("tests")
        .pwd(temp.path())
        .passes()
        .stdout_has("tests: PASS");
}

/// Spec: docs/specs/11-test-runners.md#cargo
///
/// > Parses Rust's JSON test output for per-test timing.
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_reports_test_count() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", "");
    temp.file("tests/a.rs", "#[test] fn t1() {} #[test] fn t2() {}");
    temp.file("tests/b.rs", "#[test] fn t3() {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report test count
    assert_eq!(metrics.get("test_count").and_then(|v| v.as_i64()), Some(3));
}

// =============================================================================
// BATS RUNNER SPECS
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > bats --timing tests/
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn bats_runner_executes_bats_with_timing() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#);
    temp.file("tests/basic.bats", r#"
@test "example test" {
    [ 1 -eq 1 ]
}
"#);

    // Runner should execute bats --timing
    check("tests")
        .pwd(temp.path())
        .passes()
        .stdout_has("tests: PASS");
}

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > Parses BATS TAP output with timing information.
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn bats_runner_parses_tap_timing() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#);
    temp.file("tests/a.bats", r#"
@test "test one" { sleep 0.1; }
@test "test two" { true; }
"#);

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report test count from TAP output
    assert_eq!(metrics.get("test_count").and_then(|v| v.as_i64()), Some(2));
}
```

**Verification**:
```bash
cargo test --test specs -- runners --ignored 2>&1 | grep -E "(ignored|FAILED)"
# Should show specs as ignored, not failing to compile
```

---

### Phase 2: Create Timing Extraction Specs

**Goal**: Specs for per-test timing metric extraction.

**File**: `tests/specs/checks/tests/timing.rs`

```rust
//! Behavioral specs for per-test timing extraction.
//!
//! Reference: docs/specs/11-test-runners.md#timing-metrics

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CARGO TIMING EXTRACTION
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#per-test-timing
///
/// > Average: Mean time per test
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_extracts_average_timing() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", "");
    temp.file("tests/timing.rs", r#"
use std::thread::sleep;
use std::time::Duration;

#[test] fn fast_test() { sleep(Duration::from_millis(10)); }
#[test] fn slow_test() { sleep(Duration::from_millis(100)); }
"#);

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should have average timing
    assert!(metrics.get("avg_ms").is_some());
}

/// Spec: docs/specs/11-test-runners.md#per-test-timing
///
/// > Max: Slowest individual test (with name)
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_extracts_max_timing_with_name() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", "");
    temp.file("tests/timing.rs", r#"
use std::thread::sleep;
use std::time::Duration;

#[test] fn fast_test() { sleep(Duration::from_millis(10)); }
#[test] fn slowest_test() { sleep(Duration::from_millis(200)); }
#[test] fn medium_test() { sleep(Duration::from_millis(50)); }
"#);

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should identify slowest test by name
    let max_test = metrics.get("max_test").and_then(|v| v.as_str());
    assert!(max_test.unwrap().contains("slowest_test"));
}

/// Spec: docs/specs/11-test-runners.md#timing-metrics
///
/// > Total Time: Wall-clock time for entire test suite.
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn runner_reports_total_time() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", "");
    temp.file("tests/basic.rs", "#[test] fn t() {}");

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should report total time
    assert!(metrics.get("total_ms").is_some());
}

// =============================================================================
// BATS TIMING EXTRACTION
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#bats
///
/// > Parses BATS TAP output with timing information.
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn bats_runner_extracts_per_test_timing() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
"#);
    temp.file("tests/timing.bats", r#"
@test "fast test" { true; }
@test "slow test" { sleep 0.2; }
"#);

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should extract timing from TAP output
    assert!(metrics.get("max_ms").is_some());
    let max_test = metrics.get("max_test").and_then(|v| v.as_str());
    assert!(max_test.unwrap().contains("slow test"));
}
```

**Verification**:
```bash
cargo test --test specs -- timing --ignored 2>&1 | grep -E "(ignored|FAILED)"
```

---

### Phase 3: Create Coverage Collection Specs

**Goal**: Specs for coverage collection (Rust via llvm-cov, shell via kcov).

**File**: `tests/specs/checks/tests/coverage.rs`

```rust
//! Behavioral specs for test coverage collection.
//!
//! Reference: docs/specs/11-test-runners.md#coverage-targets

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST COVERAGE (llvm-cov)
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > `cargo` runner provides implicit Rust coverage via llvm-cov.
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn cargo_runner_collects_rust_coverage() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
# Implicit: targets Rust code via llvm-cov
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#);
    temp.file("src/lib.rs", r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#);
    temp.file("tests/basic.rs", r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#);

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Should report Rust coverage percentage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let rust_coverage = coverage.unwrap().get("rust").and_then(|v| v.as_f64());
    assert!(rust_coverage.is_some());
    // Coverage should be ~50% (one function covered, one not)
    let pct = rust_coverage.unwrap();
    assert!(pct > 40.0 && pct < 60.0, "Expected ~50% coverage, got {}", pct);
}

// =============================================================================
// SHELL COVERAGE (kcov)
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#explicit-coverage
///
/// > Shell scripts via kcov: targets = ["scripts/*.sh"]
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn bats_runner_collects_shell_coverage_via_kcov() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/"
targets = ["scripts/*.sh"]  # Shell scripts via kcov
"#);
    temp.file("scripts/helper.sh", r#"#!/bin/bash
covered_function() { echo "covered"; }
uncovered_function() { echo "uncovered"; }
"#);
    temp.file("tests/helper.bats", r#"
setup() { source scripts/helper.sh; }

@test "calls covered function" {
    run covered_function
    [ "$output" = "covered" ]
}
"#);

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Should report shell coverage
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some());

    let shell_coverage = coverage.unwrap().get("shell").and_then(|v| v.as_f64());
    assert!(shell_coverage.is_some());
}

/// Spec: docs/specs/11-test-runners.md#explicit-coverage
///
/// > targets = ["myapp"] - Instrument Rust binary for coverage
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn bats_runner_collects_rust_binary_coverage() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
targets = ["myapp"]  # Instrument Rust binary
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#);
    temp.file("src/main.rs", r#"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "greet" {
        println!("Hello!");
    } else {
        println!("Usage: myapp greet");
    }
}
"#);
    temp.file("tests/cli/basic.bats", r#"
@test "greet command" {
    run ./target/debug/myapp greet
    [ "$output" = "Hello!" ]
}
"#);

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Should report coverage for Rust binary
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.unwrap().get("rust").is_some());
}

// =============================================================================
// COVERAGE MERGING
// =============================================================================

/// Spec: docs/specs/11-test-runners.md#aggregation
///
/// > Coverage: Merged across suites covering the same language
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn multiple_suite_coverages_merged() {
    let temp = Project::empty();
    temp.config(r#"
# Suite 1: Unit tests (covers internal functions)
[[check.tests.suite]]
runner = "cargo"

# Suite 2: Integration tests (covers main binary)
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
targets = ["myapp"]
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[lib]
name = "myapp"
path = "src/lib.rs"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#);
    temp.file("src/lib.rs", r#"
pub fn helper() -> i32 { 42 }
pub fn other() -> i32 { 0 }
"#);
    temp.file("src/main.rs", r#"
use myapp::helper;
fn main() { println!("{}", helper()); }
"#);
    temp.file("tests/unit.rs", r#"
#[test]
fn test_other() { assert_eq!(myapp::other(), 0); }
"#);
    temp.file("tests/cli/run.bats", r#"
@test "runs main" {
    run ./target/debug/myapp
    [ "$output" = "42" ]
}
"#);

    let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
    let metrics = result.require("metrics");

    // Coverage should be merged from both suites
    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    let rust_coverage = coverage.unwrap().get("rust").and_then(|v| v.as_f64());

    // Both helper() and other() should be covered (100%)
    // because unit tests cover other() and CLI tests cover helper() via main
    assert!(rust_coverage.unwrap() > 90.0);
}

/// Spec: docs/specs/11-test-runners.md#no-coverage
///
/// > For suites that only contribute timing: targets = []
#[test]
#[ignore = "TODO: Phase 9XX - Test runners implementation"]
fn suite_with_empty_targets_skips_coverage() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests/smoke/"
targets = []  # Explicit: timing only
"#);
    temp.file("tests/smoke/basic.bats", r#"
@test "smoke test" { true; }
"#);

    let result = check("tests").pwd(temp.path()).json().passes();
    let metrics = result.require("metrics");

    // Should have timing but no coverage
    assert!(metrics.get("total_ms").is_some());
    assert!(metrics.get("coverage").is_none());
}
```

**Verification**:
```bash
cargo test --test specs -- coverage --ignored 2>&1 | grep -E "(ignored|FAILED)"
```

---

### Phase 4: Update Module Structure

**Goal**: Wire up new spec modules.

**File**: `tests/specs/checks/tests/mod.rs`

```rust
//! Behavioral specs for tests check.
//!
//! Reference: docs/specs/checks/tests.md
//! Reference: docs/specs/11-test-runners.md

mod correlation;
mod coverage;
mod output;
mod runners;
mod timing;
```

**Verification**:
```bash
cargo test --test specs -- tests:: --ignored 2>&1 | grep -c "ignored"
# Should show count of all new ignored specs
```

---

### Phase 5: Create Fixtures (Optional)

**Goal**: Create minimal fixtures for tests that need real projects.

Some specs may work better with pre-built fixtures rather than temp directories, especially for:
- Projects that need `cargo build` to succeed
- Bats tests that need actual shell execution

**Fixture structure**:
```
tests/fixtures/tests/
├── cargo-basic/
│   ├── Cargo.toml
│   ├── quench.toml
│   ├── src/lib.rs
│   └── tests/basic.rs
├── bats-basic/
│   ├── quench.toml
│   └── tests/basic.bats
└── multi-suite/
    ├── Cargo.toml
    ├── quench.toml
    ├── src/lib.rs
    ├── src/main.rs
    ├── tests/unit.rs
    └── tests/cli/run.bats
```

**Note**: This phase is optional. If temp projects are sufficient for specs, fixtures may not be needed. The implementation phase will determine if fixtures add value.

**Verification**:
```bash
# If fixtures created:
ls tests/fixtures/tests/
```

---

### Phase 6: Final Verification

**Goal**: Ensure all specs compile and are properly ignored.

**Commands**:
```bash
# Build specs (should compile without errors)
cargo build --test specs

# Run with --ignored to see count
cargo test --test specs -- tests:: --ignored 2>&1 | grep -E "(ignored|test result)"

# Verify no specs accidentally pass (they're not implemented)
cargo test --test specs -- tests::runners 2>&1 | grep -E "0 passed"
cargo test --test specs -- tests::timing 2>&1 | grep -E "0 passed"
cargo test --test specs -- tests::coverage 2>&1 | grep -E "0 passed"

# Full check
make check
```

---

## Key Implementation Details

### Spec File Organization

| File | Purpose |
|------|---------|
| `runners.rs` | Basic runner execution (cargo test, bats --timing) |
| `timing.rs` | Per-test timing extraction (avg, max, total) |
| `coverage.rs` | Coverage collection and merging |
| `output.rs` | Output format specs (existing) |
| `correlation.rs` | Test/source correlation (existing) |

### Fixture vs Temp Project Decision

**Use temp projects** (`Project::empty()`) when:
- Testing config parsing
- Testing error messages
- Simple pass/fail scenarios

**Use fixtures** when:
- Need pre-compiled binaries
- Need complex multi-file setups
- Tests are slow due to compilation

### Coverage Spec Considerations

Coverage specs may be slow (require compilation + instrumented runs). Consider:
1. Marking coverage specs as `#[ignore]` with note about CI-only
2. Using `--ci` flag in coverage specs since coverage is CI-only
3. Mocking coverage output for fast specs, real execution for integration

### JSON Output Structure

Expected metrics structure (for verification):

```json
{
  "name": "tests",
  "passed": true,
  "metrics": {
    "test_count": 5,
    "total_ms": 1234,
    "avg_ms": 246,
    "max_ms": 500,
    "max_test": "tests::integration::slow_test",
    "coverage": {
      "rust": 82.3,
      "shell": 71.2
    },
    "suites": [
      {"name": "cargo", "tests": 3, "time_ms": 800},
      {"name": "bats", "tests": 2, "time_ms": 434}
    ]
  }
}
```

---

## Verification Plan

### Per-Phase Verification

| Phase | Command | Expected |
|-------|---------|----------|
| 1 | `cargo build --test specs` | Compiles |
| 2 | `cargo build --test specs` | Compiles |
| 3 | `cargo build --test specs` | Compiles |
| 4 | `cargo test --test specs -- tests:: 2>&1 \| grep "0 passed"` | All ignored |
| 5 | `ls tests/fixtures/tests/` (if created) | Fixtures exist |
| 6 | `make check` | Full pass |

### Final Verification

```bash
# Count new specs
cargo test --test specs -- tests::runners --ignored 2>&1 | grep "ignored"
cargo test --test specs -- tests::timing --ignored 2>&1 | grep "ignored"
cargo test --test specs -- tests::coverage --ignored 2>&1 | grep "ignored"

# Ensure none accidentally pass
cargo test --test specs -- tests:: 2>&1 | grep -E "passed.*failed.*ignored"

# Full check
make check
```

### Success Criteria

1. **All specs compile**: `cargo build --test specs` exits 0
2. **All specs ignored**: No specs pass (feature not implemented)
3. **Proper ignore annotations**: Each spec has `#[ignore = "TODO: Phase 9XX - description"]`
4. **Doc comments reference spec**: Each test has `/// Spec: docs/specs/...` comment
5. **Clippy clean**: `cargo clippy` passes
6. **`make check` passes**: Full test suite green

---

## Spec Summary

| Module | Spec Count | Description |
|--------|------------|-------------|
| `runners.rs` | 4 | cargo executes, bats executes, test counts |
| `timing.rs` | 5 | avg timing, max timing with name, total time |
| `coverage.rs` | 5 | rust coverage, shell coverage, binary coverage, merge, skip |
| **Total** | **14** | New behavioral specs |

---

## Completion Criteria

- [ ] Phase 1: `runners.rs` created with cargo/bats execution specs
- [ ] Phase 2: `timing.rs` created with timing extraction specs
- [ ] Phase 3: `coverage.rs` created with coverage collection specs
- [ ] Phase 4: `mod.rs` updated to include new modules
- [ ] Phase 5: Fixtures created (if needed)
- [ ] Phase 6: `make check` passes
- [ ] All specs marked `#[ignore = "TODO: Phase 9XX"]`
- [ ] `./done` executed successfully
