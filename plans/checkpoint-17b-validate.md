# Checkpoint 17B: Performance Complete - Validation

**Plan:** `checkpoint-17b-validate`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17A (Pre-Checkpoint Fix)

## Overview

Validate all performance checkpoint criteria from the project outline. This checkpoint confirms that performance targets are met, edge cases are handled correctly, and all optimizations have profiling justification.

**Checkpoint Criteria:**
- [ ] Benchmark: cold run < 500ms on bench-medium (50K LOC)
- [ ] Benchmark: warm run < 100ms on bench-medium (50K LOC)
- [ ] Large file (>10MB) skipped with warning
- [ ] Cache invalidation works correctly
- [ ] No P1+ optimizations applied without profiling justification

**Deliverable:** `reports/checkpoint-17-performance.md`

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── dogfood.rs          # Self-check benchmarks
│   │   └── stress.rs           # Stress tests including bench-medium
│   └── src/
│       ├── cache.rs            # File-level caching (CACHE_VERSION=22)
│       ├── walker.rs           # Parallel file walking
│       └── checks/escapes/patterns.rs  # Pattern hierarchy
├── scripts/
│   ├── profile-repo            # Profiling helper
│   └── fixtures/
│       └── generate-stress-fixtures  # Fixture generator
├── tests/fixtures/
│   ├── stress-monorepo/        # 5K files, ~50K LOC (use as bench-medium)
│   └── bench-medium/           # NEW: Explicit 50K LOC fixture (if needed)
└── reports/
    └── checkpoint-17-performance.md  # NEW: Validation report
```

## Dependencies

No new dependencies. Uses existing infrastructure:
- `criterion = "0.5"` - Benchmarking
- `hyperfine` (optional) - CLI timing
- `/usr/bin/time -l` - Memory measurement

## Implementation Phases

### Phase 1: Create bench-medium Fixture

**Goal:** Establish a 50K LOC benchmark fixture with predictable structure.

**Analysis:** The `stress-monorepo` fixture has 5K files × ~10 LOC each ≈ 50K LOC. This can serve as `bench-medium` with a symlink, or we create a dedicated fixture.

**Option A: Symlink existing fixture (recommended)**
```bash
# stress-monorepo already has ~50K LOC
cd tests/fixtures
ln -s stress-monorepo bench-medium
```

**Option B: Dedicated fixture script (if more control needed)**
```bash
# Add to scripts/fixtures/generate-stress-fixtures

generate_bench_medium() {
    local dir="$FIXTURE_DIR/bench-medium"
    # 500 files × 100 LOC each = 50K LOC
    # ...
}
```

**Verification:**
```bash
# Confirm LOC count
./scripts/profile-repo tests/fixtures/stress-monorepo 2>&1 | head -5
# Or count directly
find tests/fixtures/stress-monorepo -name "*.rs" -exec cat {} + | wc -l
```

---

### Phase 2: Validate Cold/Warm Performance

**Goal:** Confirm cold < 500ms, warm < 100ms on bench-medium.

**Test procedure:**
```bash
# Build release binary
cargo build --release

# Cold run (cache cleared)
rm -rf tests/fixtures/stress-monorepo/.quench
time ./target/release/quench check tests/fixtures/stress-monorepo

# Warm run (cache populated)
time ./target/release/quench check tests/fixtures/stress-monorepo
```

**With hyperfine for statistical accuracy:**
```bash
# Cold run
hyperfine --warmup 0 --runs 5 \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'

# Warm run
hyperfine --warmup 1 --runs 10 \
    './target/release/quench check tests/fixtures/stress-monorepo'
