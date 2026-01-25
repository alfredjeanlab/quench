# Phase 1503: DRY Refactoring - Production and Test Code

**Root Feature:** `quench-code-quality`
**Depends On:** Phase 1502 (Close Review Gaps)

## Overview

Address code duplication and DRY violations that meet one of these criteria:
1. **Sync Risk:** Duplicated logic that could get out of sync (bug if one is updated but not the other)
2. **Significant LOC Reduction:** Decreases total lines by 15+ lines
3. **Both**

Items that don't meet these criteria are deferred or removed.

## Justification Summary

| Item | Sync Risk | LOC Δ | Decision |
|------|-----------|-------|----------|
| **Production HIGH** | | | |
| 1.1 Human size formatting | HIGH: threshold logic in 2 places | -10 | ✅ KEEP |
| 1.2 Walker file builder | HIGH: field changes need 2 updates | -18 | ✅ KEEP |
| **Production MEDIUM** | | | |
| 2.1 Commit iteration | MEDIUM: hash/message logic in 2 places | -5 | ✅ KEEP (sync risk) |
| 2.2 Enum helper methods | LOW: compiler catches enum changes | +5 | ❌ REMOVE |
| 2.3 Workspace detection | LOW: languages have different needs | -10 | ❌ REMOVE |
| 2.4 Formatter factory | HIGH: new format needs 2 updates | -6 | ✅ KEEP (sync risk) |
| 2.5 CheckContext builder | HIGH: new field needs 2 updates | -10 | ✅ KEEP (sync risk) |
| 2.6 Report formatter macros | MEDIUM: but high complexity | ~0 | ❌ DEFER |
| **Tests HIGH** | | | |
| 3.1 Commit test helper | HIGH: struct change affects 30+ sites | -60 | ✅ KEEP |
| 3.2 Formatter test helpers | MEDIUM: assertion logic in 3 files | -27 | ✅ KEEP |
| 3.3 Parameterize file size tests | N/A (tests) | -70 | ✅ KEEP (LOC) |
| **Tests MEDIUM** | | | |
| 4.1 Walker config helper | LOW: test configs are independent | -15 | ✅ KEEP (borderline) |
| 4.2 Git file helper | LOW: simple 2-line pattern | -20 | ✅ KEEP (borderline) |
| 4.3 Baseline filter helper | LOW: test patterns are independent | -30 | ✅ KEEP (LOC) |

**Kept:** 11 items | **Removed:** 3 items | **Estimated LOC Reduction:** ~250 lines

---

## Implementation Phases

### Phase 1: Production Code - HIGH Priority (Sync Risk)

#### 1.1 Consolidate Human Size Formatting

| Criteria | Value |
|----------|-------|
| **Sync Risk** | HIGH - If size thresholds change (1000 vs 1024), both functions must update |
| **LOC Change** | -10 lines (remove duplicate, add 2-line parameter) |

**Problem:** Two functions with identical threshold logic but different spacing.

**Files:**
- `file_size.rs:54-65` - `human_size()` outputs `"1.5MB"`
- `report/mod.rs:232-242` - `human_bytes()` outputs `"1.5 MB"`

**Solution:** Consolidate into one function with spacing parameter.

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

**Verification:**
```bash
cargo test -p quench -- human_size
cargo test -p quench -- report
```

---

#### 1.2 Extract Walker File Builder

| Criteria | Value |
|----------|-------|
| **Sync Risk** | HIGH - WalkedFile has 6 fields; adding/changing any requires 2 updates |
| **LOC Change** | -18 lines (36 duplicated → 18 in helper + 2 call sites) |

**Problem:** Identical WalkedFile construction in parallel and sequential walker paths.

**File:** `crates/cli/src/walker.rs` (lines 307-326 and 398-417)

**Solution:** Extract helper function.

```rust
fn build_walked_file(
    entry: ignore::DirEntry,
    size: u64,
    meta: &std::io::Result<fs::Metadata>,
) -> WalkedFile {
    let (mtime_secs, mtime_nanos) = meta.as_ref().ok()
        .and_then(|m| m.modified().ok())
        .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default())
        .map(|d| (d.as_secs(), d.subsec_nanos()))
        .unwrap_or((0, 0));

    WalkedFile {
        path: entry.into_path(),
        size,
        mtime_secs,
        mtime_nanos,
        depth: entry.depth(),
        size_class: FileSizeClass::from_size(size),
    }
}
```

**Verification:**
```bash
cargo test -p quench -- walker
```

---

### Phase 2: Production Code - MEDIUM Priority (Sync Risk)

#### 2.1 Extract Commit Iteration Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | MEDIUM - Hash truncation (7 chars) or message handling in 2 places |
| **LOC Change** | -5 lines |

**Problem:** Identical commit extraction loop in `get_commits_since` and `get_all_branch_commits`.

**File:** `crates/cli/src/git.rs` (lines 91-102 and 124-131)

**Solution:** Extract helper.

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

**Verification:**
```bash
cargo test -p quench -- git
```

---

#### 2.2 Extract Formatter Factory

| Criteria | Value |
|----------|-------|
| **Sync Risk** | HIGH - Adding new output format requires updating 2 match statements |
| **LOC Change** | -6 lines |

**Problem:** Formatter dispatch duplicated in `format_report_with_options` and `format_report_to`.

**File:** `crates/cli/src/report/mod.rs` (lines 195-200 and 218-223)

**Solution:** Extract factory function.

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

**Verification:**
```bash
cargo test -p quench -- report
```

