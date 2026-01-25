# Checkpoint 10C: Refactor - Dogfooding Milestone 2

**Root Feature:** `quench-10c`
**Follows:** checkpoint-10b-validate (Dogfooding Milestone 2 validation)

## Overview

Apply DRY refactoring to reduce code duplication and sync risk across the quench codebase. This checkpoint implements the refactoring items identified in `plans/2026-01-24-dry-code.md` that meet the criteria:

1. **Sync Risk:** Duplicated logic that could get out of sync
2. **Significant LOC Reduction:** Decreases total lines by 15+ lines

**Target:** ~271 lines reduced across 11 refactoring items.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── file_size.rs              # MODIFY: Consolidate human_size
│   ├── walker.rs                 # MODIFY: Extract build_walked_file
│   ├── walker_tests.rs           # MODIFY: Add test_walker_config helper
│   ├── git.rs                    # MODIFY: Extract collect_commits
│   ├── git_tests.rs              # MODIFY: Add Commit::test helper, git helpers
│   ├── runner.rs                 # MODIFY: Add CheckContext builder
│   ├── file_size_tests.rs        # MODIFY: Parameterize size tests
│   ├── report/
│   │   ├── mod.rs                # MODIFY: Extract create_formatter
│   │   ├── mod_tests.rs          # MODIFY: Add filter test helpers
│   │   └── test_support.rs       # CREATE: Formatter test helpers
│   └── checks/git/
│       └── mod_tests.rs          # MODIFY: Use Commit::test helper
└── plans/
    └── checkpoint-10c-refactor.md  # THIS FILE
```

## Dependencies

No new dependencies required. All refactoring is internal restructuring.

## Implementation Phases

### Phase 1: Production Code - HIGH Priority

Refactor production code with HIGH sync risk. These are critical because changes to data structures require updating multiple locations.

#### 1.1 Consolidate Human Size Formatting

**File:** `crates/cli/src/file_size.rs`

Merge `human_size()` (no space) and `human_bytes()` (with space) into a single function:

```rust
/// Format bytes as human-readable string.
pub fn human_size(bytes: u64, spaced: bool) -> String {
    let space = if spaced { " " } else { "" };
    if bytes >= 1_000_000 {
        format!("{:.1}{space}MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1}{space}KB", bytes as f64 / 1_000.0)
    } else {
        format!("{}{space}B", bytes)
    }
}
```

Update callers in `report/mod.rs` to use `human_size(bytes, true)`.

**LOC Change:** -10 lines

---

#### 1.2 Extract Walker File Builder

**File:** `crates/cli/src/walker.rs`

Extract duplicate `WalkedFile` construction (parallel and sequential paths):

```rust
fn build_walked_file(
    entry: &ignore::DirEntry,
    size: u64,
    meta: &std::io::Result<fs::Metadata>,
) -> WalkedFile {
    let (mtime_secs, mtime_nanos) = meta.as_ref().ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default())
        .map(|d| (d.as_secs(), d.subsec_nanos()))
        .unwrap_or((0, 0));

    WalkedFile {
        path: entry.path().to_path_buf(),
        size,
        mtime_secs,
        mtime_nanos,
        depth: entry.depth(),
        size_class: FileSizeClass::from_size(size),
    }
}
```

**LOC Change:** -18 lines

---

### Phase 2: Production Code - MEDIUM Priority

Refactor with MEDIUM sync risk or moderate LOC reduction.

#### 2.1 Extract Commit Iteration Helper

**File:** `crates/cli/src/git.rs`

Consolidate commit extraction loop in `get_commits_since` and `get_all_branch_commits`:

```rust
fn collect_commits(repo: &Repository, revwalk: git2::Revwalk) -> Result<Vec<Commit>> {
    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        commits.push(Commit {
            hash: oid.to_string()[..7].to_string(),
            message: commit.summary().unwrap_or("").to_string(),
        });
    }
    Ok(commits)
}
```

**LOC Change:** -5 lines

---

#### 2.2 Extract Formatter Factory

**File:** `crates/cli/src/report/mod.rs`

Consolidate formatter dispatch:

```rust
fn create_formatter(format: OutputFormat, compact: bool) -> Box<dyn ReportFormatter> {
    match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter::new(compact)),
        OutputFormat::Html => Box::new(HtmlFormatter),
        OutputFormat::Markdown => Box::new(MarkdownFormatter),
    }
}
```

**LOC Change:** -6 lines

---

#### 2.3 Extract CheckContext Builder

**File:** `crates/cli/src/runner.rs`

Add builder method to `RunnerConfig`:

```rust
impl RunnerConfig {
    fn build_context<'a>(
        &'a self,
        root: &'a Path,
        files: &'a [WalkedFile],
        config: &'a Config,
        violation_count: &'a AtomicUsize,
    ) -> CheckContext<'a> {
        CheckContext {
            root,
            files,
            config,
            limit: self.limit,
            violation_count,
            changed_files: self.changed_files.as_deref(),
            fix: self.fix,
            dry_run: self.dry_run,
            ci_mode: self.ci_mode,
            base_branch: self.base_branch.as_deref(),
            staged: self.staged,
        }
    }
}
```

**LOC Change:** -10 lines

---

### Phase 3: Unit Tests - HIGH Priority

Test code refactoring with high sync risk or significant LOC reduction.

#### 3.1 Add Commit Test Helper

**File:** `crates/cli/src/git.rs` (cfg(test) block)

Add test constructor to reduce 30+ construction sites:

```rust
#[cfg(test)]
impl Commit {
    pub fn test(hash: &str, message: &str) -> Self {
        Self {
            hash: hash.to_string(),
            message: message.to_string(),
        }
    }
}
```

Update all test files using `Commit { hash: "...".to_string(), message: "...".to_string() }`.

**LOC Change:** -60 lines

---

#### 3.2 Add Formatter Test Helpers

**File:** `crates/cli/src/report/test_support.rs` (NEW)

Create shared test support module:

```rust
//! Test support for report formatters.