```

**Expected results:**
| Mode | Target | Acceptable | Expected |
|------|--------|------------|----------|
| Cold | < 500ms | < 1s | ~200-400ms |
| Warm | < 100ms | < 200ms | ~50-80ms |

---

### Phase 3: Validate Large File Handling (>10MB)

**Goal:** Confirm files >10MB are skipped with a warning.

**Current state check:** Search codebase for >10MB handling:
```bash
grep -r "10.*MB\|MAX_FILE\|FileTooLarge" crates/cli/src/
```

**Create test fixture:**
```bash
# Create 15MB file
mkdir -p tests/fixtures/large-file-test/src
dd if=/dev/zero bs=1M count=15 | tr '\0' 'x' > tests/fixtures/large-file-test/src/huge.rs
echo 'fn main() {}' >> tests/fixtures/large-file-test/src/huge.rs
```

**Test scenarios:**

1. **Warning emitted for >10MB file:**
```bash
./target/release/quench check tests/fixtures/large-file-test 2>&1
# Should show: warning: skipping src/huge.rs (15MB > 10MB limit)
```

2. **File not processed (no violations from it):**
```bash
./target/release/quench check -o json tests/fixtures/large-file-test | jq '.files_skipped'
# Should include huge.rs
```

**Implementation verification (if not yet implemented):**
Check `walker.rs` or `runner.rs` for size check:
```rust
// Expected implementation in walker.rs or file reader
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

if meta.len() > MAX_FILE_SIZE {
    tracing::warn!("skipping {} ({} > 10MB limit)", path, human_size(meta.len()));
    return None; // Skip file
}
```

---

### Phase 4: Validate Cache Invalidation

**Goal:** Confirm cache invalidates correctly on all triggers.

**Invalidation triggers (from `cache.rs`):**
1. File mtime changed
2. File size changed
3. Config hash changed
4. Quench version changed
5. `CACHE_VERSION` bumped

**Test each trigger:**

**4.1 File modification:**
```bash
# Prime cache
./target/release/quench check tests/fixtures/rust

# Modify a file
touch tests/fixtures/rust/src/lib.rs

# Run again - should re-check modified file
./target/release/quench check -o json tests/fixtures/rust | jq '.cache'
# cache.misses should be > 0
```

**4.2 Config change:**
```bash
# Prime cache
./target/release/quench check tests/fixtures/rust

# Modify config
echo "# comment" >> tests/fixtures/rust/quench.toml

# Run again - should invalidate all
./target/release/quench check -o json tests/fixtures/rust | jq '.cache'
# cache.hits should be 0 (full invalidation)
```

**4.3 CACHE_VERSION change (code change simulation):**
```bash
# This happens automatically when check logic changes
# Verify by checking CACHE_VERSION in cache.rs
grep "CACHE_VERSION" crates/cli/src/cache.rs
# Current: v22 with documented reason
```

**Unit test verification:**
```bash
cargo test cache
# Should include tests for:
# - cache_invalidates_on_mtime_change
# - cache_invalidates_on_size_change
# - cache_invalidates_on_version_mismatch
```

---

### Phase 5: Validate P1-P4 Deferral Justification

**Goal:** Confirm no P1+ optimizations applied without profiling evidence.

**Review optimization status:**

| Priority | Optimization | Status | Justification |
|----------|-------------|--------|---------------|
| P0 | File caching | DONE | Core requirement for iterative use case |
| P1 | Walker tuning | DEFERRED | File discovery < 50% of total time |
| P2 | Pattern combining | DEFERRED | Pattern matching < 50% of total time |
| P3 | Memory limits | DEFERRED | Peak ~14.5MB << 100MB target |
| P4 | Micro-opts | DEFERRED | No specific bottleneck identified |

**Evidence files:**
- `reports/phase-1401-profile.md` - Profiling results
- `docs/specs/20-performance.md` - Performance model and thresholds

**Verification:**
```bash
# Check no P1-P4 optimizations in codebase
grep -r "lasso\|bumpalo\|moka\|smol_str" crates/cli/src/
# Should return empty (these are P4 micro-optimization crates)

# Confirm performance model assumptions hold
./scripts/profile-repo . 2>&1
# Time breakdown should match model expectations
```

---

### Phase 6: Generate Validation Report

**Goal:** Create `reports/checkpoint-17-performance.md` documenting validation results.

**Report structure:**
```markdown
# Checkpoint 17: Performance Complete

Date: YYYY-MM-DD
Commit: <hash>

## Summary

All checkpoint criteria validated and passing.

