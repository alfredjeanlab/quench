# Phase 710: Tests Check - Correlation

**Root Feature:** `quench-88fc`

## Overview

Enhance the tests check to provide comprehensive source/test correlation with multiple test location strategies and per-commit scope checking. The current implementation handles branch-level aggregation; this phase adds commit-level checking with asymmetric rules (tests-first workflows are valid).

## Project Structure

```
crates/cli/src/checks/tests/
├── mod.rs                  # TestsCheck implementation
├── correlation.rs          # Source/test matching (enhanced)
├── diff.rs                 # Git diff parsing (enhanced for commits)
├── mod_tests.rs            # Unit tests
├── correlation_tests.rs    # Correlation unit tests
└── diff_tests.rs           # Diff parsing unit tests

tests/specs/checks/tests/
├── mod.rs                  # Spec module
└── correlation.rs          # Behavioral tests (add commit scope tests)
```

## Dependencies

No new external dependencies. Uses existing:
- `globset` - Pattern matching
- `serde_json` - Metrics output

## Implementation Phases

### Phase 1: Enhanced Test Location Search

**Goal**: Improve test file discovery to match more patterns from the spec.

**Current state**: Base-name matching (`parser` -> `parser_tests`, `test_parser`)

**Target state**: Location-aware matching per spec:
1. `tests/parser.rs`
2. `tests/parser_test.rs`
3. `tests/parser_tests.rs`
4. `src/parser_test.rs` (sibling)
5. `src/parser_tests.rs` (sibling)
6. `test/parser.rs`
7. Any matching test pattern containing base name

**Files to modify**:
- `crates/cli/src/checks/tests/correlation.rs`

**Changes**:

```rust
/// Expanded test location strategy for a source file.
fn find_test_locations(source_path: &Path) -> Vec<PathBuf> {
    let base_name = source_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parent = source_path.parent().unwrap_or(Path::new(""));

    vec![
        // tests/ directory variants
        PathBuf::from(format!("tests/{}.rs", base_name)),
        PathBuf::from(format!("tests/{}_test.rs", base_name)),
        PathBuf::from(format!("tests/{}_tests.rs", base_name)),
        PathBuf::from(format!("tests/test_{}.rs", base_name)),
        // test/ directory variants
        PathBuf::from(format!("test/{}.rs", base_name)),
        PathBuf::from(format!("test/{}_test.rs", base_name)),
        PathBuf::from(format!("test/{}_tests.rs", base_name)),
        // Sibling test files (same directory)
        parent.join(format!("{}_test.rs", base_name)),
        parent.join(format!("{}_tests.rs", base_name)),
    ]
}

/// Check if any changed test file correlates with the source file.
fn has_correlated_test(
    source_path: &Path,
    test_changes: &[PathBuf],
    test_base_names: &[String],
) -> bool {
    let base_name = source_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // Strategy 1: Check expected test locations
    let expected_locations = find_test_locations(source_path);
    for test_path in test_changes {
        if expected_locations.iter().any(|loc| test_path.ends_with(loc)) {
            return true;
        }
    }

    // Strategy 2: Base name matching (existing logic)
    test_base_names.iter().any(|test_name| {
        test_name == base_name
            || *test_name == format!("{}_test", base_name)
            || *test_name == format!("{}_tests", base_name)
            || *test_name == format!("test_{}", base_name)
    })
}
```

**Verification**:
- Unit test: `find_test_locations_for_nested_source_file()`
- Unit test: `sibling_test_file_correlates()`
- Spec test: `sibling_test_file_satisfies_requirement()`

---

### Phase 2: Commit-Level Change Detection

**Goal**: Parse git log to get changes per commit on a branch.

**Files to modify**:
- `crates/cli/src/checks/tests/diff.rs`

**Add**:

```rust
/// A single commit's changes.
#[derive(Debug)]
pub struct CommitChanges {
    pub hash: String,
    pub message: String,
    pub changes: Vec<FileChange>,
}

/// Get changes per commit from base..HEAD.
pub fn get_commits_since(root: &Path, base: &str) -> Result<Vec<CommitChanges>, String> {
    // Get list of commits
    let output = Command::new("git")
        .args(["log", "--format=%H|%s", &format!("{}..HEAD", base)])
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed: {}", stderr.trim()));
    }

    let commits: Vec<(String, String)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    // Get changes for each commit
    let mut result = Vec::new();
    for (hash, message) in commits.into_iter().rev() {  // oldest first
        let numstat = run_git_diff(root, &["--numstat", &format!("{}^..{}", hash, hash)])?;
        let name_status = run_git_diff(root, &["--name-status", &format!("{}^..{}", hash, hash)])?;
        let changes = merge_diff_outputs(&numstat, &name_status, root)?;

        result.push(CommitChanges { hash, message, changes });
    }

    Ok(result)
}
```