---

#### 2.3 Extract CheckContext Builder

| Criteria | Value |
|----------|-------|
| **Sync Risk** | HIGH - CheckContext has 10 fields; adding any requires 2 updates |
| **LOC Change** | -10 lines |

**Problem:** Nearly identical CheckContext construction in cached and uncached runner paths.

**File:** `crates/cli/src/runner.rs` (lines 140-152 and 264-276)

**Solution:** Add builder method.

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

**Verification:**
```bash
cargo test -p quench -- runner
```

---

### Phase 3: Unit Tests - HIGH Priority

#### 3.1 Add Commit Test Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | HIGH - Commit struct change affects 30+ construction sites |
| **LOC Change** | -60 lines (30 instances × 2 lines each → 1 helper + 30 one-liners) |

**Problem:** `Commit { hash: "...".to_string(), message: "...".to_string() }` repeated 30+ times.

**File:** `crates/cli/src/git.rs`

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

**Before/After:**
```rust
// Before (2 lines):
Commit { hash: "abc1234".to_string(), message: "feat: add feature".to_string() }

// After (1 line):
Commit::test("abc1234", "feat: add feature")
```

**Files to update:** `git_tests.rs`, `checks/git/mod_tests.rs`, `checks/git/docs_tests.rs`

**Verification:**
```bash
cargo test -p quench -- git
```

---

#### 3.2 Add Formatter Test Helpers

| Criteria | Value |
|----------|-------|
| **Sync Risk** | MEDIUM - Buffered vs streamed assertion logic in 3 files |
| **LOC Change** | -27 lines (9 lines × 3 files → 1 helper) |

**Problem:** Identical "buffered matches streamed" assertion in 3 formatter test files.

**File:** `crates/cli/src/report/test_support.rs`

```rust
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

**Files to update:** `report/text_tests.rs`, `report/html_tests.rs`, `report/markdown_tests.rs`

**Verification:**
```bash
cargo test -p quench -- report
```

---

#### 3.3 Parameterize File Size Tests

| Criteria | Value |
|----------|-------|
| **Sync Risk** | N/A (tests don't sync) |
| **LOC Change** | -70 lines (99 lines → 29 lines) |

**Problem:** 12 near-identical test functions for file size classification.

**File:** `crates/cli/src/file_size_tests.rs`

**Before (~99 lines):** 12 separate `#[test]` functions

**After (~29 lines):**
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

**Verification:**
```bash
cargo test -p quench -- file_size
```

---

### Phase 4: Unit Tests - MEDIUM Priority

#### 4.1 Add Walker Config Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Test configs are independent |
| **LOC Change** | -15 lines (6 instances × 4 lines → helper + 6 one-liners) |

**Decision:** KEEP - Borderline but improves readability.

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

**Verification:**
```bash
cargo test -p quench -- walker
```

---

#### 4.2 Add Git Test File Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Simple 2-line pattern |
| **LOC Change** | -20 lines (15 instances × 2 lines → helper + 15 one-liners) |

**Decision:** KEEP - Borderline but improves readability.

**File:** `crates/cli/src/git_tests.rs`

```rust
fn create_and_stage(temp: &TempDir, filename: &str, content: &str) {
    std::fs::write(temp.path().join(filename), content).unwrap();
    git_add(temp, filename);
}
```

**Verification:**
```bash
cargo test -p quench -- git
```

---

#### 4.3 Add Baseline Filter Test Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Test patterns are independent |
| **LOC Change** | -30 lines |

**Decision:** KEEP - Significant LOC reduction.

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

**Verification:**
```bash
cargo test -p quench -- filter
```

---

### Phase 5: Final Verification

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

## Removed Items (Did Not Meet Criteria)

### ❌ 2.2 Enum Helper Methods (REMOVED)

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Compiler catches missing match arms when enum changes |
| **LOC Change** | +5 lines (adds methods, doesn't reduce call sites significantly) |

**Reason:** No sync risk (compiler enforces exhaustive matching) and no LOC savings.

---

### ❌ 2.3 Workspace Detection Pattern (REMOVED)

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Rust and JS workspaces have legitimately different needs |
| **LOC Change** | -10 lines but adds ~30 lines of trait abstraction |

**Reason:** Structural similarity doesn't imply semantic coupling. Languages may diverge. Abstraction adds complexity without clear benefit.

---

### ❌ 2.6 Report Formatter Macros (DEFERRED)

| Criteria | Value |
|----------|-------|
| **Sync Risk** | MEDIUM - Metric changes need 3 macro updates |
| **LOC Change** | Uncertain - MetricsIter adds complexity |

**Reason:** High implementation complexity. Macros are rarely modified. Revisit if metrics iteration logic changes frequently.

---

## Summary

| Phase | Items | Sync Risk | LOC Δ |
|-------|-------|-----------|-------|
| 1. Production HIGH | 2 | 2 HIGH | -28 |
| 2. Production MEDIUM | 3 | 2 HIGH, 1 MEDIUM | -21 |
| 3. Tests HIGH | 3 | 2 HIGH/MEDIUM, 1 LOC-only | -157 |
| 4. Tests MEDIUM | 3 | 3 LOC-focused | -65 |
| **Total** | **11** | | **-271** |

## Exit Criteria

- [ ] All 11 items implemented
- [ ] Each change verified to reduce LOC or eliminate sync risk
- [ ] `make check` passes
- [ ] `git diff --stat` shows net negative lines changed
- [ ] No test count reduction (same coverage, less code)
