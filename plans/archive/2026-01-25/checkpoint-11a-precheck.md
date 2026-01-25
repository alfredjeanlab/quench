# Checkpoint 11A: Pre-Checkpoint Fix - Tests CI Mode Complete

## Overview

Verification checkpoint to confirm that CI mode threshold checking for the tests check is complete and fully functional. This follows Phase 955 which implemented coverage and timing threshold violations in CI mode. The checkpoint validates that all specs pass and documents the completion state.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/tests/
│   │   └── mod.rs               # CI threshold checking logic (verified)
│   ├── config/
│   │   └── tests_check.rs       # TestsCoverageConfig, TestsTimeConfig (verified)
│   └── cache.rs                 # CACHE_VERSION = 26 (verified)
└── tests/specs/
    └── checks/tests/
        ├── ci_metrics.rs        # CI threshold specs (all passing)
        ├── coverage.rs          # Runner integration specs (ignored - Phase 940)
        └── timing.rs            # Runner timing specs (ignored - Phase 9XX)
```

## Dependencies

No new dependencies. Verification-only checkpoint using existing tooling:
- `cargo test --test specs` for behavioral specs
- `make check` for full validation suite

## Implementation Phases

### Phase 1: Verify CI Metrics Specs Pass

**Goal:** Confirm all 9 CI metrics specs are passing.

**Verification:**
```bash
cargo test --test specs ci_metrics
```

**Expected results:**
| Spec | Status |
|------|--------|
| `ci_mode_reports_aggregated_timing_metrics` | Pass |
| `ci_mode_reports_per_suite_timing` | Pass |
| `ci_mode_reports_per_package_coverage` | Pass |
| `coverage_below_min_generates_violation` | Pass |
| `per_package_coverage_thresholds_work` | Pass |
| `time_total_exceeded_generates_violation` | Pass |
| `time_avg_exceeded_generates_violation` | Pass |
| `time_test_exceeded_generates_violation` | Pass |
| `tests_ci_violation_types_are_documented` | Pass |

### Phase 2: Verify Full Test Suite

**Goal:** Confirm no regressions in the full spec suite.

**Verification:**
```bash
cargo test --test specs
```

**Expected:** 561 passed, 11 ignored, 0 failed

**Ignored tests (expected for future phases):**
- `cli_ci_mode::ci_mode_enables_build_check` - needs build artifact fixture
- `cli_ci_mode::ci_mode_enables_license_check` - needs license file fixture
- `checks_tests::timing::*` (5 tests) - Phase 9XX test runner timing extraction
- `checks_tests::coverage::*` (4 tests) - Phase 940 runner integration

### Phase 3: Run Full Check Suite

**Goal:** Verify `make check` passes completely.

**Verification:**
```bash
make check
```

**Expected checks:**
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test --all`
- [x] `cargo build --all`
- [x] `cargo audit`
- [x] `cargo deny check`

### Phase 4: Verify Threshold Functionality

**Goal:** Manual verification that CI thresholds work end-to-end.

**Test coverage thresholds:**
```bash
# Create temp project with low coverage
mkdir -p /tmp/threshold-test && cd /tmp/threshold-test
cat > quench.toml << 'EOF'
[[check.tests.suite]]
runner = "cargo"

[check.tests.coverage]
check = "error"
min = 95
EOF
cat > Cargo.toml << 'EOF'
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
EOF
mkdir src && echo 'pub fn covered() -> i32 { 42 }
pub fn uncovered() -> i32 { 0 }' > src/lib.rs
mkdir tests && echo '#[test]
fn test_covered() { assert_eq!(test_project::covered(), 42); }' > tests/basic.rs

# Run with --ci to trigger threshold check
quench check tests --ci -o json 2>&1 | jq '.checks[0].violations[] | select(.type == "coverage_below_min")'
```

**Expected:** Violation with type `coverage_below_min`

**Test timing thresholds:**
```bash
# Same project, add timing threshold
cat >> quench.toml << 'EOF'

[check.tests.time]
check = "error"
EOF
sed -i '' 's/max_total.*/max_total = "1ms"/' quench.toml || echo '
[[check.tests.suite]]
runner = "cargo"
max_total = "1ms"' >> quench.toml

quench check tests --ci -o json 2>&1 | jq '.checks[0].violations[] | select(.type == "time_total_exceeded")'
```

**Expected:** Violation with type `time_total_exceeded`

### Phase 5: Document Completion

**Goal:** Update plan with verification results and archive.

After all phases verified:
1. Confirm implementation matches Phase 955 plan
2. Note any deviations or issues found
3. Archive the plan

## Key Implementation Details

### Violation Types (from Phase 955)

| Violation Type | Trigger | Config Location |
|---------------|---------|-----------------|
| `coverage_below_min` | Coverage below threshold | `[check.tests.coverage].min` or `[check.tests.coverage.package.<name>].min` |
| `time_total_exceeded` | Suite runtime exceeds limit | `[[check.tests.suite]].max_total` |
| `time_avg_exceeded` | Average test time exceeds limit | `[[check.tests.suite]].max_avg` |
| `time_test_exceeded` | Slowest test exceeds limit | `[[check.tests.suite]].max_test` |

### Check Level Configuration

```toml
[check.tests.coverage]
check = "error"  # error | warn | off (default: off)
min = 75

[check.tests.time]
check = "warn"   # error | warn | off (default: off)
```

### CI Mode Behavior

- Thresholds only checked when `--ci` flag is present
- Coverage collection requires appropriate tooling (llvm-cov for Rust)
- Timing extraction varies by runner (bats provides per-test timing)

## Verification Plan

### Automated Verification

```bash
# 1. CI metrics specs
cargo test --test specs ci_metrics
# Expected: 9 passed

# 2. Full spec suite
cargo test --test specs
# Expected: 561 passed, 11 ignored

# 3. Full check suite
make check
# Expected: All checks pass

# 4. Quick functional test
cd /tmp && rm -rf threshold-test && mkdir threshold-test && cd threshold-test
# (run test commands from Phase 4)
```

### Completion Criteria

- [ ] All 9 CI metrics specs pass
- [ ] Full spec suite: 561 passed, 11 ignored, 0 failed
- [ ] `make check` passes completely
- [ ] Manual threshold verification produces expected violations
- [ ] No regressions in existing functionality

## Commit Strategy

No code changes expected. If verification passes, archive the plan:

```
chore(tests): verify CI mode threshold implementation complete

Verification checkpoint for Phase 955:
- All 9 CI metrics specs passing
- Coverage threshold violations working (coverage_below_min)
- Timing threshold violations working (time_*_exceeded)
- Check levels (error/warn/off) functioning correctly

Specs verified:
- ci_mode_reports_aggregated_timing_metrics
- ci_mode_reports_per_suite_timing
- ci_mode_reports_per_package_coverage
- coverage_below_min_generates_violation
- per_package_coverage_thresholds_work
- time_total_exceeded_generates_violation
- time_avg_exceeded_generates_violation
- time_test_exceeded_generates_violation
- tests_ci_violation_types_are_documented
```
