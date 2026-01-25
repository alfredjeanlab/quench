# Checkpoint 17A: Pre-Checkpoint Fix - Performance Complete

**Plan:** `checkpoint-17a-precheck`
**Root Feature:** `quench-performance`
**Depends On:** Phase 1401 (Performance - Optimization Backlog)

## Overview

Verify the performance infrastructure is complete and all targets are met. Phase 1401 established profiling workflows and documented that all performance targets are achieved without requiring P1-P4 optimizations. This checkpoint validates the implementation against specs and ensures benchmarking infrastructure is operational.

**Current State:**
- P0 file-level caching: Complete (`crates/cli/src/cache.rs`)
- Parallel walker: Complete (`crates/cli/src/walker.rs`)
- Pattern hierarchy: Complete (`crates/cli/src/checks/escapes/patterns.rs`)
- Size-gated reading: Complete (in walker/checker)
- Profiling workflow: Complete (`scripts/profile-repo`)
- Benchmark suite: Complete (`crates/cli/benches/`)
- Performance report: Complete (`reports/phase-1401-profile.md`)

**Goal:** Confirm all performance deliverables are complete and passing.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── adapter.rs         # Adapter benchmarks
│   │   ├── baseline.rs        # CLI startup benchmarks
│   │   ├── check.rs           # Check benchmarks
│   │   ├── docs.rs            # Docs check benchmarks
│   │   ├── dogfood.rs         # Self-check benchmarks (most important)
│   │   ├── file_walking.rs    # Walker benchmarks
│   │   ├── git.rs             # Git check benchmarks
│   │   ├── javascript.rs      # JS adapter benchmarks
│   │   ├── report.rs          # Report command benchmarks
│   │   ├── stress.rs          # Stress/edge-case benchmarks
│   │   └── tests.rs           # Tests check benchmarks
│   └── src/
│       ├── cache.rs           # File-level caching (P0)
│       ├── walker.rs          # Parallel file walking
│       └── checks/escapes/patterns.rs  # Pattern hierarchy
├── scripts/
│   ├── bench-ci               # CI benchmark script
│   ├── profile-repo           # Profiling helper script
│   └── fixtures/
│       └── generate-stress-fixtures  # Stress fixture generator
├── docs/
│   ├── profiling.md           # Profiling guide
│   └── specs/20-performance.md  # Performance specification
└── reports/
    └── phase-1401-profile.md  # Profiling results
```

## Dependencies

No new dependencies. Performance infrastructure uses:
- `criterion = "0.5"` - Benchmarking framework
- `dashmap = "6"` - Concurrent HashMap for caching
- `postcard` - Binary serialization for cache persistence
- `ignore = "0.4"` - Parallel gitignore-aware walking
- `rayon = "1.11"` - Data parallelism
- `memchr = "2.7"` - SIMD byte searching
- `aho-corasick = "1"` - Multi-pattern matching

## Implementation Phases

### Phase 1: Verify Performance Targets

**Goal:** Confirm current performance meets all targets from `docs/specs/20-performance.md`.

**Targets:**

| Mode | Target | Acceptable | Measured | Status |
|------|--------|------------|----------|--------|
| Fast (cold) | <500ms | <1s | ~81-253ms | PASS |
| Fast (warm) | <100ms | <200ms | ~74.5ms | PASS |
| CI | <5s | <15s | <1s | PASS |
| Memory (fast) | <100MB | 500MB | ~14.5MB | PASS |

**Verification:**
```bash
./scripts/profile-repo .
# Expected: Cold <500ms, Warm <100ms, Memory <100MB
```

---

### Phase 2: Verify Caching Infrastructure

**Goal:** Confirm file-level caching (P0) is complete and effective.

**Implementation checklist:**
- [x] `FileCache` struct with `DashMap` storage
- [x] `FileCacheKey` using mtime+size
- [x] `CachedViolation` for minimal storage
- [x] `CACHE_VERSION` for invalidation on logic changes
- [x] Config hash invalidation
- [x] Quench version invalidation
- [x] Cache statistics (hits, misses, entries)
- [x] Atomic file persistence via temp file

**Key code:** `crates/cli/src/cache.rs`

**Verification:**
```bash
# First run (cold)
rm -rf .quench && time cargo run --release -- check --ci .

# Second run (warm) - should be significantly faster
time cargo run --release -- check --ci .

