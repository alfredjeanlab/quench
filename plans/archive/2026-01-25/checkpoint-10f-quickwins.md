# Checkpoint 10F: Quick Wins - Dogfooding Milestone 2

## Overview

This checkpoint addresses technical debt and quick improvements identified during dogfooding. With the core features complete (checks, caching, timing, pre-commit hooks), this phase focuses on:

1. **Test coverage gaps** - Add missing JSON formatter unit tests
2. **Struct improvements** - Add `scope` field to Violation for better JSON reporting
3. **Test DRYness** - Refactor repetitive test patterns with helpers and parameterization
4. **Documentation cleanup** - Archive completed plans, update status

These are low-risk, high-value improvements that strengthen the codebase before Milestone 3 (full CI integration).

## Project Structure

Changes will touch:

```
crates/cli/src/
├── check.rs                    # Add scope field to Violation
├── output/
│   ├── json.rs                 # Existing (no changes)
│   └── json_tests.rs           # NEW: Unit tests for JSON formatter
├── file_size_tests.rs          # Refactor: parameterized tests
├── git_tests.rs                # Refactor: add file helper
└── report/
    └── mod_tests.rs            # Refactor: add filter helper
plans/
├── checkpoint-10f-quickwins.md # This plan
└── archive/                    # Move completed plans
```

## Dependencies

No new external dependencies. All changes use existing crates:
- `serde_json` (already in deps) - JSON assertions in tests
- `tempfile` (already in dev-deps) - Test fixtures

## Implementation Phases

### Phase 1: Violation Scope Field

**Goal**: Enable richer JSON output by adding optional scope metadata to violations.

**Files**:
- `crates/cli/src/check.rs` - Add `scope: Option<String>` field

**Changes**:

```rust
// In check.rs, Violation struct
pub struct Violation {
    pub check: CheckKind,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub scope: Option<String>,  // NEW: e.g., "api", "cli", "parser"
    pub suggestion: Option<String>,
    pub baseline: Option<bool>,
}

impl Violation {
    // Add builder method
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }
}
```

**Verification**:
- [ ] Existing tests pass unchanged (field is optional)
- [ ] JSON output includes `scope` when set
- [ ] `cargo clippy` passes

### Phase 2: JSON Formatter Tests

**Goal**: Close test coverage gap for JSON output formatting.

**Files**:
- `crates/cli/src/output/json_tests.rs` - NEW file

**Test Cases** (based on other formatter tests):

1. **Empty report** - No violations, clean JSON structure
2. **Single violation** - All fields populated
3. **Multiple violations** - Array ordering, deduplication
4. **With scope field** - New scope field renders correctly
5. **With timing data** - `--timing` output in JSON
6. **Schema validation** - Output matches documented schema

**Template** (from text_tests.rs pattern):

```rust
// crates/cli/src/output/json_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::report::Report;
use serde_json::Value;

#[test]
fn formats_empty_report() {
    let report = Report::default();
    let output = JsonFormatter::new().format(&report);
    let json: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(json["violations"], Value::Array(vec![]));
    assert_eq!(json["summary"]["total"], 0);
}

#[test]
fn formats_violation_with_scope() {
    // ... test scope field appears in JSON
}
```

**Verification**:
- [ ] All new tests pass
- [ ] Coverage for JSON formatter established
- [ ] `cargo test output::json` passes

### Phase 3: Test Refactoring - File Size Tests

**Goal**: Reduce code duplication in file_size_tests.rs using parameterization.

**Current State**: 4 near-identical test functions (~50 LOC)
**Target**: Single parameterized test (~20 LOC)

**Pattern**:

```rust
// Before: 4 separate functions
#[test]
fn flags_large_source_file() { ... }
#[test]
fn flags_large_test_file() { ... }
// etc.

// After: parameterized
#[rstest]
#[case("source.rs", 800, true, "source file exceeds 750 lines")]
#[case("test.rs", 1200, true, "test file exceeds 1100 lines")]
#[case("source.rs", 700, false, "")]
#[case("test.rs", 1000, false, "")]
fn file_size_violations(
    #[case] filename: &str,
    #[case] lines: usize,
    #[case] expect_violation: bool,
    #[case] message_contains: &str,
) {
    // Single implementation
}
```

**Note**: If `rstest` is not already a dev-dependency, use a simple loop-based approach instead:

