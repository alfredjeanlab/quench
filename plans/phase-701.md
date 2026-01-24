# Phase 701: Tests Check - Specs (Correlation)

**Plan:** `phase-701`
**Root Feature:** `quench-tests`
**Reference:** `docs/specs/checks/tests.md`

## Overview

Write behavioral specifications (black-box tests) for the tests check's source/test correlation feature. This phase creates the test harness that validates the tests check correctly detects when source file changes lack corresponding test changes.

The tests check enforces that code changes are accompanied by test changes (commit checking / fast mode). These specs verify:
- Git integration (`--staged`, `--base`)
- TDD support (tests-first workflow)
- Inline test detection (`#[cfg(test)]`)
- Placeholder test recognition (`#[ignore]`)
- File exclusions (`mod.rs`, `main.rs`)
- JSON output metrics

## Project Structure

Files to create:

```
tests/
├── specs/
│   └── checks/
│       └── tests/
│           ├── mod.rs           # NEW: Module root
│           └── correlation.rs   # NEW: Source/test correlation specs
└── fixtures/
    └── tests/                   # NEW: Test fixtures
        ├── source-no-tests/     # Source change, no test change
        ├── tests-only/          # Test change only (TDD)
        ├── inline-tests/        # Inline #[cfg(test)] changes
        ├── placeholder/         # #[ignore] placeholder tests
        └── excluded/            # mod.rs, main.rs exclusions
```

## Dependencies

- No new external crates required
- Uses existing test helpers from `tests/specs/prelude.rs`
- Requires git for `--staged` and `--base` testing (via `std::process::Command`)

## Implementation Phases

### Phase 1: Create Test Module Structure

Create the tests check spec module and register it.

**File to create:** `tests/specs/checks/tests/mod.rs`

```rust
//! Behavioral specs for tests check.
//!
//! Reference: docs/specs/checks/tests.md

mod correlation;
```

**File to modify:** `tests/specs/checks/mod.rs`

Add module declaration:

```rust
mod tests;
```

**Verification:**
- [ ] `cargo test --test specs` compiles

### Phase 2: Create Git Helper Functions

Add helper functions for git operations in tests. These mirror the pattern from `tests/specs/checks/docs/commit.rs`.

**File to create:** `tests/specs/checks/tests/correlation.rs`

```rust
//! Behavioral specs for source/test correlation.
//!
//! Reference: docs/specs/checks/tests.md#commit-checking-fast-mode

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
use std::process::Command;

/// Initialize a git repo with user config and initial commit.
fn init_git_repo(path: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "chore: initial commit", "--allow-empty"])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Stage files without committing.
fn git_stage(path: &std::path::Path) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Add and commit all changes.
fn git_commit(path: &std::path::Path, msg: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}

/// Create a feature branch.
fn git_branch(path: &std::path::Path, name: &str) {
    Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(path)
        .output()
        .unwrap();
}
```

**Verification:**
- [ ] `cargo test --test specs` compiles

### Phase 3: Spec - `--staged` Checks Only Staged Files

