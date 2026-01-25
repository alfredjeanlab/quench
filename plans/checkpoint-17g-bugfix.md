# Checkpoint 17G: Bug Fixes - Performance

**Plan:** `checkpoint-17g-bugfix`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17F (Quick Wins - Performance)

## Overview

Post-optimization stabilization phase to address test gaps and edge cases introduced during checkpoint 17F performance quick wins. The 17F checkpoint implemented:

1. Arc-based cache violations (O(1) clone instead of O(n) deep clone)
2. Async cache persistence (background thread writes)
3. Pre-sized collections in runner
4. O(1) file lookup with HashSet

**Current State:**
- All 1117 lib tests pass
- All 511 spec tests pass
- `make check` passes completely
- Performance targets exceeded (124ms cold, 44ms warm)

**Issues Identified:**
1. No unit test for `persist_async` background thread completion
2. Ignored doctest for `persist_async` example code
3. No explicit test verifying Arc violation O(1) clone behavior
4. Pre-sized collection estimate may cause extra allocation on cold runs

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── cache.rs              # UPDATE: Fix doctest, add tests
│   ├── cache_tests.rs        # UPDATE: Add persist_async and Arc tests
│   └── runner.rs             # VERIFY: Pre-size behavior on cold runs
└── tests/
    └── specs/
        └── modes/
            └── cache.rs      # VERIFY: Cache integration tests
```

## Dependencies

None - internal fixes only.

## Implementation Phases

### Phase 1: Add Test for Async Cache Persistence

**Goal:** Verify the `persist_async` background thread completes successfully.

**Problem:** The `persist_async` method is used in production but has no dedicated unit test. The doctest is ignored.

**File:** `crates/cli/src/cache_tests.rs`

**Add test:**
```rust
#[test]
fn cache_persist_async_completes() {
    let dir = tempdir().unwrap();
    let cache_path = dir.path().join("cache.bin");
    let config_hash = 12345u64;

    // Create and populate cache
    let cache = FileCache::new(config_hash);
    let file_path = PathBuf::from("src/lib.rs");
    let key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 500,
        size: 1000,
    };
    cache.insert(file_path.clone(), key.clone(), vec![]);

    // Persist asynchronously and wait for completion
    let handle = cache.persist_async(cache_path.clone());
    handle.join().expect("thread panicked").expect("persist failed");

    // Verify file exists and can be restored
    assert!(cache_path.exists());
    let restored = FileCache::from_persistent(&cache_path, config_hash).unwrap();
    let result = restored.lookup(&file_path, &key);
    assert!(result.is_some());
}
```

**Verification:**
```bash
cargo test -p quench -- cache_persist_async
```

---

### Phase 2: Fix Ignored Doctest

**Goal:** Enable or properly document the ignored doctest for `persist_async`.

**Problem:** The doctest at `cache.rs:341` is marked `ignore` without explanation.

**File:** `crates/cli/src/cache.rs`

**Current:**
```rust
/// # Example
/// ```ignore
/// // Fire and forget - cache write happens in background
/// let _handle = cache.persist_async(cache_path);
/// // Process can exit while write is in progress
/// ```
```

**Fix:** Update doctest to be runnable or document why it must be ignored.

Option A - Make it runnable with proper setup:
```rust
/// # Example
/// ```
/// # use std::path::PathBuf;
/// # use tempfile::tempdir;
/// # use quench::cache::FileCache;
/// # let dir = tempdir().unwrap();
/// # let cache_path = dir.path().join("cache.bin");
/// # let cache = FileCache::new(0);
/// // Fire and forget - cache write happens in background
/// let handle = cache.persist_async(cache_path);
/// // Wait for completion (optional - process can exit without waiting)
/// handle.join().unwrap().unwrap();
/// ```
```

Option B - Document the ignore reason:
```rust
/// # Example
/// ```ignore
/// // This example is illustrative only - requires FileCache instance
/// let _handle = cache.persist_async(cache_path);
/// ```
```

**Decision:** Use Option A for better documentation coverage.

**Verification:**
```bash
cargo test --doc -- persist_async
```

---

### Phase 3: Add Test for Arc Violation Cloning

**Goal:** Verify that Arc-wrapped violations provide O(1) clone behavior.

**Problem:** The Arc optimization was added for performance, but there's no test verifying the behavior.

**File:** `crates/cli/src/cache_tests.rs`

**Add test:**
```rust
#[test]
fn cache_lookup_returns_arc_for_efficient_cloning() {
    use std::sync::Arc;

    let cache = FileCache::new(0);
    let path = PathBuf::from("src/main.rs");
    let key = FileCacheKey {
        mtime_secs: 100,
        mtime_nanos: 0,
        size: 50,
    };

    // Insert violations
    let violations = vec![CachedViolation {
        check: "test".to_string(),
        line: Some(1),
        violation_type: "test".to_string(),
        advice: "test".to_string(),
        value: None,
        threshold: None,
        pattern: None,
        lines: None,
        nonblank: None,
        target_path: None,
    }];
    cache.insert(path.clone(), key.clone(), violations);

    // Get two references - should be the same Arc (same pointer)
    let arc1 = cache.lookup(&path, &key).unwrap();
    let arc2 = cache.lookup(&path, &key).unwrap();

    // Verify both point to same underlying data (Arc::ptr_eq)
    assert!(Arc::ptr_eq(&arc1, &arc2));
}
```

**Verification:**
```bash
cargo test -p quench -- arc_for_efficient
```

---

### Phase 4: Verify Pre-sized Collection Behavior

**Goal:** Verify the pre-sized collection estimates work correctly for both cold and warm runs.

**Problem:** The `Vec::with_capacity(file_count / 10 + 1)` estimate assumes ~10% cache miss. On cold runs (100% miss), this will reallocate. Verify this is acceptable and doesn't cause issues.

**File:** `crates/cli/src/runner.rs`

**Analysis:**
```rust
// runner.rs:84-88
let file_count = files.len();
let mut cached_violations: HashMap<PathBuf, CachedViolationsArc> =
    HashMap::with_capacity(file_count);
