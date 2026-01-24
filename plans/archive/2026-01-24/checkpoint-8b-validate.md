# Checkpoint 8B: Tests Correlation Complete - Validation

**Root Feature:** `quench-91c5`

## Overview

Validate the tests correlation feature (commit/branch checking) by running the existing implementation through structured tests and documenting results. The feature is already implemented; this checkpoint verifies it meets all criteria and adds snapshot tests for output stability.

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── mod.rs           # Tests check implementation
│   ├── correlation.rs   # Source/test correlation logic
│   └── diff.rs          # Git diff parsing
├── tests/
│   ├── specs/checks/tests/
│   │   ├── mod.rs
│   │   ├── correlation.rs  # Existing behavioral tests
│   │   └── output.rs       # NEW: Output snapshot tests
│   └── fixtures/           # Test fixtures (uses temp dirs for git tests)
└── reports/
    └── checkpoint-8-tests-correlation.md  # Validation report
```

## Dependencies

No new dependencies required. Uses existing:
- `assert_cmd` for CLI testing
- `predicates` for output assertions
- Git commands for integration testing

## Implementation Phases

### Phase 1: Verify Staged Mode Works

**Goal**: Confirm `quench check --staged` works correctly in fixtures with staged changes.

**Steps**:
1. Run existing test: `staged_flag_checks_only_staged_files`
2. Manually verify in a temp project:
   ```bash
   cd $(mktemp -d)
   git init && git config user.email "t@t" && git config user.name "T"
   echo '[check.tests.commit]\ncheck = "error"' > quench.toml
   git add quench.toml && git commit -m "init"

   mkdir src && echo "pub fn f() {}" > src/lib.rs
   git add src/lib.rs
   quench check --staged  # Should FAIL with missing_tests
   ```
3. Document results in validation report

**Verification**: Test passes, manual verification succeeds

### Phase 2: Verify Base Mode Works

**Goal**: Confirm `quench check --base main` works correctly in fixtures with branch changes.

**Steps**:
1. Run existing tests:
   - `base_flag_compares_against_git_ref`
   - `source_change_without_test_change_generates_violation`
   - `branch_scope_aggregates_all_changes`
2. Manually verify in a temp project:
   ```bash
   cd $(mktemp -d)
   git init && git config user.email "t@t" && git config user.name "T"
   echo '[check.tests.commit]\ncheck = "error"' > quench.toml
   git add . && git commit -m "init"

   git checkout -b feature/test
   mkdir src && echo "pub fn f() {}" > src/lib.rs
   git add . && git commit -m "add feature"
   quench check --base main  # Should FAIL with missing_tests
   ```
3. Document results in validation report

**Verification**: Tests pass, manual verification succeeds

### Phase 3: Add Output Snapshot Tests

**Goal**: Add exact output tests for tests correlation to catch format regressions.

**Files to create/modify**:
- `tests/specs/checks/tests/output.rs` (NEW)
- `tests/specs/checks/tests/mod.rs` (add `mod output;`)

**Tests to add**:

```rust
//! Output format specs for tests check.

use crate::prelude::*;
use std::process::Command;

fn init_git_repo(path: &std::path::Path) { /* reuse from correlation.rs */ }
fn git_stage(path: &std::path::Path) { /* reuse */ }
fn git_commit(path: &std::path::Path, msg: &str) { /* reuse */ }
fn git_branch(path: &std::path::Path, name: &str) { /* reuse */ }

/// Spec: Text output format for missing_tests violation
#[test]
fn tests_text_output_missing_tests_staged() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}");
    git_stage(temp.path());

    check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .fails()
        .stdout_eq("tests: FAIL
  src/feature.rs: missing_tests (added, +1 lines)
    Add tests for this file.

FAIL: tests
");
}

/// Spec: Text output format for branch mode with multiple violations
#[test]
fn tests_text_output_missing_tests_branch() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);
    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/test");

    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add parser and lexer");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("tests: FAIL")
        .stdout_has("src/parser.rs: missing_tests")
        .stdout_has("src/lexer.rs: missing_tests");
}