Verify that `--staged` flag limits checking to staged (but not committed) changes.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#git-integration
///
/// > `quench check --staged` - Staged changes (pre-commit)
///
/// Only staged files are checked, unstaged changes are ignored.
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn staged_flag_checks_only_staged_files() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());

    // Add source file (unstaged)
    temp.file("src/unstaged.rs", "pub fn unstaged() {}");

    // Stage a different source file
    temp.file("src/staged.rs", "pub fn staged() {}");
    git_stage(temp.path());

    // Unstage the first file (only staged.rs should be checked)
    Command::new("git")
        .args(["restore", "--staged", "src/unstaged.rs"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should fail only for staged.rs (no test)
    check("tests")
        .pwd(temp.path())
        .args(&["--staged"])
        .fails()
        .stdout_has("staged.rs")
        .stdout_lacks("unstaged.rs");
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`
- [ ] `cargo test --test specs -- --ignored` shows spec

### Phase 4: Spec - `--base REF` Compares Against Git Ref

Verify that `--base` flag compares current state against a git reference.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#git-integration
///
/// > `quench check --base main` - Compare to branch (PR/CI)
/// > `quench check --base HEAD~5` - Compare to commits
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn base_flag_compares_against_git_ref() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());

    // Create feature branch
    git_branch(temp.path(), "feature/new-thing");

    // Add source file without test
    temp.file("src/feature.rs", "pub fn feature() {}");
    git_commit(temp.path(), "feat: add feature");

    // Check against main - should fail (source without test)
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("feature.rs");
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 5: Spec - Source Change Without Test Change Generates Violation

Verify the core functionality: source changes without corresponding test changes fail.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#check-levels
///
/// > Source changes require corresponding test changes:
/// > - New source files → require new test file (or test additions)
/// > - Modified source files → require test changes
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn source_change_without_test_change_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/no-tests");

    // Add source file without any tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    git_commit(temp.path(), "feat: add parser");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("parser.rs")
        .stdout_has("missing_tests");
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 6: Spec - Test Change Without Source Change Passes (TDD)

Verify TDD workflow support: tests-first is allowed.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#commit-scope
///
/// > Tests without code = **OK** (TDD recognized)
///
/// Writing tests before implementation is a valid workflow.
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn test_change_without_source_change_passes_tdd() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/tdd");

    // Add test file without corresponding source
    temp.file(
        "tests/parser_tests.rs",
        r#"#[test]
fn test_parse() {
    // TDD: test written first
    assert!(true);
}
"#,
    );
    git_commit(temp.path(), "test: add parser tests");

    // Should pass - TDD workflow is valid
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 7: Spec - Inline `#[cfg(test)]` Change Satisfies Test Requirement

Verify that inline test modules count as test changes.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#inline-test-changes-rust
///
/// > For Rust, changes to `#[cfg(test)]` blocks in the same file
/// > **satisfy the test requirement**
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn inline_cfg_test_change_satisfies_test_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/inline-tests");

    // Add source file with inline tests
    temp.file(
        "src/parser.rs",
        r#"pub fn parse() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert!(parse());
    }
}
"#,
    );
    git_commit(temp.path(), "feat: add parser with tests");

    // Should pass - inline tests satisfy the requirement
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 8: Spec - Placeholder Test Satisfies Test Requirement

Verify that `#[ignore]` placeholder tests count as valid correlation.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#placeholder-tests
///
/// > Placeholder tests indicate planned test implementation.
/// > ```rust
/// > #[test]
/// > #[ignore = "TODO: implement parser"]
/// > fn test_parser() { todo!() }
/// > ```
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn placeholder_test_satisfies_test_requirement() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
placeholders = "allow"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/placeholder");

    // Add source file
    temp.file("src/parser.rs", "pub fn parse() {}");

    // Add placeholder test
    temp.file(
        "tests/parser_tests.rs",
        r#"#[test]
#[ignore = "TODO: implement parser tests"]
fn test_parse() {
    todo!()
}
"#,
    );
    git_commit(temp.path(), "feat: add parser with placeholder test");

    // Should pass - placeholder indicates test intent
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 9: Spec - Excluded Files Don't Require Tests

Verify that excluded files (`mod.rs`, `main.rs`, `lib.rs`) don't require tests.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#configuration
///
/// > exclude = [
/// >   "**/mod.rs",           # Module declarations
/// >   "**/lib.rs",           # Library roots
/// >   "**/main.rs",          # Binary entry points
/// > ]
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn excluded_files_dont_require_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/excluded");

    // Add excluded files without tests
    temp.file("src/mod.rs", "pub mod parser;");
    temp.file("src/main.rs", "fn main() {}");
    temp.file("src/lib.rs", "pub mod api;");
    git_commit(temp.path(), "feat: add module files");

    // Should pass - these files are excluded by default
    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 10: Spec - JSON Includes Metrics