use super::{Baseline, CheckFilter, ReportFormatter};

/// Assert buffered and streamed output match.
pub fn assert_buffered_matches_streamed<F: ReportFormatter>(
    formatter: &F,
    baseline: &Baseline,
    filter: &dyn CheckFilter,
) {
    let buffered = formatter.format(baseline, filter).unwrap();
    let mut streamed = Vec::new();
    formatter.format_to(&mut streamed, baseline, filter).unwrap();
    let streamed_str = String::from_utf8(streamed).unwrap();
    assert_eq!(buffered, streamed_str, "Buffered and streamed output should match");
}
```

Update `text_tests.rs`, `html_tests.rs`, `markdown_tests.rs` to use helper.

**LOC Change:** -27 lines

---

#### 3.3 Parameterize File Size Tests

**File:** `crates/cli/src/file_size_tests.rs`

Replace 12 near-identical test functions with parameterized test:

```rust
#[test]
fn file_size_classification() {
    let cases = [
        (1024, FileSizeClass::Small, "1KB"),
        (63 * 1024, FileSizeClass::Small, "63KB"),
        (MMAP_THRESHOLD - 1, FileSizeClass::Small, "just under threshold"),
        (MMAP_THRESHOLD, FileSizeClass::Normal, "at threshold"),
        (100 * 1024, FileSizeClass::Normal, "100KB"),
        (500 * 1024, FileSizeClass::Normal, "500KB"),
        (1024 * 1024, FileSizeClass::Oversized, "1MB"),
        (5 * 1024 * 1024, FileSizeClass::Oversized, "5MB"),
        (10 * 1024 * 1024, FileSizeClass::TooLarge, "10MB"),
        (100 * 1024 * 1024, FileSizeClass::TooLarge, "100MB"),
    ];

    for (size, expected, desc) in cases {
        assert_eq!(
            FileSizeClass::from_size(size),
            expected,
            "Failed for {} ({} bytes)", desc, size
        );
    }
}
```

**LOC Change:** -70 lines

---

### Phase 4: Unit Tests - MEDIUM Priority

Test helpers with moderate LOC reduction.

#### 4.1 Add Walker Config Helper

**File:** `crates/cli/src/walker_tests.rs`

```rust
fn test_walker_config() -> WalkerConfig {
    WalkerConfig {
        git_ignore: false,
        hidden: false,
        ..Default::default()
    }
}
```

**LOC Change:** -15 lines

---

#### 4.2 Add Git Test File Helper

**File:** `crates/cli/src/git_tests.rs`

```rust
fn create_and_stage(temp: &TempDir, filename: &str, content: &str) {
    std::fs::write(temp.path().join(filename), content).unwrap();
    git_add(temp, filename);
}
```

**LOC Change:** -20 lines

---

#### 4.3 Add Baseline Filter Test Helper

**File:** `crates/cli/src/report/mod_tests.rs`

```rust
fn assert_filter_excludes(
    baseline: &Baseline,
    excluded: &[&str],
    should_be_none: &[&str],
    should_be_some: &[&str],
) {
    let filter = ExcludeChecks(excluded.iter().map(|s| s.to_string()).collect());
    let filtered = FilteredMetrics::new(baseline, &filter);

    for metric in should_be_none {
        assert!(get_metric(&filtered, metric).is_none(),
            "{} should be None when excluding {:?}", metric, excluded);
    }
    for metric in should_be_some {
        assert!(get_metric(&filtered, metric).is_some(),
            "{} should be Some when excluding {:?}", metric, excluded);
    }
}
```

**LOC Change:** -30 lines

---

### Phase 5: Final Verification

Run full test suite and verify LOC reduction.

```bash
# Unit tests
cargo test -p quench