/// Spec: JSON output includes change_type and lines_changed
#[test]
fn tests_json_output_violation_structure() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}\npub fn more() {}");
    git_stage(temp.path());

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .json()
        .fails();

    let violations = result.violations();
    assert_eq!(violations.len(), 1);

    let v = &violations[0];
    assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("missing_tests"));
    assert_eq!(v.get("change_type").and_then(|v| v.as_str()), Some("added"));
    assert_eq!(v.get("lines_changed").and_then(|v| v.as_i64()), Some(2));
}

/// Spec: Text output passes when tests exist
#[test]
fn tests_text_output_passes() {
    let temp = Project::empty();
    temp.config(r#"[check.tests.commit]
check = "error"
"#);
    init_git_repo(temp.path());

    temp.file("src/feature.rs", "pub fn feature() {}");
    temp.file("tests/feature_tests.rs", "#[test] fn t() {}");
    git_stage(temp.path());

    check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .passes();
}
```

**Verification**: `cargo test --test specs checks_tests::output`

### Phase 4: Run Full Test Suite

**Goal**: Ensure all tests correlation specs pass.

**Steps**:
1. Run all correlation tests: `cargo test --test specs checks_tests`
2. Run full spec suite: `cargo test --test specs`
3. Run `make check` for complete validation

**Verification**: All tests pass, no regressions

### Phase 5: Write Validation Report

**Goal**: Create `reports/checkpoint-8-tests-correlation.md` documenting validation results.

**Report structure**:
```markdown
# Checkpoint 8: Tests Correlation Complete - Validation Report

**Date**: YYYY-MM-DD
**Status**: PASS/FAIL

## Summary

| Criterion | Status | Details |
|-----------|--------|---------|
| --staged works | PASS/FAIL | ... |
| --base works | PASS/FAIL | ... |
| Output snapshots | PASS/FAIL | N specs added |

## Phase 1: Staged Mode Validation
[Test output, manual verification results]

## Phase 2: Base Mode Validation
[Test output, manual verification results]

## Phase 3: Output Snapshot Tests
[New tests added, test run output]

## Phase 4: Full Test Suite
[Test counts, make check output]

## Conclusion
[Summary of checkpoint completion]
```

**Verification**: Report exists and documents passing status

## Key Implementation Details

### Git Test Helpers

The correlation tests use helper functions to set up git repos:
- `init_git_repo()` - Initialize git repo with config
- `git_stage()` - Stage files
- `git_commit()` - Add and commit
- `git_branch()` - Create feature branch

These should be reused (or extracted to a shared module) for output tests.

### Output Format Patterns

Tests correlation output follows the standard format:
```
tests: FAIL
  <file>: missing_tests (<change_type>, +N lines)
    Add tests for this file.

FAIL: tests
```

JSON output includes:
```json
{
  "type": "missing_tests",
  "file": "src/feature.rs",
  "change_type": "added",
  "lines_changed": 10,
  "advice": "Add tests for this file."
}
```

### Existing Test Coverage

The following scenarios are already tested in `correlation.rs`:
- Staged mode: `staged_flag_checks_only_staged_files`
- Base mode: `base_flag_compares_against_git_ref`
- TDD workflow: `test_change_without_source_change_passes_tdd`
- Inline tests: `inline_cfg_test_change_satisfies_test_requirement`
- Placeholders: `placeholder_test_satisfies_test_requirement`
- Exclusions: `excluded_files_dont_require_tests`
- Commit scope: `commit_scope_*` tests
- Branch scope: `branch_scope_aggregates_all_changes`
- JSON output: `json_includes_*` and `missing_tests_json_*` tests

## Verification Plan

### Unit Tests
No new unit tests needed - feature is implemented.

### Behavioral Tests
1. Existing: 26 tests in `tests/specs/checks/tests/correlation.rs`
2. New: 4 tests in `tests/specs/checks/tests/output.rs` (snapshot tests)

### Manual Verification
1. Create temp git repo
2. Stage source file without test
3. Run `quench check --staged` - verify failure
4. Create feature branch with changes
5. Run `quench check --base main` - verify failure
6. Add tests, verify pass

### CI Validation
Run `make check` which includes:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`