**Verification**:
- Unit test: `get_commits_since_returns_oldest_first()`
- Unit test: `commit_changes_include_message()`

---

### Phase 3: Commit Scope with Asymmetric Rules

**Goal**: Implement per-commit checking where tests-first is OK, code-first fails.

**Spec rules**:
- Tests without code = **OK** (TDD recognized)
- Code without tests = **FAIL**
- Each commit checked independently

**Files to modify**:
- `crates/cli/src/checks/tests/mod.rs`
- `crates/cli/src/checks/tests/correlation.rs`

**Add to correlation.rs**:

```rust
/// Result of analyzing a single commit.
#[derive(Debug)]
pub struct CommitAnalysis {
    pub hash: String,
    pub message: String,
    pub source_without_tests: Vec<PathBuf>,
    pub is_test_only: bool,  // TDD commit - tests but no source
}

/// Analyze a single commit for source/test correlation.
pub fn analyze_commit(
    commit: &CommitChanges,
    config: &CorrelationConfig,
    root: &Path,
) -> CommitAnalysis {
    let result = analyze_correlation(&commit.changes, config, root);

    CommitAnalysis {
        hash: commit.hash.clone(),
        message: commit.message.clone(),
        source_without_tests: result.without_tests,
        is_test_only: !result.test_only.is_empty()
            && result.with_tests.is_empty()
            && result.without_tests.is_empty(),
    }
}
```

**Modify mod.rs run()**:

```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    let config = &ctx.config.check.tests.commit;

    if config.check == "off" {
        return CheckResult::passed(self.name());
    }

    let correlation_config = build_correlation_config(config);

    // Branch scope: aggregate all changes (existing behavior)
    if config.scope == "branch" || ctx.staged {
        return self.run_branch_scope(ctx, &correlation_config);
    }

    // Commit scope: check each commit individually
    if let Some(base) = ctx.base_branch {
        return self.run_commit_scope(ctx, base, &correlation_config);
    }

    CheckResult::passed(self.name())
}

fn run_commit_scope(
    &self,
    ctx: &CheckContext,
    base: &str,
    config: &CorrelationConfig,
) -> CheckResult {
    let commits = match get_commits_since(ctx.root, base) {
        Ok(c) => c,
        Err(e) => return CheckResult::skipped(self.name(), e),
    };

    let mut violations = Vec::new();
    let mut failing_commits = Vec::new();

    for commit in &commits {
        let analysis = analyze_commit(commit, config, ctx.root);

        // TDD commits (test-only) are OK
        if analysis.is_test_only {
            continue;
        }

        // Code without tests is a violation
        if !analysis.source_without_tests.is_empty() {
            failing_commits.push(analysis.hash.clone());

            for path in &analysis.source_without_tests {
                // Check for inline tests or placeholders (same as branch scope)
                if self.has_test_coverage(path, ctx, config) {
                    continue;
                }

                let advice = format!(
                    "Commit {} modifies {} without test changes",
                    &analysis.hash[..7],
                    path.display()
                );
                violations.push(Violation::file_only(path, "missing_tests", advice));
            }
        }
    }

    let metrics = json!({
        "commits_checked": commits.len(),
        "commits_failing": failing_commits.len(),
        "scope": "commit",
    });

    if violations.is_empty() {
        CheckResult::passed(self.name()).with_metrics(metrics)
    } else if ctx.config.check.tests.commit.check == "warn" {
        CheckResult::passed_with_warnings(self.name(), violations).with_metrics(metrics)
    } else {
        CheckResult::failed(self.name(), violations).with_metrics(metrics)
    }
}
```

**Verification**:
- Spec test: `commit_scope_fails_on_code_only_commit()`
- Spec test: `commit_scope_passes_test_only_commit_tdd()`
- Spec test: `commit_scope_passes_when_each_commit_has_tests()`

---

### Phase 4: Inline Test Detection Improvements

**Goal**: Ensure inline test changes work correctly with commit scope.

The current `has_inline_test_changes()` function accepts a base ref and diffs against it. For commit scope, we need to diff within a single commit.

**Files to modify**:
- `crates/cli/src/checks/tests/correlation.rs`

**Add**:

```rust
/// Check inline test changes for a specific commit.
pub fn has_inline_test_changes_in_commit(
    file_path: &Path,
    commit_hash: &str,
    root: &Path,
) -> bool {
    let rel_path = file_path.strip_prefix(root).unwrap_or(file_path);
    let rel_path_str = match rel_path.to_str() {
        Some(s) => s,
        None => return false,
    };

    let range = format!("{}^..{}", commit_hash, commit_hash);
    let output = Command::new("git")
        .args(["diff", &range, "--", rel_path_str])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let diff = String::from_utf8_lossy(&o.stdout);
            changes_in_cfg_test(&diff)
        }
        _ => false,
    }
}
```

