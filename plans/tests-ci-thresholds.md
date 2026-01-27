# Tests Check CI Mode Coverage and Time Thresholds (Phases 950-955)

## Overview

Implement behavioral specs and complete the implementation for tests check CI mode thresholds. The core threshold checking logic exists but lacks comprehensive behavioral specs and may have gaps in coverage aggregation.

**Current state**: Implementation exists in `checks/tests/mod.rs`:
- Coverage thresholds: lines 586-637 (`check_coverage_thresholds`)
- Time thresholds: lines 640-715 (`check_time_thresholds`)

**Gap**: Missing behavioral specs that test these thresholds end-to-end.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/tests/
│   │   ├── mod.rs           # Threshold checking logic (exists)
│   │   └── runners/mod.rs   # Suite execution (exists)
│   └── config/
│       └── test_config.rs   # Config structs (exists)
├── tests/
│   ├── specs/checks/tests/
│   │   ├── mod.rs           # Module exports
│   │   ├── coverage.rs      # Coverage collection specs (partial)
│   │   ├── timing.rs        # Timing extraction specs (partial)
│   │   └── thresholds.rs    # NEW: Threshold violation specs
│   └── fixtures/
│       ├── tests-ci/                  # Existing CI fixture
│       ├── tests-coverage-below/      # NEW: Coverage below min
│       ├── tests-coverage-package/    # NEW: Per-package coverage
│       └── tests-time-exceeded/       # NEW: Time thresholds
└── docs/specs/checks/tests.md         # Spec documentation (exists)
```

## Dependencies

No new external dependencies required. Uses existing:
- `tempfile` - For dynamic test fixtures
- `serde_json` - For JSON output validation
- `assert_cmd` - For CLI testing

## Implementation Phases

### Phase 950: Threshold Violation Specs

Add behavioral specs for threshold violations in `tests/specs/checks/tests/thresholds.rs`.

#### Tasks

1. **Create thresholds.rs spec file**
   ```rust
   //! Behavioral specs for CI mode threshold violations.
   //!
   //! Reference: docs/specs/checks/tests.md#coverage, #test-time

   use crate::prelude::*;
   ```

2. **Spec: Coverage below min generates violation**
   ```rust
   /// Spec: docs/specs/checks/tests.md#coverage
   ///
   /// > coverage.min = 75 generates violation when coverage < 75%
   #[test]
   fn coverage_below_min_generates_violation() {
       let temp = Project::cargo("test_project");
       temp.config(r#"
   [[check.tests.suite]]
   runner = "cargo"

   [check.tests.coverage]
   check = "error"
   min = 90
   "#);
       // lib.rs with function, tests only cover half
       temp.file("src/lib.rs", r#"
   pub fn covered() -> i32 { 42 }
   pub fn uncovered() -> i32 { 0 }
   "#);
       temp.file("tests/basic.rs", r#"
   #[test]
   fn test_covered() { assert_eq!(test_project::covered(), 42); }
   "#);

       let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();
       assert!(result.has_violation("coverage_below_min"));
   }
   ```

3. **Spec: Time total exceeded generates violation**
   ```rust
   /// Spec: docs/specs/checks/tests.md#test-time
   ///
   /// > max_total = "100ms" generates violation when suite exceeds
   #[test]
   fn time_total_exceeded_generates_violation() {
       let temp = Project::cargo("test_project");
       temp.config(r#"
   [[check.tests.suite]]
   runner = "cargo"
   max_total = "10ms"  # Very low threshold

   [check.tests.time]
   check = "error"
   "#);
       temp.file("src/lib.rs", "pub fn f() {}");
       temp.file("tests/slow.rs", r#"
   use std::thread::sleep;
   use std::time::Duration;
   #[test]
   fn slow_test() { sleep(Duration::from_millis(50)); }
   "#);

       let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();
       assert!(result.has_violation("time_total_exceeded"));
   }
   ```

4. **Spec: Violation types are correct**
   ```rust
   /// Spec: docs/specs/checks/tests.md#json-output
   ///
   /// > violation.type is one of: coverage_below_min, time_total_exceeded,
   /// > time_avg_exceeded, time_test_exceeded
   #[test]
   fn violation_types_are_correct() {
       // Test each violation type exists in output
   }
   ```

5. **Update mod.rs to include thresholds module**
   ```rust
   mod thresholds;
   ```

### Phase 951: Per-Package Coverage Specs

Add specs for per-package coverage thresholds.

#### Tasks

1. **Spec: Per-package coverage thresholds work**
   ```rust
   /// Spec: docs/specs/checks/tests.md#per-package-coverage
   ///
   /// > [check.tests.coverage.package.core]
   /// > min = 90
   #[test]
   fn per_package_coverage_threshold() {
       let temp = Project::empty();
       temp.config(r#"
   [[check.tests.suite]]
   runner = "cargo"

   [check.tests.coverage]
   check = "error"

   [check.tests.coverage.package.core]
   min = 95
   "#);
       // Create workspace with core package at low coverage
       // ...
   }
   ```

2. **Spec: Package coverage below min includes package name**
   ```rust
   #[test]
   fn package_coverage_violation_includes_name() {
       // Verify violation references the package name
   }
   ```

### Phase 952: Time Threshold Specs

Add specs for all time threshold types.

#### Tasks

1. **Spec: max_avg threshold**
   ```rust
   /// Spec: docs/specs/checks/tests.md#test-time
   ///
   /// > max_avg = "50ms" - Average time per test
   #[test]
   fn max_avg_exceeded_generates_violation() {
       let temp = Project::cargo("test_project");
       temp.config(r#"
   [[check.tests.suite]]
   runner = "cargo"
   max_avg = "5ms"

   [check.tests.time]
   check = "error"
   "#);
       // Multiple slow tests
   }
   ```

2. **Spec: max_test threshold (slowest individual)**
   ```rust
   /// Spec: docs/specs/checks/tests.md#test-time
   ///
   /// > max_test = "500ms" - Slowest individual test
   #[test]
   fn max_test_exceeded_generates_violation() {
       // One test exceeds individual max
   }
   ```

3. **Spec: Time violation includes test name**
   ```rust
   #[test]
   fn time_test_violation_includes_test_name() {
       // Verify the slowest test name appears in violation
   }
   ```

### Phase 953: Check Level Behavior Specs

Add specs for error/warn/off behavior.

#### Tasks

1. **Spec: coverage.check = "warn" reports but passes**
   ```rust
   #[test]
   fn coverage_warn_level_reports_but_passes() {
       let temp = Project::cargo("test_project");
       temp.config(r#"
   [check.tests.coverage]
   check = "warn"
   min = 99
   "#);
       // Low coverage but should pass with warning
       let result = check("tests").pwd(temp.path()).args(&["--ci"]).json().passes();
       assert!(result.has_violation("coverage_below_min"));
   }
   ```

2. **Spec: coverage.check = "off" skips checking**
   ```rust
   #[test]
   fn coverage_off_skips_threshold_checking() {
       // Even with min = 99 and low coverage, no violation
   }
   ```

3. **Spec: time.check = "warn" reports but passes**
   ```rust
   #[test]
   fn time_warn_level_reports_but_passes() {
       // Time exceeded but should pass with warning
   }
   ```

4. **Spec: time.check = "off" skips checking**
   ```rust
   #[test]
   fn time_off_skips_threshold_checking() {
       // Time exceeded but no violation generated
   }
   ```

### Phase 954: Implementation Gaps

Fill any implementation gaps discovered during spec writing.

#### Potential Tasks

1. **Coverage aggregation by language**
   - Verify `coverage: { rust: 82.3, python: 71.2 }` structure
   - Check `checks/tests/mod.rs` aggregates correctly

2. **Per-package coverage breakdown**
   - Check config parsing for `[check.tests.coverage.package.X]`
   - Verify package metrics collection from llvm-cov

3. **Suite name in time violations**
   - Ensure `<suite:cargo>` format is correct
   - Verify multi-suite scenarios

### Phase 955: Exact Output Tests

Add exact output comparison tests for CI threshold violations.

#### Tasks

1. **Create threshold violation fixture**
   - `tests/fixtures/tests-threshold-violations/`
   - Configured to generate predictable violations

2. **Exact output spec for coverage violation**
   ```rust
   #[test]
   fn coverage_violation_output_format() {
       cli().on("tests-threshold-violations")
           .args(&["--ci", "--tests"])
           .exits(1)
           .stdout_eq(
   "tests: FAIL
     <coverage:rust>: coverage_below_min (actual: 50 vs min: 90)
       Coverage 50.0% below minimum 90.0%

   FAIL: tests
   ");
   }
   ```

3. **Exact output spec for time violation**
   ```rust
   #[test]
   fn time_violation_output_format() {
       // Exact format for time_total_exceeded
   }
   ```

4. **JSON output structure test**
   ```rust
   #[test]
   fn threshold_violations_json_structure() {
       // Verify JSON includes threshold field with actual/min values
   }
   ```

## Key Implementation Details

### Violation Structure

Threshold violations include a `threshold` field for ratcheting:

```rust
Violation::file_only(
    format!("<coverage:{}>", lang),
    "coverage_below_min",
    advice,
)
.with_threshold(actual as i64, min as i64);
```

JSON output:
```json
{
  "file": "<coverage:rust>",
  "type": "coverage_below_min",
  "advice": "Coverage 50.0% below minimum 90.0%",
  "threshold": { "actual": 50, "min": 90 }
}
```

### Check Level Flow

```
check = "error" -> violation causes exit(1)
check = "warn"  -> violation in output, exit(0)
check = "off"   -> no threshold checking performed
```

Implemented in `check_coverage_thresholds`:
```rust
let is_error = config.check == "error";
// ...
violations.push((v, is_error));  // Second tuple element
```

### Test Fixture Pattern

Use `Project::cargo()` for dynamic test projects:
```rust
let temp = Project::cargo("test_project");
temp.config(r#"
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
min = 90
"#);
temp.file("src/lib.rs", "pub fn f() -> i32 { 42 }");
// Only 50% coverage
temp.file("tests/t.rs", "#[test] fn t() {}");

check("tests").pwd(temp.path()).args(&["--ci"]).json().fails();
```

## Verification Plan

### Unit Tests
- Coverage threshold logic in `checks/tests/mod.rs`
- Time threshold logic with mock suite results

### Behavioral Specs
- `cargo test --test specs -- thresholds` - All threshold specs
- `cargo test --test specs -- coverage` - Coverage specs
- `cargo test --test specs -- timing` - Timing specs

### Integration
- `quench check --ci --tests` on tests-ci fixture
- Manual verification of output format

### Checklist

- [ ] `thresholds.rs` spec file created and passing
- [ ] All Phase 950 outline items have specs
- [ ] Coverage below min generates `coverage_below_min` violation
- [ ] Per-package coverage thresholds work
- [ ] Time over `max_total` generates `time_total_exceeded` violation
- [ ] Time over `max_test` generates `time_test_exceeded` violation
- [ ] Check level error/warn/off behavior verified
- [ ] Exact output tests added
- [ ] `make check` passes
