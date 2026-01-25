# Phase 1503: DRY Refactoring - Remaining Items

**Root Feature:** `quench-884c`
**Previous Work:** ef8a240 (8 of 11 items completed)

## Overview

Complete remaining DRY refactoring items from the original plan.

**Completed in ef8a240:**
- 1.1 Human size formatting consolidation
- 1.2 Walker file builder extraction
- 2.1 Commit iteration helper
- 2.2 Formatter factory
- 2.3 CheckContext builder
- 3.1 Commit test helper (`test_commit` in checks/git/mod_tests.rs)
- 3.2 Formatter test helpers
- 4.1 Walker config helper

**Remaining:** 3 items (~120 LOC reduction potential)

---

## Remaining Items

### 3.3 Parameterize File Size Tests

| Criteria | Value |
|----------|-------|
| **Sync Risk** | N/A (tests don't sync) |
| **LOC Change** | -40 lines (consolidate 4 classification tests into 1) |

**Problem:** 4 near-identical test functions for file size classification.

**File:** `crates/cli/src/file_size_tests.rs`

**Current (~50 lines):** 4 separate classification tests

**After (~20 lines):**
```rust
#[test]
fn file_size_classification() {
    let cases = [
        // Small class
        (1024, FileSizeClass::Small, "1KB"),
        (63 * 1024, FileSizeClass::Small, "63KB"),
        (MMAP_THRESHOLD, FileSizeClass::Small, "at mmap threshold"),
        // Normal class
        (MMAP_THRESHOLD + 1, FileSizeClass::Normal, "just over mmap threshold"),
        (500 * 1024, FileSizeClass::Normal, "500KB"),
        (SOFT_LIMIT_SIZE, FileSizeClass::Normal, "at soft limit"),
        // Oversized class
        (SOFT_LIMIT_SIZE + 1, FileSizeClass::Oversized, "just over soft limit"),
        (5 * 1024 * 1024, FileSizeClass::Oversized, "5MB"),
        (MAX_FILE_SIZE, FileSizeClass::Oversized, "at max"),
        // TooLarge class
        (MAX_FILE_SIZE + 1, FileSizeClass::TooLarge, "just over max"),
        (15 * 1024 * 1024, FileSizeClass::TooLarge, "15MB"),
        (1024 * 1024 * 1024, FileSizeClass::TooLarge, "1GB"),
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

### 4.2 Add Git Test File Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Simple 2-line pattern |
| **LOC Change** | -20 lines (15 instances × 2 lines → helper + 15 one-liners) |

**Problem:** Repeated `fs::write` + `git_add` pattern in git_tests.rs.

**File:** `crates/cli/src/git_tests.rs`

**Add helper:**
```rust
/// Create a file and stage it.
fn create_and_stage(temp: &TempDir, filename: &str, content: &str) {
    std::fs::write(temp.path().join(filename), content).unwrap();
    git_add(temp, filename);
}
```

**Before:**
```rust
std::fs::write(temp.path().join("test.txt"), "content").unwrap();
git_add(&temp, "test.txt");
```

**After:**
```rust
create_and_stage(&temp, "test.txt", "content");
```

**Verification:**
```bash
cargo test -p quench -- git
```

---

### 4.3 Add Baseline Filter Test Helper

| Criteria | Value |
|----------|-------|
| **Sync Risk** | LOW - Test patterns are independent |
| **LOC Change** | -30 lines |

**Problem:** Repeated filter assertion pattern in report/mod_tests.rs.

**File:** `crates/cli/src/report/mod_tests.rs`

**Add helper:**
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

## Final Verification

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

## Summary

| Item | Sync Risk | LOC Δ |
|------|-----------|-------|
| 3.3 Parameterize file size tests | N/A | -40 |
| 4.2 Git test file helper | LOW | -20 |
| 4.3 Baseline filter test helper | LOW | -30 |
| **Total** | | **-90** |

## Exit Criteria

- [ ] All 3 items implemented
- [ ] `make check` passes
- [ ] `git diff --stat` shows net negative lines changed
- [ ] No test count reduction (same coverage, less code)