**Verification**:
- Unit test: `inline_test_changes_detected_in_single_commit()`
- Spec test: `commit_scope_inline_cfg_test_satisfies()`

---

### Phase 5: Integration and Polish

**Goal**: Integrate all components and add comprehensive tests.

**Tasks**:
1. Update metrics output for both scopes
2. Add advisory messages for commit scope violations
3. Ensure placeholder detection works in commit scope
4. Add behavioral specs for edge cases

**Spec tests to add**:

```rust
// tests/specs/checks/tests/correlation.rs

/// Spec: docs/specs/checks/tests.md#commit-scope
#[test]
fn commit_scope_fails_on_source_without_tests() {
    let temp = Project::empty();
    temp.config(r#"
[check.tests.commit]
check = "error"
scope = "commit"
"#);

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/commit-scope");

    // First commit: tests only (TDD) - OK
    temp.file("tests/parser_tests.rs", "#[test] fn t() {}");
    git_commit(temp.path(), "test: add parser tests");

    // Second commit: source without tests - FAIL
    temp.file("src/lexer.rs", "pub fn lex() {}");
    git_commit(temp.path(), "feat: add lexer");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .fails()
        .stdout_has("lexer.rs")
        .stdout_lacks("parser");  // TDD commit should pass
}

/// Spec: docs/specs/checks/tests.md#commit-scope
#[test]
fn commit_scope_passes_when_each_commit_correlates() {
    let temp = Project::empty();
    temp.config(r#"
[check.tests.commit]
check = "error"
scope = "commit"
"#);

    init_git_repo(temp.path());
    git_branch(temp.path(), "feature/proper-commits");

    // Each commit has both source and tests
    temp.file("src/parser.rs", "pub fn parse() {}");
    temp.file("tests/parser_tests.rs", "#[test] fn t() {}");
    git_commit(temp.path(), "feat: add parser with tests");

    temp.file("src/lexer.rs", "pub fn lex() {}");
    temp.file("tests/lexer_tests.rs", "#[test] fn t() {}");
    git_commit(temp.path(), "feat: add lexer with tests");

    check("tests")
        .pwd(temp.path())
        .args(&["--base", "main"])
        .passes();
}
```

**Verification**:
- All existing specs continue to pass
- New commit scope specs pass
- `make check` passes

---

## Key Implementation Details

### Base Name Extraction

Source file `src/foo/parser.rs` extracts base name `parser`:
- Matches: `parser_tests.rs`, `parser_test.rs`, `test_parser.rs`, `parser.rs` (in tests/)
- Does NOT match: `foo_parser.rs`, `parserutil.rs`

### Asymmetric Rules

```
Commit 1: tests/parser.rs     -> PASS (TDD)
Commit 2: src/parser.rs       -> PASS (tests exist from commit 1, but wait...)
```

Important: In commit scope, **each commit is checked independently**. Commit 2 has source changes with no test changes in that commit, so it would fail. The "tests first" rule means you write tests in the same commit as the code, or write tests first and code second (both in same commit is fine).

Clarification from spec:
- "Tests without code = OK" means a commit with ONLY test files passes
- "Code without tests = FAIL" means a commit with source changes must also have test changes

### Scope Selection

```toml
[check.tests.commit]
scope = "branch"   # All changes aggregated (default)
scope = "commit"   # Per-commit with asymmetric rules
```

`--staged` always uses branch-like behavior (single unit of changes).

---

## Verification Plan

### Unit Tests

```bash
cargo test --package quench -- checks::tests
```

Tests:
- `find_test_locations_for_source_file()`
- `sibling_test_correlates()`
- `get_commits_since_parses_log()`
- `analyze_commit_detects_test_only()`
- `inline_test_changes_in_commit()`

### Behavioral Specs

```bash
cargo test --test specs -- correlation
```

New specs:
- `commit_scope_fails_on_source_without_tests()`
- `commit_scope_passes_test_only_commit()`
- `commit_scope_passes_source_with_tests_in_same_commit()`
- `commit_scope_with_inline_tests()`
- `sibling_test_file_satisfies_requirement()`

### Integration

```bash
make check
```

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

### Manual Verification

1. Create test repo with multiple commits
2. Run `quench check --base main` with `scope = "branch"` - aggregates
3. Run `quench check --base main` with `scope = "commit"` - per-commit
4. Verify TDD workflow (test-only commits) passes in both modes
5. Verify code-only commits fail in commit scope mode
