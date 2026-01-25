# Checkpoint 11H: Tech Debt - Tests CI Mode

## Overview

Address accumulated tech debt in the tests CI mode feature. This includes enabling ignored timing specs that are now implementable, removing dead code, and improving test coverage. The timeout support added in checkpoint 11G exposed several specs that were marked as "future work" but are now achievable.

**Follows:** checkpoint-11g-bugfix (Timeout Support)

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── mod.rs                      # MODIFY: Remove dead code
│   ├── runners/
│   │   ├── mod.rs                  # MODIFY: Remove dead code utilities
│   │   ├── cargo.rs                # MODIFY: Add per-test timing (optional)
│   │   ├── bats.rs                 # No changes (timing already works)
│   │   ├── coverage.rs             # MODIFY: Remove unused struct fields
│   │   ├── instrumented.rs         # MODIFY: Remove/use cleanup_coverage_profiles
│   │   ├── jest.rs                 # MODIFY: Remove unused struct fields
│   │   ├── kcov.rs                 # MODIFY: Remove/use cleanup_kcov_output
│   │   └── vitest.rs               # MODIFY: Remove unused struct fields
├── tests/specs/checks/tests/
│   ├── timing.rs                   # MODIFY: Enable passing specs
│   ├── coverage.rs                 # No changes (Phase 940)
│   └── ci_metrics.rs               # No changes (working)
└── tests/fixtures/                 # No changes needed
```

## Dependencies

No new dependencies. Uses existing:
- `cargo test --release` for test execution
- Existing test runner framework in `runners/mod.rs`

## Implementation Phases

### Phase 1: Audit and Enable Passing Timing Specs

**Goal:** Identify which ignored timing specs already pass and enable them.

The following specs are marked `#[ignore = "TODO: Phase 9XX"]` but may already work:

| Spec | File:Line | Status | Reason |
|------|-----------|--------|--------|
| `runner_reports_total_time` | timing.rs:71 | Likely passes | `total_ms` is already in metrics |
| `bats_runner_extracts_per_test_timing` | timing.rs:92 | Likely passes | bats runner has timing extraction |
| `cargo_runner_extracts_average_timing` | timing.rs:17 | Blocked | No per-test timing from cargo |
| `cargo_runner_extracts_max_timing_with_name` | timing.rs:43 | Blocked | No per-test timing from cargo |
| `runner_fails_when_test_exceeds_max_time` | timing.rs:127 | Blocked | Needs per-test timing |

**Action Items:**

1. Run `runner_reports_total_time` without ignore - verify it passes
2. Run `bats_runner_extracts_per_test_timing` without ignore - verify it passes
3. Remove `#[ignore]` from passing specs
4. Update remaining specs with accurate phase reference: `#[ignore = "TODO: Per-test timing extraction requires cargo JSON output"]`

**Verification:**
```bash
cargo test --test specs timing -- --ignored  # See which are blocked
cargo test --test specs timing              # After enabling, verify passes
```

### Phase 2: Remove Dead Code in Runners

**Goal:** Clean up unused code marked with `#[allow(dead_code)]`.

**Files to modify:**

1. **`runners/kcov.rs:254`** - `cleanup_kcov_output()` function
   - Determine if this should be called after kcov coverage collection
   - Either integrate or remove

2. **`runners/instrumented.rs:314`** - `cleanup_coverage_profiles()` function
   - Determine if this should be called after llvm-cov runs
   - Either integrate or remove

3. **`runners/coverage.rs:137-141`** - `LlvmCovLines` struct fields
   - `count` and `covered` fields marked dead_code
   - Verify if deserialize-only or removable

4. **`runners/jest.rs:105`** - `JestTestFile::name` field
   - Verify if deserialize-only or removable

5. **`runners/vitest.rs:107`** - `VitestTestFile::name` field
   - Verify if deserialize-only or removable

6. **`mod.rs:953`** - `SuiteResults::to_json()` method
   - Comment says "Will be used in future phases"
   - Evaluate if still needed or remove

**Pattern for deserialize-only fields:**
```rust
// If field is needed for JSON deserialization but not used directly:
#[allow(dead_code)] // Deserialized from JSON
pub name: String,

// Or use serde attribute:
#[serde(skip_serializing)]
pub name: String,
```

**Verification:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

### Phase 3: Add Missing Unit Tests for Timing Extraction

**Goal:** Ensure timing extraction is well-tested at the unit level.

**File:** `crates/cli/src/checks/tests/runners/bats_tests.rs`

Add tests for timing extraction edge cases:

```rust
#[test]
fn extracts_timing_from_tap_output() {
    let tap = r#"1..2
ok 1 fast test in 5ms
ok 2 slow test in 150ms
"#;
    let result = parse_tap_output(tap, Duration::from_millis(200));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
    assert_eq!(result.tests[0].duration, Duration::from_millis(5));
    assert_eq!(result.tests[1].duration, Duration::from_millis(150));
}

#[test]
fn handles_tap_without_timing() {
    let tap = r#"1..1
ok 1 test without timing
"#;
    let result = parse_tap_output(tap, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests[0].duration, Duration::ZERO);
}

#[test]
fn extracts_timing_with_seconds_suffix() {
    let tap = r#"1..1
ok 1 slow test in 1.5s
"#;
    let result = parse_tap_output(tap, Duration::from_secs(2));

    assert_eq!(result.tests[0].duration, Duration::from_secs_f64(1.5));
}
```