# Check cache stats in JSON output
cargo run --release -- check --ci -o json . | jq .cache
```

---

### Phase 3: Verify Benchmark Suite

**Goal:** Confirm all benchmark files compile and run.

**Benchmark files:**

| Benchmark | Purpose |
|-----------|---------|
| `dogfood.rs` | Self-check performance (primary) |
| `baseline.rs` | CLI startup overhead |
| `file_walking.rs` | Walker performance |
| `check.rs` | Per-check performance |
| `stress.rs` | Edge-case handling |
| `git.rs` | Git check performance |
| `docs.rs` | Docs check performance |
| `adapter.rs` | Adapter classification |
| `javascript.rs` | JS adapter |
| `report.rs` | Report generation |
| `tests.rs` | Tests check |

**Verification:**
```bash
# Build all benchmarks (compile check)
cargo bench --no-run

# Run dogfood benchmarks (most important)
cargo bench --bench dogfood

# Run all benchmarks
cargo bench
```

---

### Phase 4: Verify Profiling Scripts

**Goal:** Confirm profiling workflow is operational.

**Scripts:**

| Script | Purpose |
|--------|---------|
| `scripts/profile-repo` | Run profiling with timing and memory |
| `scripts/bench-ci` | CI-friendly benchmark runner |

**Verification:**
```bash
# Profile quench on itself
./scripts/profile-repo .

# Run CI benchmarks
./scripts/bench-ci

# Save baseline for regression tracking
./scripts/bench-ci --save-baseline
```

---

### Phase 5: Verify Documentation

**Goal:** Confirm performance documentation is complete and accurate.

**Documentation files:**

| File | Content |
|------|---------|
| `docs/specs/20-performance.md` | Performance specification |
| `docs/profiling.md` | Profiling guide |
| `reports/phase-1401-profile.md` | Profiling results |

**Checklist:**
- [x] Performance targets documented
- [x] Performance model explained
- [x] Edge cases identified
- [x] Profiling commands documented
- [x] Benchmark commands documented
- [x] Optimization backlog defined
- [x] P0-P4 priority levels explained

**Verification:**
```bash
# Check docs exist and are non-empty
wc -l docs/specs/20-performance.md docs/profiling.md reports/phase-1401-profile.md
```

---

### Phase 6: Full Integration Testing

**Goal:** Run complete test suite and verify no regressions.

**Actions:**
1. Run unit tests for cache module
2. Run full make check
3. Verify benchmark suite compiles

**Verification:**
```bash
# Cache unit tests
cargo test cache

# Full suite
make check

# Benchmark compilation
cargo bench --no-run
```

## Key Implementation Details

### File Caching Strategy

The cache uses `(path, mtime, size)` as key for file-level invalidation:

```rust
pub struct FileCacheKey {
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub size: u64,
}
```

Invalidation triggers:
- File mtime changed
- File size changed
- Config hash changed (check settings)
- Quench version changed
- `CACHE_VERSION` bumped (check logic changed)

### Performance Model

From `docs/specs/20-performance.md`:

```
Total Time = File Discovery + File Reading + Pattern Matching + Aggregation
```

| Phase | % of Time | Strategy |
|-------|-----------|----------|
| File discovery | 30-50% | Parallel `ignore` crate walker |
| File reading | 20-30% | Size-gated, mmap for >64KB |
| Pattern matching | 20-40% | Literal -> Aho-Corasick -> regex |
| Aggregation | <5% | Early termination, bounded output |

### Optimization Status

| Priority | Optimization | Status | Reason |
|----------|-------------|--------|--------|
| P0 | File caching | Implemented | Core requirement |
| P1 | Walker tuning | Deferred | <50% in discovery |
| P2 | Pattern combining | Deferred | <50% in matching |
| P3 | Memory limits | Deferred | Peak ~14.5MB |
| P4 | Micro-opts | Deferred | No specific bottleneck |

## Verification Plan

### Performance Targets
```bash
./scripts/profile-repo .
# Cold: <500ms, Warm: <100ms, Memory: <100MB
```

### Benchmark Suite
```bash
# Quick check - compilation
cargo bench --no-run

# Full benchmark run
cargo bench --bench dogfood
```

### Unit Tests
```bash
cargo test cache
```

### Full Suite
```bash
make check
# Expected: All checks pass (fmt, clippy, test, build, audit, deny)
```

## Checklist

- [ ] `./scripts/profile-repo .` shows targets met
- [ ] `cargo bench --no-run` compiles all benchmarks
- [ ] `cargo bench --bench dogfood` runs successfully
- [ ] `cargo test cache` passes
- [ ] `docs/specs/20-performance.md` exists and is current
- [ ] `docs/profiling.md` exists and is current
- [ ] `reports/phase-1401-profile.md` documents results
- [ ] `make check` passes
- [ ] Plan archived after verification