Verify JSON output includes the required metrics.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#json-output
///
/// > "metrics": {
/// >   "source_files_changed": 5,
/// >   "with_test_changes": 3,
/// >   ...
/// > }
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn json_includes_source_files_changed_metrics() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/metrics");

    // Add source files: 2 with tests, 1 without
    temp.file("src/good1.rs", "pub fn good1() {}");
    temp.file("tests/good1_tests.rs", "#[test] fn t() {}");
    temp.file("src/good2.rs", "pub fn good2() {}");
    temp.file("tests/good2_tests.rs", "#[test] fn t() {}");
    temp.file("src/bad.rs", "pub fn bad() {}");
    git_commit(temp.path(), "feat: add files");

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .json()
        .fails();

    let metrics = result.require("metrics");
    assert_eq!(
        metrics.get("source_files_changed").and_then(|v| v.as_u64()),
        Some(3)
    );
    assert_eq!(
        metrics.get("with_test_changes").and_then(|v| v.as_u64()),
        Some(2)
    );
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

### Phase 11: Spec - Violation Type Is Always `missing_tests`

Verify all violations from the tests check use the correct type.

**Add to:** `tests/specs/checks/tests/correlation.rs`

```rust
/// Spec: docs/specs/checks/tests.md#json-output
///
/// > **Violation types**: `missing_tests`
#[test]
#[ignore = "TODO: Phase 701 - tests check correlation"]
fn tests_violation_type_is_always_missing_tests() {
    let temp = Project::empty();
    temp.config(
        r#"[check.tests.commit]
check = "error"
"#,
    );

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/violation-type");

    // Add multiple source files without tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add parser and lexer");

    let result = check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .json()
        .fails();

    // All violations should be of type "missing_tests"
    for violation in result.violations() {
        assert_eq!(
            violation.get("type").and_then(|v| v.as_str()),
            Some("missing_tests"),
            "unexpected violation type: {:?}",
            violation
        );
    }

    // Should have at least 2 violations
    assert!(result.violations().len() >= 2);
}
```

**Verification:**
- [ ] Spec compiles with `#[ignore]`

## Key Implementation Details

### Test Pattern: Git Setup

All correlation specs follow this pattern:

1. Create temp project with config
2. Initialize git repo with initial commit
3. Create feature branch
4. Make changes (source/test files)
5. Commit changes
6. Run check with `--base main` or `--staged`

### File Matching Patterns

Per the spec, test files are matched using these patterns:
- `tests/**/*` - Everything in tests/ directory
- `test/**/*` - Everything in test/ directory
- `**/*_test.rs` - Suffix pattern
- `**/*_tests.rs` - Suffix pattern

Source files are matched against:
- `src/**/*.rs` (default)

### Excluded Files (Default)

Per spec, these files never require tests:
- `**/mod.rs` - Module declarations
- `**/lib.rs` - Library roots
- `**/main.rs` - Binary entry points
- `**/generated/**` - Generated code

### TDD Asymmetry

The tests check has asymmetric rules for commit scope:
- **Tests without code = OK** (TDD recognized)
- **Code without tests = FAIL**

This allows writing tests first, then implementation.

## Verification Plan

### Compile Check

```bash
cargo test --test specs -- --ignored 2>&1 | grep "phase-701"
```

Should show 9 ignored specs with "Phase 701" tag.

### Spec Count

```bash
cargo test --test specs checks::tests -- --ignored --list
```

Should list all 9 correlation specs.

### Full Test Suite

```bash
make check
```

All existing tests should pass. New specs are ignored until implementation.

### After Implementation

Once the tests check correlation feature is implemented:

```bash
# Remove #[ignore] from specs
# Run specs
cargo test --test specs checks::tests::correlation
```

All 9 specs should pass.