```rust
#[test]
fn file_size_violations() {
    let cases = [
        ("source.rs", 800, true),
        ("test.rs", 1200, true),
        ("source.rs", 700, false),
        ("test.rs", 1000, false),
    ];
    for (filename, lines, expect_violation) in cases {
        // Test each case
    }
}
```

**Verification**:
- [ ] Equivalent test coverage maintained
- [ ] LOC reduced by ~30 lines
- [ ] All tests pass

### Phase 4: Test Refactoring - Git & Baseline Helpers

**Goal**: Extract common test patterns into helper functions.

**4a. Git Test Helper**

```rust
// In git_tests.rs
fn create_and_stage_file(repo: &TempRepo, name: &str, content: &str) {
    let path = repo.path().join(name);
    fs::write(&path, content).unwrap();
    repo.git(&["add", name]);
}
```

**4b. Baseline Filter Helper**

```rust
// In report/mod_tests.rs
fn assert_baseline_filtered(
    violations: Vec<Violation>,
    baseline: &[&str],
    expected_count: usize,
) {
    let report = Report::new(violations);
    let filtered = report.filter_baseline(baseline);
    assert_eq!(filtered.violations.len(), expected_count);
}
```

**Verification**:
- [ ] No behavioral changes
- [ ] Reduced code duplication
- [ ] All tests pass

### Phase 5: Documentation & Cleanup

**Goal**: Archive completed plans, update project status.

**Tasks**:

1. **Archive this plan** (after completion):
   ```bash
   mkdir -p plans/archive/2026-01-25
   mv plans/checkpoint-10f-quickwins.md plans/archive/2026-01-25/
   ```

2. **Update CHANGELOG** (if exists) with quick wins summary

3. **Verify dogfooding status**:
   ```bash
   quench check --timing  # Should pass with <50ms
   ```

**Verification**:
- [ ] All plans archived appropriately
- [ ] quench passes on itself
- [ ] `make check` passes

## Key Implementation Details

### Scope Field Design

The `scope` field is intentionally optional and generic:
- **For git checks**: Could be the commit scope (e.g., "cli", "docs")
- **For code checks**: Could be the module or package name
- **Default**: `None` (backwards compatible)

This design allows future checks to provide richer context without breaking existing behavior.

### Test Parameterization Strategy

Prefer simple loop-based tests over macro-heavy approaches:
- More readable for maintainers
- No additional dependencies
- Clear failure messages (include case index in assertions)

```rust
for (i, (input, expected)) in cases.iter().enumerate() {
    let result = function_under_test(input);
    assert_eq!(result, *expected, "Case {i} failed: input={input:?}");
}
```

### JSON Test Structure

Follow the pattern established by `text_tests.rs` and `html_tests.rs`:
1. Create minimal Report fixtures
2. Format to string
3. Parse and assert on structure
4. Use `serde_json::Value` for flexible assertions

## Verification Plan

### Per-Phase Verification

Each phase has inline checkboxes. Complete these before moving to the next phase.

### Final Verification

Run the full check suite:

```bash
# Full test suite
make check

# Quench on itself (dogfooding)
cargo run -- check --timing

# Specific test modules
cargo test check::tests
cargo test output::json
cargo test file_size
cargo test git_tests
cargo test report::mod_tests
```

### Success Criteria

1. **All tests pass**: `cargo test --all` exits 0
2. **No new warnings**: `cargo clippy` clean
3. **Dogfooding passes**: `quench check` on quench reports 0 violations
4. **Performance maintained**: `--timing` shows <100ms warm run
5. **LOC reduced**: Test files have net negative LOC change (~-50 lines)

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1. Scope field | Low - additive change | Optional field, no breaking changes |
| 2. JSON tests | Very low - new tests only | No production code changes |
| 3-4. Refactoring | Low - test code only | Run full suite after each change |
| 5. Cleanup | Very low - documentation | Review before archiving |

## Timeline Summary

| Phase | Description | Estimated Effort |
|-------|-------------|------------------|
| 1 | Violation scope field | 30 minutes |
| 2 | JSON formatter tests | 1-2 hours |
| 3 | File size test refactor | 30 minutes |
| 4 | Git & baseline helpers | 30 minutes |
| 5 | Documentation cleanup | 15 minutes |

Total: ~3-4 hours of focused work