# Behavioral specs
cargo test --test specs

# Full CI check
make check

# Verify LOC reduction
git diff --stat HEAD~1  # Should show net negative lines
```

---

## Key Implementation Details

### Refactoring Strategy

Each refactoring follows the same pattern:
1. Identify duplicated code locations
2. Extract common logic into helper function
3. Update all call sites to use the helper
4. Verify tests pass
5. Check for LOC reduction

### Sync Risk Mitigation

The primary goal is reducing sync risk - places where the same logic is duplicated and one could be updated without the other. Examples:

| Before | Sync Risk | After |
|--------|-----------|-------|
| Two `human_size` functions | Change threshold in one, forget other | Single function with parameter |
| Duplicate `WalkedFile` construction | Add field, update one path | Single `build_walked_file` helper |
| `CheckContext` in two places | Add new field, miss one site | Builder method |

### Test Count Stability

Refactoring should not reduce test coverage:
- Same number of test assertions (just organized differently)
- Parameterized tests cover same cases as individual tests
- Test helpers extract assertion logic, not test scenarios

---

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test -p quench -- human_size` | Tests pass |
| 1 | `cargo test -p quench -- walker` | Tests pass |
| 2 | `cargo test -p quench -- git` | Tests pass |
| 2 | `cargo test -p quench -- report` | Tests pass |
| 2 | `cargo test -p quench -- runner` | Tests pass |
| 3-4 | `cargo test -p quench` | All unit tests pass |
| 5 | `make check` | Full CI suite passes |
| 5 | `git diff --stat` | Net negative LOC |

---

## Summary

| Phase | Items | Sync Risk | LOC Δ |
|-------|-------|-----------|-------|
| 1. Production HIGH | 2 | 2 HIGH | -28 |
| 2. Production MEDIUM | 3 | 2 HIGH, 1 MEDIUM | -21 |
| 3. Tests HIGH | 3 | 2 HIGH/MEDIUM, 1 LOC-only | -157 |
| 4. Tests MEDIUM | 3 | 3 LOC-focused | -65 |
| **Total** | **11** | | **-271** |

---

## Completion Criteria

- [ ] Phase 1: Human size and walker file builder consolidated
- [ ] Phase 2: Commit iteration, formatter factory, context builder extracted
- [ ] Phase 3: Commit test helper, formatter test helper, parameterized size tests
- [ ] Phase 4: Walker config, git file, and filter test helpers added
- [ ] Phase 5: `make check` passes, net LOC reduction verified
- [ ] `./done` executed successfully
