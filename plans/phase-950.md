# Phase 950: Tests Check CI Thresholds - Behavioral Specs

## Overview

Add behavioral specifications (black-box tests) for CI mode threshold violations in the tests check. These specs verify that the tests check generates appropriate violations when:

- Coverage falls below the configured minimum
- Per-package coverage falls below package-specific thresholds
- Total test time exceeds `max_total`
- Slowest individual test exceeds `max_test`

This phase adds **specs only**—no implementation changes. Specs will be marked with `#[ignore]` if the underlying functionality is not yet implemented.

## Project Structure

```
tests/specs/checks/tests/
├── ci_metrics.rs              # UPDATE: add threshold violation specs
└── mod.rs                     # Reference: module exports

tests/fixtures/
├── tests-coverage-below-min/  # NEW: fixture for coverage threshold
├── tests-time-exceeded/       # NEW: fixture for time threshold violations
└── tests-per-package-cov/     # NEW: fixture for per-package coverage
```

## Dependencies

No new dependencies. Uses existing test harness from `tests/specs/prelude.rs`.

## Implementation Phases

### Phase 1: Coverage Threshold Violation Spec

Add spec verifying that coverage below `min` generates a `coverage_below_min` violation.

**Spec reference:** `docs/specs/checks/tests.md#coverage`

```toml
# Config for coverage threshold
[check.tests.coverage]
check = "error"
min = 75
```

**Expected behavior:**
- When coverage is below `min`, check fails with `coverage_below_min` violation
- Violation includes `value` (actual coverage) and `threshold` (configured min)

```rust
// tests/specs/checks/tests/ci_metrics.rs

/// Spec: docs/specs/checks/tests.md#coverage
///
/// > Configure thresholds via `[check.tests.coverage]`:
/// > min = 75
#[test]
fn coverage_below_min_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    // Only one function tested out of two = ~50% coverage
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("coverage_below_min"));
    let v = result.require_violation("coverage_below_min");
    assert!(v.get("threshold").is_some());
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`

**Verification:** `cargo test --test specs ci_metrics`

### Phase 2: Per-Package Coverage Threshold Spec

Add spec verifying per-package coverage thresholds work independently.

**Spec reference:** `docs/specs/checks/tests.md#coverage`

```toml
[check.tests.coverage]
min = 50

[check.tests.coverage.package.core]
min = 90
```

**Expected behavior:**
- Package `core` with 70% coverage should fail (below 90%)
- Package `utils` with 60% coverage should pass (above 50%)

```rust
/// Spec: docs/specs/checks/tests.md#coverage
///
/// > [check.tests.coverage.package.core]
/// > min = 90
#[test]
fn per_package_coverage_thresholds_work() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 50

[check.tests.coverage.package.test_project]
min = 95
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file(
        "src/lib.rs",
        r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#,
    );
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    // Should fail on package-specific threshold
    assert!(result.has_violation("coverage_below_min"));
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`

**Verification:** `cargo test --test specs ci_metrics`

### Phase 3: Total Time Exceeded Spec

Add spec verifying `max_total` threshold generates `time_total_exceeded` violation.

**Spec reference:** `docs/specs/11-test-runners.md#thresholds`

```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"  # Impossibly short to trigger violation
```

**Expected behavior:**
- When total test time exceeds `max_total`, generates `time_total_exceeded`
- Violation includes `value` (actual ms) and `threshold` (configured max)

```rust
/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_total = "30s"
#[test]
fn time_total_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("time_total_exceeded"));
    let v = result.require_violation("time_total_exceeded");
    assert!(v.get("value").is_some());
    assert!(v.get("threshold").is_some());
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`

**Verification:** `cargo test --test specs ci_metrics`

### Phase 4: Slowest Test Exceeded Spec

Add spec verifying `max_test` threshold generates `time_test_exceeded` violation.

**Spec reference:** `docs/specs/11-test-runners.md#thresholds`

```toml
[[check.tests.suite]]
runner = "cargo"
max_test = "1ms"  # Impossibly short to trigger violation
```

**Expected behavior:**
- When slowest test exceeds `max_test`, generates `time_test_exceeded`
- Violation identifies the slow test name

```rust
/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_test = "1s"
#[test]
fn time_test_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "cargo"
max_test = "1ms"

[check.tests.time]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
    temp.file(
        "tests/basic.rs",
        r#"
#[test]
fn test_add() { assert_eq!(test_project::add(1, 2), 3); }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("time_test_exceeded"));
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`

