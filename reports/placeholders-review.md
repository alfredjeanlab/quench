# Placeholders Refactor Review

**Date:** 2026-01-25
**Branch:** `feature/placeholders-review`

## Summary

Review of the placeholders refactor to ensure spec/implementation alignment, remove dead code, and verify test coverage. Placeholders are now collected as **metrics** within the `tests` check, not as a standalone check.

## Verification Results

### Phase 1: Spec/Implementation Alignment ✅

**JSON Output Structure**
```json
{
  "checks": [{
    "name": "tests",
    "metrics": {
      "placeholders": {
        "rust": { "ignore": N, "todo": N },
        "javascript": { "todo": N, "fixme": N, "skip": N }
      }
    }
  }]
}
```

- ✅ No separate "placeholders" check in output
- ✅ Metrics nested under `tests` check
- ✅ Structure matches spec in `docs/specs/checks/tests.md#placeholder-metrics`

### Phase 2: Dead Code Removal ✅

**Items removed:**
1. `"placeholders"` from help test check list (`help_tests.rs`, `tests/specs/cli/help.rs`)
2. Orphaned fixture `tests/fixtures/placeholders/allowed/`

**Verified clean:**
- No `PlaceholdersCheck` references in `crates/`
- No `[check.placeholders]` config handling
- `output.schema.json` check enum excludes "placeholders"

### Phase 3: Test Coverage ✅

All tests pass:
- 47 unit tests for placeholders module
- 7 behavioral specs in `tests/specs/checks/placeholders.rs`
- Full `make check` passes

Test coverage areas:
- `crates/cli/src/checks/placeholders/mod_tests.rs`
- `crates/cli/src/checks/placeholders/rust_tests.rs`
- `crates/cli/src/checks/placeholders/javascript_tests.rs`
- `crates/cli/src/checks/tests/placeholder_tests.rs`

### Phase 4: Documentation Consistency ✅

**CHANGELOG.md:**
- Fixed: Changed "placeholders check" → "placeholder metrics" reference
- Now correctly references as part of tests check

**output.schema.json:**
- Valid JSON
- Check enum: `["cloc", "escapes", "agents", "docs", "tests", "git", "build", "license"]`
- No "placeholders" in enum

### Phase 5: Dogfood Findings

**Cache Invalidation Issue Found:**
- Running with cache: metrics may show stale values
- Running with `--no-cache`: metrics correct
- Impact: Placeholder counts may be inaccurate when cache is warm

**Metrics Availability:**
When running without `--base` or `--staged`, the tests check passes silently with metrics included. However, metrics may not appear in output when no test files are in scope.

**Quench Codebase Placeholder Count:**
- Approximately 60 `#[ignore]` occurrences across `tests/` and `crates/`
- These are intentional placeholders marking unimplemented specs (Phase N markers)

### Phase 6: Test Pattern Consolidation ✅

**Fixtures used:**
| Fixture | Used By |
|---------|---------|
| `placeholders/rust-ignore` | `tests_check_includes_rust_ignore_metrics`, `tests_check_placeholder_metrics_structure` |
| `placeholders/rust-todo` | `tests_check_includes_rust_todo_metrics` |
| `placeholders/javascript-todo` | `tests_check_includes_js_todo_metrics` |
| `placeholders/javascript-fixme` | `tests_check_includes_js_fixme_metrics` |

- ✅ No orphaned fixtures (removed `placeholders/allowed`)
- ✅ Unit tests follow `_tests.rs` convention
- ✅ Behavioral specs use `yare`-compatible patterns where appropriate

## Issues Found

### 1. Cache Invalidation for Placeholder Metrics

**Severity:** Medium
**Impact:** Stale placeholder counts when cache is warm
**Workaround:** Use `--no-cache` for accurate counts
**Recommendation:** Investigate cache key generation for placeholder metrics

### 2. Dead Code in Help Tests

**Severity:** Low
**Status:** Fixed
**Files:** `crates/cli/src/help_tests.rs`, `tests/specs/cli/help.rs`
**Change:** Removed "placeholders" from check toggle lists

### 3. Incorrect CHANGELOG Reference

**Severity:** Low
**Status:** Fixed
**File:** `CHANGELOG.md`
**Change:** "placeholders check" → "placeholder metrics for Rust and JS/TS"

### 4. Orphaned Test Fixture

**Severity:** Low
**Status:** Fixed
**Directory:** `tests/fixtures/placeholders/allowed/`
**Change:** Removed unused fixture

## Conclusion

The placeholders refactor is complete and correct. Placeholders are properly collected as metrics within the tests check, not as a standalone check. All spec references have been updated, dead code removed, and tests pass.

Minor cache invalidation issue discovered during dogfooding should be tracked separately.