## Environment

- Hardware: <spec>
- OS: <version>
- Rust: <version>

## Checkpoint Criteria

### 1. Cold Run Performance (< 500ms on 50K LOC)

| Fixture | LOC | Target | Measured | Status |
|---------|-----|--------|----------|--------|
| stress-monorepo | ~50K | < 500ms | XXXms | PASS |

### 2. Warm Run Performance (< 100ms on 50K LOC)

| Fixture | LOC | Target | Measured | Status |
|---------|-----|--------|----------|--------|
| stress-monorepo | ~50K | < 100ms | XXXms | PASS |

### 3. Large File Handling (>10MB skipped with warning)

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| 15MB file skipped | warning emitted | <output> | PASS |
| File not processed | no violations | <output> | PASS |

### 4. Cache Invalidation

| Trigger | Test | Status |
|---------|------|--------|
| File mtime | touch + re-check | PASS |
| File size | modify + re-check | PASS |
| Config change | edit quench.toml | PASS |
| Version mismatch | unit tests | PASS |

### 5. Optimization Justification

| Optimization | Status | Evidence |
|-------------|--------|----------|
| P0: File caching | DONE | Core requirement |
| P1: Walker tuning | DEFERRED | < 50% discovery time |
| P2: Pattern combining | DEFERRED | < 50% matching time |
| P3: Memory limits | DEFERRED | Peak 14.5MB << 100MB |
| P4: Micro-opts | DEFERRED | No bottleneck identified |

## Conclusion

Checkpoint 17 validated. All performance targets met.
```

## Key Implementation Details

### Performance Model

From `docs/specs/20-performance.md`:
```
Total Time = File Discovery + File Reading + Pattern Matching + Aggregation
```

| Phase | % of Time | Strategy |
|-------|-----------|----------|
| File discovery | 30-50% | Parallel `ignore` crate |
| File reading | 20-30% | Size-gated, mmap >64KB |
| Pattern matching | 20-40% | Literal → Aho-Corasick → regex |
| Aggregation | <5% | Early termination |

### Cache Key Strategy

```rust
pub struct FileCacheKey {
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub size: u64,
}
```

### Large File Thresholds

| Size | Strategy |
|------|----------|
| < 64KB | Direct read |
| 64KB - 1MB | Memory-mapped |
| 1MB - 10MB | Memory-mapped, report oversized |
| > 10MB | Skip with warning |

## Verification Plan

### Quick Validation
```bash
# 1. Build
cargo build --release

# 2. Run profile script on stress fixture
./scripts/profile-repo tests/fixtures/stress-monorepo

# 3. Run cache tests
cargo test cache

# 4. Full test suite
make check
```

### Complete Validation
```bash
# Phase 1: Confirm fixture exists
ls tests/fixtures/stress-monorepo

# Phase 2: Cold/warm benchmarks
hyperfine --warmup 0 --runs 5 \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'

hyperfine --warmup 1 --runs 10 \
    './target/release/quench check tests/fixtures/stress-monorepo'

# Phase 3: Large file test
mkdir -p /tmp/large-file-test/src
dd if=/dev/zero bs=1M count=15 | tr '\0' 'a' > /tmp/large-file-test/src/huge.txt
./target/release/quench check /tmp/large-file-test 2>&1 | grep -i warn

# Phase 4: Cache invalidation
cargo test cache::tests

# Phase 5: Optimization audit
grep -r "lasso\|bumpalo\|moka" crates/cli/src/ || echo "No P4 optimizations found"

# Phase 6: Generate report
# Manual: write results to reports/checkpoint-17-performance.md
```

## Checklist

- [ ] stress-monorepo fixture exists (~50K LOC)
- [ ] Cold run < 500ms on stress-monorepo
- [ ] Warm run < 100ms on stress-monorepo
- [ ] Large file (>10MB) handling verified
- [ ] Cache invalidation tests pass
- [ ] No P1-P4 optimizations without justification
- [ ] `reports/checkpoint-17-performance.md` created
- [ ] `make check` passes
- [ ] Plan archived