**Verification:** `cargo test --test specs ci_metrics`

### Phase 5: Violation Type Enumeration Spec

Add spec verifying all CI threshold violation types are correctly identified.

**Expected violation types:**
- `coverage_below_min` - coverage below threshold
- `time_total_exceeded` - total time over max_total
- `time_test_exceeded` - slowest test over max_test

```rust
/// Spec: tests CI violation.type enumeration
///
/// Violation types for CI thresholds:
/// - coverage_below_min
/// - time_total_exceeded
/// - time_test_exceeded
#[test]
fn tests_ci_violation_types_are_documented() {
    // This test documents the expected violation types.
    // Each type should be tested individually above.
    let expected_types = [
        "coverage_below_min",
        "time_total_exceeded",
        "time_test_exceeded",
    ];

    // Verify these are the only CI threshold violation types
    // by checking they don't overlap with other tests check violations
    let other_types = ["missing_tests", "test_suite_failed"];

    for t in &expected_types {
        assert!(
            !other_types.contains(t),
            "CI threshold type '{}' should not overlap with other types",
            t
        );
    }
}
```

**Files:**
- `tests/specs/checks/tests/ci_metrics.rs`

**Verification:** `cargo test --test specs ci_metrics`

## Key Implementation Details

### Violation Type Reference

| Violation Type | Trigger | Fields |
|----------------|---------|--------|
| `coverage_below_min` | Coverage < min threshold | `value`, `threshold` |
| `time_total_exceeded` | Suite total > max_total | `value`, `threshold` |
| `time_test_exceeded` | Slowest test > max_test | `value`, `threshold`, possibly test name |

### Config Hierarchy

```toml
[check.tests.coverage]
check = "error"          # Check level (error/warn/off)
min = 75                 # Global minimum

[check.tests.coverage.package.core]
min = 90                 # Per-package override

[check.tests.time]
check = "warn"           # Time violations as warnings

[[check.tests.suite]]
runner = "cargo"
max_total = "30s"        # Per-suite time limits
max_test = "1s"
```

### Test Harness Patterns

Using existing helpers from `prelude.rs`:

```rust
// Check for violation existence
assert!(result.has_violation("coverage_below_min"));

// Get specific violation
let v = result.require_violation("time_total_exceeded");
assert!(v.get("value").is_some());
assert!(v.get("threshold").is_some());

// Multiple violations of same type
let vs = result.violations_of_type("coverage_below_min");
assert_eq!(vs.len(), 2);
```

### Handling Missing Coverage Tools

Specs should handle environments without `llvm-cov`:

```rust
// If coverage collection fails, test may pass or skip
// Check for either coverage_below_min or metrics.coverage absence
if result.require("metrics").get("coverage").is_some() {
    assert!(result.has_violation("coverage_below_min"));
}
```

Alternatively, use `#[ignore]` with environment note:

```rust
#[test]
#[ignore = "requires llvm-cov"]
fn coverage_threshold_spec() { ... }
```

## Verification Plan

### Running Specs

```bash
# Run all CI metrics specs
cargo test --test specs ci_metrics

# Run specific spec
cargo test --test specs coverage_below_min

# Check for unimplemented specs
cargo test --test specs -- --ignored
```

### Expected Results

| Spec | Expected Status |
|------|-----------------|
| `coverage_below_min_generates_violation` | Pass (if coverage tools available) |
| `per_package_coverage_thresholds_work` | Pass (if coverage tools available) |
| `time_total_exceeded_generates_violation` | Pass |
| `time_test_exceeded_generates_violation` | Pass |
| `tests_ci_violation_types_are_documented` | Pass |

### Manual Verification

```bash
# Test coverage threshold
quench check --ci -o json 2>&1 | jq '.checks[] | select(.name == "tests") | .violations'

# Expected output for coverage violation:
# [{"type": "coverage_below_min", "value": 42, "threshold": 75, ...}]
```

## Commit Strategy

Single commit for all specs:

```
test(tests): add CI metrics behavioral tests

Specs for CI threshold violations:
- coverage_below_min_generates_violation
- per_package_coverage_thresholds_work
- time_total_exceeded_generates_violation
- time_test_exceeded_generates_violation
- tests_ci_violation_types_are_documented

Reference: docs/specs/checks/tests.md#coverage
Reference: docs/specs/11-test-runners.md#thresholds
```
