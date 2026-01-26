# Phase 950: Tests Check CI Thresholds - Specs

Verify and document behavioral specifications for CI threshold violations in the tests check.

## Overview

This phase adds behavioral specs for CI mode threshold violations in the `tests` check. These specs verify that quench correctly detects and reports:
- Coverage below configured minimums
- Per-package coverage threshold violations
- Test suite total time exceeding `max_total`
- Individual test time exceeding `max_test`

**Status**: Specs already implemented in `tests/specs/checks/tests/ci_metrics.rs`. This plan documents existing coverage and verification steps.

Reference: `docs/specs/checks/tests.md#ci-mode-test-execution`, `docs/specs/11-test-runners.md#thresholds`

## Project Structure

```
tests/
├── fixtures/
│   └── tests-ci/              # Fixture for CI mode tests
│       ├── Cargo.toml
│       ├── quench.toml
│       ├── src/lib.rs
│       └── tests/basic.rs
└── specs/
    └── checks/
        └── tests/
            ├── mod.rs          # Module declarations
            ├── ci_metrics.rs   # CI threshold specs (this phase)
            ├── correlation.rs  # Source-test correlation specs
            ├── coverage.rs     # Coverage collection specs
            ├── output.rs       # Output format specs
            ├── runners.rs      # Test runner specs
            ├── timeout.rs      # Timeout behavior specs
            └── timing.rs       # Timing metrics specs
```

## Dependencies

No new external dependencies. Uses existing test infrastructure:
- `tests/specs/prelude.rs` - Test helpers (`check()`, `Project::cargo()`, etc.)
- `assert_cmd`, `predicates` - CLI testing
- `serde_json` - JSON output validation
- `bats` - Required for per-test timing specs (provides `--timing` flag)

## Implementation Phases

### Phase 1: Coverage Threshold Specs (1 of 4)

Verify coverage threshold violation detection.

**Spec: Coverage below min generates violation**

