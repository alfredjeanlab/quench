# Checkpoint 11C: Refactor - Tests CI Mode

## Overview

Refactor the tests CI mode test suite to reduce duplication and improve maintainability. The current implementation has ~22 instances of nearly identical Cargo project setup boilerplate spread across 5 test files. This checkpoint extracts common patterns into reusable helpers.

## Project Structure

```
quench/
├── tests/specs/
│   ├── prelude.rs                    # MODIFIED: Add CargoProject helper
│   └── checks/tests/
│       ├── mod.rs                    # MODIFIED: Remove ci_output module
│       ├── ci_metrics.rs             # MODIFIED: Consolidated CI tests
│       ├── ci_output.rs              # DELETED: Merged into ci_metrics.rs
│       ├── coverage.rs               # MODIFIED: Use CargoProject helper
│       ├── timing.rs                 # MODIFIED: Use CargoProject helper
│       └── runners.rs                # MODIFIED: Use CargoProject helper
└── tests/fixtures/
    └── tests-ci/                     # UNCHANGED: Fixture for deterministic output
```

## Dependencies

No new dependencies. Refactoring uses existing test infrastructure.

## Implementation Phases

### Phase 1: Add CargoProject Helper to Prelude

**Goal:** Extract the repeated Cargo project setup into a reusable helper.

**File:** `tests/specs/prelude.rs`

Add a builder method to `Project` that creates a minimal Rust project:

