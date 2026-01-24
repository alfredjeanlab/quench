# Checkpoint 8A: Pre-Checkpoint Fix - Tests Correlation Complete

**Plan:** `checkpoint-8a-precheck`
**Root Feature:** `quench-tests`
**Depends On:** Phase 720 (Tests Check - Output Enhancement)

## Overview

Verify and document completion of the tests correlation feature. The implementation exists and passes all behavioral specs. This checkpoint validates feature completeness against the specification and ensures no gaps remain before moving to the next milestone.

**Current State:**
- Core implementation: **Complete** (`crates/cli/src/checks/tests/`)
- Branch scope: **Complete** with metrics
- Commit scope: **Complete** with metrics
- JSON output: **Complete** with `change_type` and `lines_changed`
- Behavioral specs: **22 specs passing**
- Unit tests: **All passing**
- `make check`: **Passing**

**Goal:** Confirm tests correlation feature is production-ready.

## Project Structure

```
crates/cli/src/checks/tests/
├── mod.rs              # Main check implementation (352 lines)
├── mod_tests.rs        # Unit tests
├── correlation.rs      # Source/test matching logic
└── diff.rs             # Git diff parsing, ChangeType enum

tests/specs/checks/tests/
└── correlation.rs      # 22 behavioral specs (all passing)

docs/specs/checks/
└── tests.md            # Feature specification
```

## Dependencies

No new dependencies. Feature complete with existing:
- `serde` / `serde_json` - JSON serialization
- `globset` - Pattern matching for test files

## Implementation Phases

### Phase 1: Spec-Implementation Audit

**Goal:** Verify every spec requirement has corresponding implementation and test coverage.

**Spec requirements from `docs/specs/checks/tests.md`:**

| Requirement | Implementation | Spec |
|-------------|----------------|------|
| Branch scope (default) | `run_branch_scope()` | `branch_scope_aggregates_all_changes` |
| Commit scope | `run_commit_scope()` | `commit_scope_*` (5 specs) |
| TDD recognition | Tests-only commits pass | `test_change_without_source_change_passes_tdd` |
| Inline tests (Rust) | `has_inline_test_changes()` | `inline_cfg_test_change_satisfies_test_requirement` |
| Placeholder tests | `has_placeholder_test()` | `placeholder_test_satisfies_test_requirement` |
| --staged flag | `get_staged_changes()` | `staged_flag_checks_only_staged_files` |
| --base flag | `get_base_changes()` | `base_flag_compares_against_git_ref` |
| Exclusion patterns | `CorrelationConfig.exclude` | `excluded_files_dont_require_tests` |
| change_type field | `Violation.with_change_info()` | `missing_tests_json_includes_change_type_*` |
| lines_changed field | `Violation.with_change_info()` | `missing_tests_json_includes_lines_changed` |
| Branch metrics | `source_files_changed`, etc. | `json_includes_source_files_changed_metrics` |
| Commit metrics | `commits_checked`, etc. | `commit_scope_json_includes_commit_metrics` |

**Verification:**
```bash
cargo test --test specs -- correlation
```

---

### Phase 2: Edge Case Verification

**Goal:** Ensure edge cases are handled correctly.

**Edge cases to verify:**

1. **Empty diff** - No changes to check
   - Expected: Pass silently
   - Code: `run_branch_scope()` returns `CheckResult::passed()`