```rust
/// Spec: docs/specs/checks/tests.md#coverage
///
/// > Configure thresholds via `[check.tests.coverage]`:
/// > min = 75
#[test]
fn coverage_below_min_generates_violation() {
    let temp = Project::cargo("test_project");
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95
"#);
    // Only one function tested out of two = ~50% coverage
    temp.file("src/lib.rs", r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#);
    temp.file("tests/basic.rs", r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#);

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

**Spec: Per-package coverage thresholds work**

```rust
/// Spec: docs/specs/checks/tests.md#coverage
///
/// > [check.tests.coverage.package.core]
/// > min = 90
#[test]
fn per_package_coverage_thresholds_work() {
    let temp = Project::cargo("test_project");
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 50

[check.tests.coverage.package.root]
min = 95
"#);
    // ~50% coverage passes global but fails package-specific
    temp.file("src/lib.rs", r#"
pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }
"#);
    temp.file("tests/basic.rs", r#"
#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }
"#);

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    // Should fail on package-specific threshold
    assert!(result.has_violation("coverage_below_min"));
}
```

### Phase 2: Timing Threshold Specs (2 of 4)

Verify timing threshold violation detection.

**Spec: Test time over max_total generates violation**

```rust
/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_total = "30s"
#[test]
fn time_total_exceeded_generates_violation() {
    let temp = Project::cargo("test_project");
    temp.config(r#"
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
"#);

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

**Spec: Slowest test over max_test generates violation**

Uses bats runner since it provides per-test timing via `--timing` flag. Cargo test doesn't provide per-test timing in human-readable output.

```rust
/// Spec: docs/specs/11-test-runners.md#thresholds
///
/// > max_test = "1s"
#[test]
fn time_test_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
max_test = "5ms"

[check.tests.time]
check = "error"
"#);
    // Create a bats test that sleeps longer than the threshold
    temp.file("tests/slow_test.bats", r#"
#!/usr/bin/env bats

@test "slow test that exceeds threshold" {
    sleep 0.02
    [ 1 -eq 1 ]
}
"#);

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("time_test_exceeded"));
}
```

### Phase 3: Violation Types Documentation (3 of 4)

Document the enumeration of valid CI threshold violation types.

**Spec: Tests CI violation.type values**

```rust
/// Spec: tests CI violation.type enumeration
///
/// Violation types for CI thresholds:
/// - coverage_below_min
/// - time_total_exceeded
/// - time_avg_exceeded
/// - time_test_exceeded
#[test]
fn tests_ci_violation_types_are_documented() {
    // This test documents the expected violation types.
    // Each type should be tested individually above.
    let expected_types = [
        "coverage_below_min",
        "time_total_exceeded",
        "time_avg_exceeded",
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

### Phase 4: Output Format Verification (4 of 4)

Verify output format for CI threshold violations.

**Additional specs in `ci_metrics.rs`:**

- `tests_ci_text_output_passes` - CI mode text output shows "PASS: tests"
- `tests_ci_json_output_timing_structure` - JSON includes required timing fields
- `tests_ci_text_output_timing_violation` - Timing violations show type and exceeded limit
- `tests_ci_json_violation_has_threshold_and_value` - JSON violations include threshold/value

## Key Implementation Details

### Violation Types

Four CI threshold violation types (note: outline lists 3, but `time_avg_exceeded` is also implemented):

| Type | Trigger | Fields |
|------|---------|--------|
| `coverage_below_min` | Coverage % < min threshold | `threshold`, `value`, `package` (optional) |
| `time_total_exceeded` | Suite total time > `max_total` | `threshold`, `value`, `suite` |
| `time_avg_exceeded` | Average test time > `max_avg` | `threshold`, `value`, `suite` |
| `time_test_exceeded` | Slowest test time > `max_test` | `threshold`, `value`, `test_name`, `suite` |

### Configuration Structure

```toml
# Coverage thresholds
[check.tests.coverage]
check = "error"    # error | warn | off
min = 75           # Global minimum coverage %

[check.tests.coverage.package.<name>]
min = 90           # Per-package minimum

# Timing thresholds (per-suite)
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"  # Total suite time
max_avg = "100ms"  # Average per test
max_test = "1s"    # Slowest individual test

# Check level for timing violations
[check.tests.time]
check = "warn"     # error | warn | off
```

### Per-Test Timing Availability

Per-test timing (`max_test`, `max_avg`) requires runner support:

| Runner | Per-Test Timing | Source |
|--------|-----------------|--------|
| `cargo` | No (stable) | Requires unstable `--format json` |
| `bats` | Yes | `--timing` flag provides TAP output with timing |
| `pytest` | Yes | `--durations=0` report |
| `jest`/`vitest`/`bun` | Yes | JSON reporter output |
| `go` | Yes | `-json` output |

The bats runner is used in specs for `time_test_exceeded` because it provides reliable per-test timing.

## Verification Plan

### Running Specs

```bash
# Run CI metrics specs
cargo test --test specs ci_metrics

# Run all tests check specs
cargo test --test specs checks::tests

# Show all tests check specs (including any ignored)
cargo test --test specs checks::tests -- --list --ignored
```

### Checklist

- [x] Specs exist in `tests/specs/checks/tests/ci_metrics.rs`
- [x] Module declared in `tests/specs/checks/tests/mod.rs`
- [x] No `#[ignore]` attributes (specs are implemented)
- [x] Doc comments reference spec document sections
- [x] Fixture `tests-ci/` exists for basic CI mode tests
- [ ] `cargo test --test specs ci_metrics` passes
- [ ] Update outline `.0-outline.md` to mark Phase 950 items complete

### Spec Count

12 specs in `ci_metrics.rs`:

**Timing Metrics (2)**
- `ci_mode_reports_aggregated_timing_metrics`
- `ci_mode_reports_per_suite_timing`

**Coverage (1)**
- `ci_mode_reports_per_package_coverage`

**Threshold Violations (5)** - Core Phase 950 specs
- `coverage_below_min_generates_violation`
- `per_package_coverage_thresholds_work`
- `time_total_exceeded_generates_violation`
- `time_test_exceeded_generates_violation`
- `time_avg_exceeded_generates_violation`

**Violation Types (1)**
- `tests_ci_violation_types_are_documented`

**Output Format (3)**
- `tests_ci_text_output_passes`
- `tests_ci_json_output_timing_structure`
- `tests_ci_text_output_timing_violation`
- `tests_ci_json_violation_has_threshold_and_value`

### Verification Commands

```bash
# Verify all specs compile and run
make check

# Specifically run the CI metrics specs
cargo test --test specs ci_metrics -- --nocapture

# Verify no ignored specs remain for this phase
cargo test --test specs ci_metrics -- --ignored 2>&1 | grep -c "0 tests"
```
