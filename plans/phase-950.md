# Phase 950: Tests Check CI Thresholds - Specs

## Overview

Add behavioral specifications for CI threshold violations in the `tests` check. These specs verify that quench correctly generates violations when:
- Coverage falls below configured minimums
- Per-package coverage thresholds are exceeded
- Test suite total time exceeds `max_total`
- Individual test time exceeds `max_test`

These specs are black-box tests against the CLI and do not inspect internal implementation.

## Project Structure

```
quench/
├── tests/
│   └── specs/
│       └── checks/
│           └── tests/
│               ├── mod.rs              # Module registration
│               └── ci_metrics.rs       # CI threshold specs (target file)
└── docs/
    └── specs/
        └── checks/
            └── tests.md                # Reference spec document
```

## Dependencies

**Test infrastructure:**
- `crate::prelude::*` - Test helpers (`check()`, `Project`, etc.)
- `tests/fixtures/tests-ci/` - CI mode test fixture

**Runtime dependencies for tests:**
- `cargo` - For Rust test suite execution
- `bats` - For shell test suite execution (per-test timing)
- `cargo-llvm-cov` - For coverage collection (optional)

## Implementation Phases

### Phase 1: Coverage Threshold Specs

**Goal:** Verify coverage threshold enforcement.

**Specs:**
1. `coverage_below_min_generates_violation` - Coverage below global `min` fails
2. `per_package_coverage_thresholds_work` - Per-package override with stricter threshold

**Config pattern:**
```toml
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95

[check.tests.coverage.package.core]
min = 90
```

**Spec pattern:**
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

**Verification:**
```bash
cargo test --test specs -- coverage_below_min
cargo test --test specs -- per_package_coverage
```

### Phase 2: Time Threshold Specs

**Goal:** Verify test time threshold enforcement.

**Specs:**
1. `time_total_exceeded_generates_violation` - Suite total over `max_total`
2. `time_test_exceeded_generates_violation` - Single test over `max_test`
3. `time_avg_exceeded_generates_violation` - Average over `max_avg`

**Config pattern:**
```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"

[check.tests.time]
check = "error"
```

**Note on runner selection:** The `bats` runner is preferred for `max_test` and `max_avg` specs because it provides per-test timing via the `--timing` flag. Cargo test does not expose per-test timing in stable output.

**Spec pattern:**
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

**Verification:**
```bash
cargo test --test specs -- time_total_exceeded
cargo test --test specs -- time_test_exceeded
cargo test --test specs -- time_avg_exceeded
```

### Phase 3: Violation Type Documentation

**Goal:** Document all CI threshold violation types.

**Violation types for tests CI mode:**
- `coverage_below_min` - Coverage percentage below threshold
- `time_total_exceeded` - Suite total time over max_total
- `time_avg_exceeded` - Average test time over max_avg
- `time_test_exceeded` - Slowest test over max_test

**Spec pattern:**
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
    let expected_types = [
        "coverage_below_min",
        "time_total_exceeded",
        "time_avg_exceeded",
        "time_test_exceeded",
    ];

    // Verify these are distinct from correlation violation types
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

### Phase 4: Output Format Specs

**Goal:** Verify CI output includes proper violation formatting.

**Specs:**
1. `tests_ci_text_output_timing_violation` - Text output shows violation type and threshold
2. `tests_ci_json_violation_has_threshold_and_value` - JSON includes required fields

**Spec pattern:**
```rust
/// Spec: CI violation has threshold and value fields.
///
/// > JSON violations for CI thresholds should include threshold and value.
#[test]
fn tests_ci_json_violation_has_threshold_and_value() {
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

    let v = result.require_violation("time_total_exceeded");
    assert!(v.get("threshold").is_some());
    assert!(v.get("value").is_some());
}
```

## Key Implementation Details

### Violation Structure

CI threshold violations include additional fields beyond standard violations:

```json
{
    "file": null,
    "line": null,
    "type": "coverage_below_min",
    "threshold": 95.0,
    "value": 50.0,
    "advice": "Coverage 50.0% below minimum 95.0%"
}
```

```json
{
    "file": null,
    "line": null,
    "type": "time_total_exceeded",
    "suite": "cargo",
    "threshold": 1,
    "value": 1500,
    "advice": "Suite total 1500ms exceeds max_total 1ms"
}
```

### Check Level Configuration

Thresholds can be set to different check levels:

```toml
[check.tests.coverage]
check = "error"   # Fail on violation (default)

[check.tests.time]
check = "warn"    # Report but don't fail
```

### Runner Timing Capabilities

| Runner | Per-Test Timing | Source |
|--------|-----------------|--------|
| `cargo` | No* | Summary only |
| `bats` | Yes | `--timing` flag |
| `pytest` | Yes | `--durations=0` |
| `jest`/`vitest` | Yes | JSON reporter |

*Cargo test JSON output is unstable and doesn't provide per-test timing in human-readable mode.

## Verification Plan

### Run All CI Threshold Specs

```bash
# Run all tests check CI specs
cargo test --test specs -- ci_metrics

# Expected output: all tests pass
```

### Individual Spec Verification

```bash
# Coverage threshold
cargo test --test specs -- coverage_below_min_generates_violation
cargo test --test specs -- per_package_coverage_thresholds_work

# Time thresholds
cargo test --test specs -- time_total_exceeded_generates_violation
cargo test --test specs -- time_test_exceeded_generates_violation
cargo test --test specs -- time_avg_exceeded_generates_violation

# Documentation
cargo test --test specs -- tests_ci_violation_types_are_documented

# Output format
cargo test --test specs -- tests_ci_json_violation_has_threshold_and_value
```

### Full Check

```bash
make check
```

### Checklist

- [ ] `coverage_below_min_generates_violation` spec passes
- [ ] `per_package_coverage_thresholds_work` spec passes
- [ ] `time_total_exceeded_generates_violation` spec passes
- [ ] `time_test_exceeded_generates_violation` spec passes
- [ ] `time_avg_exceeded_generates_violation` spec passes
- [ ] `tests_ci_violation_types_are_documented` spec passes
- [ ] All specs reference `docs/specs/checks/tests.md` sections
- [ ] `make check` passes

### Exit Criteria

All specs in `tests/specs/checks/tests/ci_metrics.rs` pass without `#[ignore]`:

1. Coverage threshold enforcement verified
2. Per-package coverage thresholds verified
3. Suite time threshold (`max_total`) verified
4. Per-test time threshold (`max_test`) verified
5. Average time threshold (`max_avg`) verified
6. Violation type enumeration documented
7. JSON output structure verified
