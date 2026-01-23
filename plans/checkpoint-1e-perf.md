# Checkpoint 1E: Performance Fixes - CLI Runs

**Root Feature:** `quench-409c`

## Overview

Review the benchmark report from Checkpoint 1D and apply targeted fixes for identified bottlenecks. The benchmark analysis shows all performance targets are met by significant margins (15x-135x faster than targets), so this checkpoint focuses on a single P2 optimization: disabling parallel file walking for small codebases where thread overhead exceeds benefits.

**Key finding:** Parallel walking is slower than single-threaded for codebases with <1000 files due to thread pool initialization overhead.

## Project Structure

```
quench/
├── crates/cli/
│   ├── src/
│   │   └── walker.rs          # Parallel file walker (modify)
│   └── benches/
│       └── file_walking.rs    # Walker benchmarks (verify)
├── reports/
│   └── checkpoint-1-benchmarks.md  # Update with before/after
└── tests/
    └── specs/walker_spec.rs   # Verify behavior unchanged
```

## Dependencies

No new dependencies required. All existing infrastructure sufficient.

## Implementation Phases

### Phase 1: Review Benchmark Report

**Goal:** Confirm the bottleneck and understand the optimization opportunity.

**Tasks:**
1. Read `reports/checkpoint-1-benchmarks.md` and identify P2 items
2. Confirm parallel walking overhead measurements:
   - bench-small (52 files): Parallel 3.11ms vs Single 361µs (0.12x)
   - bench-medium (530 files): Parallel 3.65ms vs Single 1.48ms (0.41x)
   - bench-large (5138 files): Parallel 8.63ms vs Single 12.6ms (1.46x speedup)
3. Determine threshold: ~1000 files is crossover point

**Verification:** Document the decision rationale for threshold selection.

---

### Phase 2: Implement Adaptive Walker

**Goal:** Add automatic fallback to sequential walking for small codebases.

**Location:** `crates/cli/src/walker.rs`

**Tasks:**
1. Add `parallel_threshold` field to `WalkerConfig`:
   ```rust
   /// Minimum file count estimate for parallel walking (default: 1000).
   /// Below this threshold, single-threaded walking is used.
   pub parallel_threshold: usize,
   ```

2. Add quick directory scan heuristic using `std::fs::read_dir` on top-level:
   ```rust
   fn should_use_parallel(&self, root: &Path) -> bool {
       // Quick heuristic: count top-level entries
       // If > threshold / 10, likely a large codebase
       let entry_count = std::fs::read_dir(root)
           .map(|entries| entries.count())
           .unwrap_or(0);

       entry_count >= self.config.parallel_threshold / 10
   }
   ```

3. Modify `walk()` to conditionally use `builder.build()` vs `builder.build_parallel()`:
   ```rust
   pub fn walk(&self, root: &Path) -> (Receiver<WalkedFile>, WalkHandle) {
       // ... existing setup ...

       if self.should_use_parallel(root) {
           // Use parallel walker for large codebases
           let walker = builder.build_parallel();
           // ... parallel implementation ...
       } else {
           // Use sequential walker for small codebases
           let walker = builder.build();
           // ... sequential implementation ...
       }
   }
   ```

4. Add explicit `force_parallel` and `force_sequential` config options for testing:
   ```rust
   /// Force parallel mode regardless of heuristic.
   pub force_parallel: bool,
   /// Force sequential mode regardless of heuristic.
   pub force_sequential: bool,
   ```

**Verification:** Unit tests pass, walker behavior unchanged for large codebases.

---

### Phase 3: Add Unit Tests

**Goal:** Test the adaptive behavior and threshold logic.

**Location:** `crates/cli/src/walker_tests.rs`

**Tasks:**
1. Add test for heuristic function:
   ```rust
   #[test]
   fn should_use_parallel_on_large_directory() {
       // Create temp dir with many files
       // Assert parallel mode selected
   }

   #[test]
   fn should_use_sequential_on_small_directory() {
       // Create temp dir with few files
       // Assert sequential mode selected
   }
   ```

2. Add test for force overrides:
   ```rust
   #[test]
   fn force_parallel_overrides_heuristic() { ... }

   #[test]
   fn force_sequential_overrides_heuristic() { ... }
   ```