let mut uncached_files: Vec<&WalkedFile> = Vec::with_capacity(file_count / 10 + 1);
```

The estimate is conservative for warm runs (expect ~10% miss) but undersized for cold runs (100% miss). This is acceptable because:

1. Cold runs are infrequent (first run, config change, version upgrade)
2. Vec reallocation is logarithmic (doubles capacity), so max ~4 reallocations
3. The optimization targets warm runs (the common case)
4. Memory overhead is minimal

**Action:** Document the rationale in a comment.

**Update `runner.rs`:**
```rust
// Pre-size for expected distribution (optimized for warm cache case)
// Cold runs will reallocate, but that's acceptable as they're infrequent
let file_count = files.len();
let mut cached_violations: HashMap<PathBuf, CachedViolationsArc> =
    HashMap::with_capacity(file_count);
// Expect ~10% cache miss on warm runs. Cold runs will reallocate.
let mut uncached_files: Vec<&WalkedFile> = Vec::with_capacity(file_count / 10 + 1);
```

**Verification:**
```bash
cargo test -p quench -- runner
```

---

### Phase 5: Run Full Test Suite and Performance Validation

**Goal:** Verify all changes pass tests and don't regress performance.

**Steps:**
1. Run full test suite
2. Run `make check`
3. Dogfood: run quench on quench
4. Verify performance hasn't regressed

**Verification:**
```bash
# Full test suite
cargo test --all

# CI checks
make check

# Dogfooding
cargo run -- check
cargo run -- check -o json | head -1  # Verify JSON output

# Performance sanity check (warm run should be <100ms)
time cargo run --release -- check tests/fixtures/bench-medium
```

---

### Phase 6: Bump CACHE_VERSION if Check Logic Changed

**Goal:** Verify cache version is appropriate for any changes made.

**Checklist:**
- [ ] Review if any check logic was modified
- [ ] If logic changed, bump `CACHE_VERSION` in `crates/cli/src/cache.rs`

**Current version:** 22

**Expected:** No bump needed - this checkpoint only adds tests and comments, no check logic changes.

**Verification:**
```bash
grep "pub const CACHE_VERSION" crates/cli/src/cache.rs
```

## Key Implementation Details

### Async Cache Persistence Safety

The `persist_async` method:
1. Clones all cache data before spawning thread (no shared state)
2. Uses atomic rename for crash safety
3. Creates parent directories if needed
4. Returns JoinHandle for optional waiting

The caller in `cmd_check.rs` waits for completion before exit:
```rust
// cmd_check.rs:542-546
if let Some(handle) = cache_handle
    && let Err(e) = handle.join().unwrap_or(Ok(()))
{
    tracing::warn!("failed to persist cache: {}", e);
}
```

### Arc Violation Pattern

The Arc wrapper provides:
- O(1) clone on cache hits (just refcount increment)
- Thread-safe shared access
- Automatic cleanup when all references dropped

Serialization/deserialization handles Arc transparently:
- `persist()`: Extracts Vec from Arc for serialization
- `from_persistent()`: Wraps Vec in Arc on load

### Pre-sized Collection Rationale

| Scenario | Cache Miss Rate | Reallocations |
|----------|----------------|---------------|
| Warm run | ~10% | 0-1 |
| Cold run | 100% | ~4 |
| Config change | 100% | ~4 |

The ~4 reallocations on cold runs add negligible overhead compared to the work of running checks on all files.

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test -p quench -- cache_persist_async` | Test passes |
| 2 | `cargo test --doc -- persist_async` | Doctest passes |
| 3 | `cargo test -p quench -- arc_for_efficient` | Test passes |
| 4 | `cargo test -p quench -- runner` | All runner tests pass |
| 5 | `make check` | All quality gates pass |
| 6 | `grep CACHE_VERSION crates/cli/src/cache.rs` | Version = 22 |

## Exit Criteria

- [ ] `persist_async` has dedicated unit test
- [ ] Doctest for `persist_async` is enabled or properly documented
- [ ] Arc violation cloning behavior is tested
- [ ] Pre-sized collection rationale is documented in comments
- [ ] `make check` passes
- [ ] Dogfooding passes: `quench check` on quench
- [ ] No performance regressions