2. **All deletions** - Only deleted files
   - Expected: Pass (deletions don't require tests)
   - Code: `ChangeType::Deleted` filtered out in correlation

3. **Mixed changes** - Some files pass, some fail
   - Expected: Report only failing files
   - Code: `result.without_tests` contains only uncovered files

4. **Nested test files** - Tests in subdirectories
   - Expected: Pattern matching finds them
   - Code: `test_patterns` supports `tests/**/*`

5. **No git context** - Neither --staged nor --base
   - Expected: Pass silently (no changes to check)
   - Code: Returns `CheckResult::passed()` at line 97

**Manual verification:**
```bash
# Empty diff
cd /tmp && mkdir empty-test && cd empty-test
git init && git commit --allow-empty -m "init"
quench check tests  # Should pass

# Deletion only
git rm some-file.rs
quench check tests --staged  # Should pass
```

---

### Phase 3: Output Format Validation

**Goal:** Verify JSON output matches spec exactly.

**Expected JSON structure (from spec):**

```json
{
  "name": "tests",
  "passed": false,
  "violations": [
    {
      "file": "src/parser.rs",
      "line": null,
      "type": "missing_tests",
      "change_type": "modified",
      "lines_changed": 79,
      "advice": "Add tests in tests/parser_tests.rs or update inline #[cfg(test)] block"
    }
  ],
  "metrics": {
    "source_files_changed": 5,
    "with_test_changes": 3,
    "without_test_changes": 2,
    "scope": "branch"
  }
}
```

**Validation checklist:**
- [ ] `change_type` is "added" | "modified" (never "deleted")
- [ ] `lines_changed` is positive integer
- [ ] `scope` is "branch" | "commit"
- [ ] Metrics present even on pass
- [ ] `line` is `null` for file-level violations

**Verification:**
```bash
cargo test --test specs -- missing_tests_json
```

---

### Phase 4: Configuration Coverage

**Goal:** Verify all config options work as documented.

**Configuration options:**

```toml
[check.tests.commit]
check = "error"              # error | warn | off
scope = "branch"             # branch | commit
placeholders = "allow"       # allow | forbid
test_patterns = [...]        # Custom patterns
source_patterns = [...]      # Source file patterns
exclude = [...]              # Files that never need tests
```

**Test matrix:**

| Option | Value | Expected Behavior |
|--------|-------|-------------------|
| `check = "off"` | Disabled | Skip entirely, always pass |
| `check = "warn"` | Advisory | Report but exit 0 |
| `check = "error"` | Enforce | Fail on violations |
| `scope = "branch"` | Aggregate | All commits count together |
| `scope = "commit"` | Per-commit | Each commit checked independently |
| `placeholders = "allow"` | Lenient | #[ignore] tests satisfy |
| `placeholders = "forbid"` | Strict | #[ignore] tests don't count |

**Verification:**
```bash
cargo test --test specs -- tests::correlation
```

---

### Phase 5: Integration Testing

**Goal:** Run full test suite and verify no regressions.

**Commands:**
```bash
# All tests check specs
cargo test --test specs -- tests::correlation

# Full test suite
cargo test --all

# Full CI check
make check
```

**Verification checklist:**
- [ ] All 22 correlation specs pass
- [ ] All unit tests pass
- [ ] No clippy warnings
- [ ] `make check` completes successfully

---

### Phase 6: Documentation Check

**Goal:** Ensure spec and implementation are aligned.

**Tasks:**
1. Compare `docs/specs/checks/tests.md` against implementation
2. Verify all documented features have corresponding specs
3. Check for any undocumented behavior

**Documented but not yet implemented (future work):**
- CI mode test execution (`--ci`)
- Coverage collection
- Test time metrics
- Test suites configuration

These are explicitly out of scope for this checkpoint (commit checking only).

**Verification:**
```bash
# Ensure no Phase 720 references remain
grep -r "Phase 720" tests/specs/
# Should return empty

# Verify all correlation specs exist
cargo test --test specs -- correlation 2>&1 | grep -c "test result: ok"
```

## Key Implementation Details

### Violation Fields

The `Violation` struct includes optional fields for tests check:

```rust
// crates/cli/src/check.rs
pub struct Violation {
    pub file: String,
    pub line: Option<usize>,
    #[serde(rename = "type")]
    pub violation_type: String,
    pub advice: String,

    // Tests check fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_changed: Option<i64>,
}
```

Builder method for setting both:
```rust
pub fn with_change_info(mut self, change_type: impl Into<String>, lines_changed: i64) -> Self {
    self.change_type = Some(change_type.into());
    self.lines_changed = Some(lines_changed);
    self
}
```

### Scope Selection Logic

```rust
// Branch scope (default): aggregate all changes
// Commit scope: check each commit individually
// Staged mode: always uses branch-like behavior

if config.scope == "commit"
    && !ctx.staged
    && let Some(base) = ctx.base_branch
{
    return self.run_commit_scope(ctx, base, &correlation_config);
}
self.run_branch_scope(ctx, &correlation_config)
```

### Test File Matching

Pattern-based matching for source file `src/parser.rs`:
1. `tests/parser_tests.rs`
2. `tests/parser_test.rs`
3. `tests/parser.rs`
4. `src/parser_tests.rs`
5. `src/parser_test.rs`
6. `test/parser.rs`

### Default Exclusions

```rust
const DEFAULT_EXCLUDE: &[&str] = &[
    "**/mod.rs",
    "**/lib.rs",
    "**/main.rs",
    "**/generated/**",
];
```

## Verification Plan

### Unit Tests
```bash
cargo test -p quench -- checks::tests
```

Tests:
- `violation_with_change_info_serializes_correctly`
- `violation_without_change_info_omits_fields`
- Configuration parsing tests

### Behavioral Specs
```bash
cargo test --test specs -- tests::correlation
```

22 Specs:
- `staged_flag_checks_only_staged_files`
- `base_flag_compares_against_git_ref`
- `source_change_without_test_change_generates_violation`
- `test_change_without_source_change_passes_tdd`
- `inline_cfg_test_change_satisfies_test_requirement`
- `placeholder_test_satisfies_test_requirement`
- `excluded_files_dont_require_tests`
- `json_includes_source_files_changed_metrics`
- `tests_violation_type_is_always_missing_tests`
- `missing_tests_json_includes_change_type_modified`
- `missing_tests_json_includes_change_type_added`
- `missing_tests_json_includes_lines_changed`
- `commit_scope_fails_on_source_without_tests`
- `commit_scope_passes_test_only_commit_tdd`
- `commit_scope_passes_when_each_commit_has_tests`
- `commit_scope_inline_cfg_test_satisfies`
- `branch_scope_aggregates_all_changes`
- `commit_scope_sibling_test_file_satisfies`
- `commit_scope_json_includes_commit_metrics`

### Full Suite
```bash
make check
```

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

### Completion Criteria

- [ ] All 22 behavioral specs pass
- [ ] All unit tests pass
- [ ] `make check` completes successfully
- [ ] No ignored specs remain for tests correlation
- [ ] Spec matches implementation (no undocumented behavior)
- [ ] Cache version correct (`v21` in `cache.rs`)