```rust
impl Project {
    /// Create a minimal Cargo project with tests check configured.
    ///
    /// Creates:
    /// - `Cargo.toml` with package name and edition
    /// - `quench.toml` with cargo test suite
    /// - `src/lib.rs` with a simple function
    /// - `tests/basic.rs` with one passing test
    ///
    /// # Example
    /// ```ignore
    /// let temp = Project::cargo("my_project");
    /// check("tests").pwd(temp.path()).passes();
    /// ```
    pub fn cargo(name: &str) -> Self {
        let temp = Self::empty();
        temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
"#);
        temp.file("Cargo.toml", &format!(r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#));
        temp.file("src/lib.rs", &format!(
            "pub fn add(a: i32, b: i32) -> i32 {{ a + b }}"
        ));
        temp.file("tests/basic.rs", &format!(r#"
#[test]
fn test_add() {{ assert_eq!({name}::add(1, 2), 3); }}
"#));
        temp
    }
}
```

**Verification:**
```bash
cargo build --test specs
```

### Phase 2: Consolidate CI Tests into Single File

**Goal:** Merge `ci_output.rs` into `ci_metrics.rs` since both test CI mode behavior.

**File:** `tests/specs/checks/tests/ci_metrics.rs`

Add a new section for output format tests at the end:

```rust
// =============================================================================
// CI OUTPUT FORMAT
// =============================================================================

/// Spec: CI mode text output shows test results summary.
///
/// > CI mode should output "PASS: tests" on success.
#[test]
fn tests_ci_text_output_passes() {
    check("tests")
        .on("tests-ci")
        .args(&["--ci"])
        .passes()
        .stdout_has("PASS: tests");
}

// ... rest of ci_output.rs tests
```

**Delete:** `tests/specs/checks/tests/ci_output.rs`

**Update:** `tests/specs/checks/tests/mod.rs` to remove `mod ci_output;`

**Verification:**
```bash
cargo test --test specs ci_metrics
# Should pass all 13 CI tests
```

### Phase 3: Refactor ci_metrics.rs to Use Helper

**Goal:** Replace boilerplate in `ci_metrics.rs` with `Project::cargo()`.

**Before (repeated 7 times):**
```rust
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
```

**After:**
```rust
let temp = Project::cargo("test_project");
```

For tests that need custom config (like thresholds), override after:
```rust
let temp = Project::cargo("test_project");
temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#);
```

**Tests to refactor:**
- `ci_mode_reports_aggregated_timing_metrics`
- `ci_mode_reports_per_suite_timing`
- `ci_mode_reports_per_package_coverage` (needs custom lib.rs)
- `coverage_below_min_generates_violation` (needs custom lib.rs)
- `per_package_coverage_thresholds_work` (needs custom lib.rs)
- `time_total_exceeded_generates_violation`
- `tests_ci_text_output_timing_violation`
- `tests_ci_json_violation_has_threshold_and_value`

**Verification:**
```bash
cargo test --test specs ci_metrics
```

### Phase 4: Refactor Other Test Files

**Goal:** Apply `Project::cargo()` helper to remaining test files.

**File:** `tests/specs/checks/tests/runners.rs`

Refactor tests:
- `cargo_runner_executes_cargo_test`
- `cargo_runner_reports_test_count`

**File:** `tests/specs/checks/tests/coverage.rs`

Refactor tests:
- `cargo_runner_collects_rust_coverage` (needs custom lib.rs)

**File:** `tests/specs/checks/tests/timing.rs`

Refactor tests (all currently ignored, but still clean up):
- `cargo_runner_extracts_average_timing`
- `cargo_runner_extracts_max_timing_with_name`
- `runner_reports_total_time`
- `runner_fails_when_test_exceeds_max_time`

**Verification:**
```bash
cargo test --test specs checks_tests
```

### Phase 5: Use tests-ci Fixture Where Possible

**Goal:** Replace temp project creation with the `tests-ci` fixture for tests that don't need custom config.

**Candidates:**
- `ci_mode_reports_aggregated_timing_metrics` → use fixture
- `ci_mode_reports_per_suite_timing` → use fixture

**Pattern:**
```rust
// Before
let temp = Project::cargo("test_project");
let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();

// After
let result = check("tests").on("tests-ci").args(&["--ci"]).json().passes();
```

**Verification:**
```bash
cargo test --test specs ci_metrics
```

### Phase 6: Verify and Commit

**Goal:** Ensure all tests pass and commit the refactoring.

**Verification:**
```bash
# Run all tests check specs
cargo test --test specs checks_tests

# Run full spec suite
cargo test --test specs
# Expected: 565 passed, 11 ignored

# Full validation
make check
```

**Files changed:**
- `tests/specs/prelude.rs` (modified)
- `tests/specs/checks/tests/mod.rs` (modified)
- `tests/specs/checks/tests/ci_metrics.rs` (modified)
- `tests/specs/checks/tests/ci_output.rs` (deleted)
- `tests/specs/checks/tests/runners.rs` (modified)
- `tests/specs/checks/tests/coverage.rs` (modified)
- `tests/specs/checks/tests/timing.rs` (modified)

**Commit message:**
```
refactor(tests): reduce CI mode test boilerplate (checkpoint 11c)

Extract common Cargo project setup into Project::cargo() helper.
Consolidate ci_output.rs into ci_metrics.rs for better organization.
Use tests-ci fixture where applicable.

Changes:
- Add Project::cargo(name) helper to prelude
- Merge ci_output.rs tests into ci_metrics.rs
- Refactor ~22 instances of boilerplate to use helper

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

## Key Implementation Details

### Project::cargo() Helper Design

The helper creates a "batteries included" Cargo project that:
1. Has a working `Cargo.toml` with the given package name
2. Includes `quench.toml` with cargo test suite configured
3. Provides `src/lib.rs` with a simple testable function
4. Includes `tests/basic.rs` with one passing test

This covers the common case. Tests needing custom setup can:
- Override `quench.toml` with `temp.config(...)`
- Override source files with `temp.file(...)`

### File Organization After Refactor

```
tests/specs/checks/tests/
├── mod.rs          # Module exports (ci_output removed)
├── ci_metrics.rs   # All CI mode tests (13 tests)
├── correlation.rs  # Test correlation (unchanged)
├── coverage.rs     # Coverage collection (refactored)
├── output.rs       # Output format (unchanged)
├── runners.rs      # Test runners (refactored)
└── timing.rs       # Timing extraction (refactored)
```

### Boilerplate Reduction

| File | Before | After | Savings |
|------|--------|-------|---------|
| ci_metrics.rs | ~140 lines setup | ~50 lines | -90 lines |
| ci_output.rs | ~60 lines | merged | -60 lines |
| runners.rs | ~50 lines setup | ~20 lines | -30 lines |
| coverage.rs | ~40 lines setup | ~15 lines | -25 lines |
| timing.rs | ~80 lines setup | ~30 lines | -50 lines |
| **Total** | ~370 lines | ~115 lines | **~255 lines** |

## Verification Plan

| Step | Command | Expected Result |
|------|---------|-----------------|
| Build compiles | `cargo build --test specs` | Success |
| CI metrics tests | `cargo test --test specs ci_metrics` | 13 passed |
| All tests checks | `cargo test --test specs checks_tests` | All passed |
| Full suite | `cargo test --test specs` | 565 passed, 11 ignored |
| Lint check | `make check` | All checks pass |

## Completion Criteria

- [ ] `Project::cargo()` helper added to prelude
- [ ] `ci_output.rs` merged into `ci_metrics.rs`
- [ ] Boilerplate replaced with helper calls
- [ ] `tests-ci` fixture used where applicable
- [ ] All specs pass (no regressions)
- [ ] `make check` passes
- [ ] Changes committed
- [ ] `./done` executed