3. Add test verifying identical results regardless of mode:
   ```rust
   #[test]
   fn parallel_and_sequential_produce_same_files() {
       // Walk same directory in both modes
       // Compare file lists (order may differ)
   }
   ```

**Verification:** `cargo test -p quench walker` passes.

---

### Phase 4: Re-run Benchmarks

**Goal:** Verify improvement on small/medium fixtures without regression on large.

**Tasks:**
1. Run file walking benchmarks:
   ```bash
   cargo bench --bench file_walking
   ```

2. Compare against saved baseline:
   ```bash
   ./scripts/bench-ci --compare
   ```

3. Expected improvements:
   - bench-small: ~3.1ms → ~0.4ms (8x improvement)
   - bench-medium: ~3.6ms → ~1.5ms (2.4x improvement)
   - bench-large: ~8.6ms → ~8.6ms (no regression)

4. Run end-to-end check benchmarks to verify overall impact:
   ```bash
   cargo bench --bench check
   ```

**Verification:** Small/medium fixtures show improvement, large fixture shows no regression.

---

### Phase 5: Update Benchmark Report

**Goal:** Document before/after measurements.

**Location:** `reports/checkpoint-1-benchmarks.md`

**Tasks:**
1. Add new section "Checkpoint 1E: Optimization Results":
   ```markdown
   ## Checkpoint 1E: Optimization Results

   **Applied:** Adaptive parallel/sequential walker threshold

   ### File Walking Improvements

   | Fixture | Before | After | Improvement |
   |---------|--------|-------|-------------|
   | bench-small | 3.11ms | Xms | Y% |
   | bench-medium | 3.65ms | Xms | Y% |
   | bench-large | 8.63ms | Xms | (baseline) |

   ### End-to-End Impact

   | Fixture | Before | After | Improvement |
   |---------|--------|-------|-------------|
   | bench-small | 12.0ms | Xms | Y% |
   | bench-medium | 19.2ms | Xms | Y% |
   ```

2. Update summary table status if any metrics improved significantly.

3. Mark P2 item as addressed.

**Verification:** Report updated with actual measurements.

---

### Phase 6: Final Verification

**Goal:** Ensure all quality gates pass.

**Tasks:**
1. Run full check suite:
   ```bash
   make check
   ```

2. Verify no behavior changes in spec tests:
   ```bash
   cargo test -p quench --test specs
   ```

3. Commit changes with spec list.

**Verification:** `make check` passes, all specs pass.

## Key Implementation Details

### Threshold Heuristic

The threshold is based on empirical measurements from Checkpoint 1D:

| File Count | Parallel | Sequential | Better |
|------------|----------|------------|--------|
| 52 | 3.11ms | 0.36ms | Sequential |
| 530 | 3.65ms | 1.48ms | Sequential |
| 1059 | 9.61ms | 9.90ms | ~Equal |
| 5138 | 8.63ms | 12.6ms | Parallel |

**Chosen threshold:** 1000 files
**Heuristic proxy:** Count top-level entries, multiply by ~10 as depth estimate

### Why Not Just Set Threads to 1?

Setting `threads(1)` on `WalkBuilder` still incurs thread pool overhead from rayon. The parallel walker spawns threads even with `threads(1)`. Only `build()` (non-parallel) avoids this entirely.

### Force Flags for Testing

The `force_parallel` and `force_sequential` options allow:
1. Benchmark both modes on same fixture
2. Test parallel mode in CI even on small test fixtures
3. User override if heuristic is wrong for specific workload

## Verification Plan

1. **Unit tests pass:** `cargo test -p quench walker` shows new tests passing
2. **Spec tests pass:** `cargo test -p quench --test specs` unchanged behavior
3. **Benchmarks improved:** Small/medium fixtures show measured improvement
4. **No regression:** Large fixture performance unchanged or better
5. **Report updated:** `reports/checkpoint-1-benchmarks.md` has before/after
6. **Quality gates:** `make check` passes all checks

### Success Criteria

- [ ] Adaptive threshold implemented in walker.rs
- [ ] Unit tests for threshold logic added
- [ ] Spec tests still pass (behavior unchanged)
- [ ] bench-small improved by >50%
- [ ] bench-medium improved by >30%
- [ ] bench-large no regression (within ±10%)
- [ ] Benchmark report updated with results
- [ ] `make check` passes