**Verification:**
```bash
cargo test --lib bats
```

### Phase 4: Improve Spec Documentation

**Goal:** Update ignored specs with accurate blocking reasons and phase references.

**Files to update:**

1. **timing.rs** - Update ignore messages:
```rust
// Before:
#[ignore = "TODO: Phase 9XX - Test runners implementation"]

// After:
#[ignore = "Blocked: Cargo test doesn't provide per-test timing in stable output"]
```

2. **coverage.rs** - Verify Phase 940 is still accurate:
```rust
// Keep as:
#[ignore = "TODO: Phase 940 - Requires runner integration"]
```

**Verification:**
```bash
cargo test --test specs -- --ignored 2>&1 | grep "ignore"
```

### Phase 5: Add Integration Test for CI Metrics

**Goal:** Add a behavioral spec that validates the full CI metrics flow.

**File:** `tests/specs/checks/tests/ci_metrics.rs`

Add comprehensive CI metrics validation:

```rust
/// Spec: CI mode JSON output includes all expected fields.
#[test]
fn ci_mode_json_has_complete_metrics_structure() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
"#,
    );
    temp.file(
        "tests/basic.bats",
        r#"
#!/usr/bin/env bats

@test "quick test" { [ 1 -eq 1 ]; }
@test "another test" { [ 2 -eq 2 ]; }
"#,
    );

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    // Top-level required fields
    assert!(metrics.get("test_count").is_some());
    assert!(metrics.get("total_ms").is_some());

    // Suites array structure
    let suites = metrics.get("suites").unwrap().as_array().unwrap();
    assert!(!suites.is_empty());

    let suite = &suites[0];
    assert!(suite.get("name").is_some());
    assert!(suite.get("runner").is_some());
    assert!(suite.get("passed").is_some());
    assert!(suite.get("test_count").is_some());
    assert!(suite.get("total_ms").is_some());

    // Per-test timing (bats provides this)
    if suite.get("test_count").unwrap().as_u64() > Some(0) {
        // avg_ms and max_ms should be present when tests exist
        assert!(suite.get("avg_ms").is_some() || suite.get("max_ms").is_some());
    }
}
```

**Verification:**
```bash
cargo test --test specs ci_metrics
```

## Key Implementation Details

### Cargo Per-Test Timing Limitation

The `cargo test` command doesn't provide per-test timing in its stable output format:

```text
test tests::fast ... ok
test tests::slow ... ok

test result: ok. 2 passed; 0 failed; finished in 0.01s
```

Options for future work:
1. **Nightly only:** Use `cargo test -- --format json -Z unstable-options`
2. **Detect capability:** Check if JSON format is available, fall back to no per-test timing
3. **Parse report-time:** Use `--report-time` flag (also unstable)

For now, cargo runner reports `Duration::ZERO` for individual tests, only `total_ms` is accurate.

### Bats Timing Extraction

The bats runner correctly extracts per-test timing from TAP output:

```text
ok 1 fast test in 5ms
ok 2 slow test in 150ms
```

The `extract_timing()` function in `bats.rs:146` handles both `ms` and `s` suffixes.

### Dead Code Decision Matrix

| Code | Keep/Remove | Reason |
|------|-------------|--------|
| `cleanup_kcov_output` | Keep | Useful for explicit cleanup, mark as `pub(crate)` |
| `cleanup_coverage_profiles` | Keep | Useful for explicit cleanup, mark as `pub(crate)` |
| `LlvmCovLines::count/covered` | Keep | Deserialize-only for JSON parsing |
| `JestTestFile::name` | Keep | Deserialize-only for JSON parsing |
| `VitestTestFile::name` | Keep | Deserialize-only for JSON parsing |
| `SuiteResults::to_json` | Remove | Duplicated by inline JSON building in run_test_suites |

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test --test specs timing` | 2+ timing specs pass |
| 2 | `cargo clippy -- -D warnings` | No dead_code warnings |
| 3 | `cargo test --lib bats` | Timing extraction tests pass |
| 4 | `cargo test --test specs -- --ignored` | Clear ignore messages |
| 5 | `cargo test --test specs ci_metrics` | CI metrics spec passes |
| All | `make check` | Full CI passes |

## Completion Criteria

- [ ] Phase 1: `runner_reports_total_time` and `bats_runner_extracts_per_test_timing` specs enabled
- [ ] Phase 2: All `#[allow(dead_code)]` annotations resolved (removed or justified)
- [ ] Phase 3: Timing extraction unit tests added
- [ ] Phase 4: Ignored specs have accurate blocking reasons
- [ ] Phase 5: CI metrics integration spec added
- [ ] All tests pass
- [ ] `make check` passes
- [ ] Changes committed
- [ ] `./done` executed

## Out of Scope (Deferred)

The following items remain tech debt for future checkpoints:

1. **Per-test timing for cargo** - Requires unstable Rust features (Phase 9XX)
2. **Shell coverage via kcov** - Requires kcov integration (Phase 940)
3. **Rust binary coverage via instrumentation** - Requires instrumented builds (Phase 940)
4. **Coverage merging across suites** - Requires above coverage features (Phase 940)
