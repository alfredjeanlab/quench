# Checkpoint 17F: Quick Wins - Performance

**Plan:** `checkpoint-17f-quickwins`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17E (Performance Profiling Infrastructure)

## Overview

Apply low-effort, high-impact performance optimizations identified during checkpoint 17E profiling. Focus on quick wins that reduce overhead without architectural changes.

**Current Performance (from 17E Baseline):**

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Cold run | < 500ms | 316.5ms | PASS |
| Warm run | < 100ms | 47.1ms | PASS |
| Memory | < 100MB | 14.5MB | PASS |

**Goal:** Further optimize warm runs (the common case) and reduce overhead in the hot paths. Target: 20-30% improvement in warm runs, negligible cold run regression.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── quickwins.rs          # NEW: Micro-benchmarks for quick wins
│   └── src/
│       ├── cache.rs              # ENHANCE: Async persistence, reduce cloning
│       ├── runner.rs             # ENHANCE: Reduce allocations, Arc for files
│       ├── walker.rs             # ENHANCE: Tune parallel threshold
│       └── checks/
│           └── escapes.rs        # ENHANCE: Pattern combining with Aho-Corasick
└── reports/
    └── profiling/
        └── quickwins-validation.md  # NEW: Before/after measurements
```

## Dependencies

**Existing (no changes):**
- `dashmap = "6.0"` - Concurrent cache
- `rayon = "1.10"` - Parallelism
- `postcard = "1.0"` - Cache serialization

**Optional additions (if profiling justifies):**
- `aho-corasick = "1.1"` - Multi-pattern matching (already in dependency tree via `ignore`)

No new runtime dependencies required.

## Implementation Phases

### Phase 1: Reduce Cache Cloning Overhead

**Goal:** Eliminate unnecessary cloning in hot paths.

**Problem:** `cache.rs:lookup()` clones the entire violations Vec on every hit:
```rust
// Current: clones on every lookup
return Some(entry.violations.clone());
```

For warm runs where most files are cached, this cloning is the dominant cost.

**Fix 1a:** Return reference instead of clone

Modify `crates/cli/src/cache.rs`:

```rust
/// Look up cached violations for a file.
///
/// Returns Some if the file has a valid cache entry (matching mtime+size).
/// The violations are borrowed from the cache.
pub fn lookup<'a>(&'a self, path: &Path, key: &FileCacheKey) -> Option<dashmap::mapref::one::Ref<'a, PathBuf, CachedFileResult>> {
    if let Some(entry) = self.inner.get(path) {
        if entry.key == *key {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(entry);
        }
    }
    self.misses.fetch_add(1, Ordering::Relaxed);
    None
}
```

**Fix 1b:** Update runner to use borrowed data

Modify `crates/cli/src/runner.rs` to work with references:

```rust
// Before: clones violations for cached files
cached_violations.insert(file.path.clone(), violations);

// After: store Ref or defer to iteration
// Key insight: we iterate violations once, don't need to store them
```

**Alternative approach:** Store violations behind `Arc<Vec<CachedViolation>>` to make cloning cheap (O(1) refcount increment instead of O(n) deep clone).

**Verification:**
```bash
cargo bench --bench cache -- warm
# Compare before/after cache lookup times
```

---

### Phase 2: Async Cache Persistence

**Goal:** Don't block on cache write when exiting.

**Problem:** Cache is persisted synchronously after checks complete:
```rust
// In cmd_check.rs
cache.persist(&cache_path)?;  // Blocks until write complete
```

For cold→warm transitions, this adds latency to the perceived cold run time.

**Fix:** Spawn background thread for cache write

Add to `crates/cli/src/cache.rs`:

```rust
use std::thread::JoinHandle;

impl FileCache {
    /// Persist cache to disk asynchronously.
    ///
    /// Returns a join handle that can be waited on, or ignored if caller
    /// doesn't care about completion.
    pub fn persist_async(&self, path: PathBuf) -> JoinHandle<Result<(), CacheError>> {
        // Clone data for the background thread
        let cache = PersistentCache {
            version: CACHE_VERSION,
            quench_version: self.quench_version.clone(),
            config_hash: self.config_hash,
            files: self
                .inner
                .iter()
                .map(|e| (e.key().clone(), e.value().clone()))
                .collect(),
        };

        std::thread::spawn(move || {
            let temp_path = path.with_extension("tmp");
            let bytes = postcard::to_allocvec(&cache)?;
            std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
            std::fs::write(&temp_path, &bytes)?;
            std::fs::rename(&temp_path, &path)?;
            Ok(())
        })
    }
}
```

**Update `cmd_check.rs`:**

```rust
// Fire and forget - cache write happens in background
let _cache_handle = cache.persist_async(cache_path);
// Don't wait for completion unless we need to guarantee persistence
```

**Note:** For correctness, we may want to wait in CI mode to ensure cache is persisted before process exits. Add parameter `wait: bool` to control.

**Verification:**
```bash
# Time cold run with cache write
hyperfine --warmup 0 --runs 3 \
    --prepare 'rm -rf tests/fixtures/bench-medium/.quench' \
    './target/release/quench check tests/fixtures/bench-medium'
```

---

### Phase 3: Reduce Runner Allocations

**Goal:** Eliminate per-run HashMap allocations in hot path.

**Problem:** Runner creates multiple HashMaps per run:
```rust
// runner.rs:81-82
let mut cached_violations: HashMap<PathBuf, Vec<CachedViolation>> = HashMap::new();
let mut uncached_files: Vec<&WalkedFile> = Vec::new();
```

For warm runs, this allocation overhead is significant relative to actual work.

**Fix 3a:** Pre-size collections based on file count

```rust
let file_count = files.len();
let mut cached_violations: HashMap<PathBuf, Vec<CachedViolation>> =
    HashMap::with_capacity(file_count);
let mut uncached_files: Vec<&WalkedFile> =
    Vec::with_capacity(file_count / 10); // Expect ~10% cache miss
```

**Fix 3b:** Avoid intermediate WalkedFile clone

Current code clones all uncached WalkedFiles:
```rust
// runner.rs:95-105 - clones for ownership
let uncached_owned: Vec<WalkedFile> = uncached_files
    .iter()
    .map(|f| WalkedFile { ... })
    .collect();
```

Instead, make CheckContext work with references:
```rust
pub struct CheckContext<'a> {
    // ...
    pub files: &'a [&'a WalkedFile],  // Slice of refs instead of slice of owned
}
```

Or use Arc<WalkedFile> from the start to make cloning free.

**Verification:**
```bash
cargo bench --bench cache -- warm
# Monitor allocations with dhat or heaptrack
```

---

### Phase 4: Walker Threshold Tuning

**Goal:** Optimize parallel/sequential threshold based on profiling data.

**Problem:** Current threshold is 1000 files (based on benchmarks). Profiling may show a different optimal value.

**Approach:**

1. Run profiling script on various fixture sizes:
```bash
for size in 100 500 1000 2000 5000; do
    ./scripts/perf/profile.sh tests/fixtures/bench-$size
done
```

2. Analyze breakeven point where parallel overhead equals sequential time

3. Update `walker.rs` threshold if profiling shows improvement:
```rust
/// Default threshold for switching from sequential to parallel walking.
/// Based on benchmarks: parallel overhead exceeds benefits below this threshold.
pub const DEFAULT_PARALLEL_THRESHOLD: usize = 500; // Tuned from 1000
```

**Additional optimization:** Add heuristic based on directory structure:
```rust
fn should_use_parallel(&self, root: &Path) -> bool {
    // Quick heuristic: check if common parallel-friendly indicators exist
    // (e.g., src/, lib/, packages/ directories suggesting multi-package repo)
    let has_deep_structure = root.join("src").is_dir()
        || root.join("packages").is_dir()
        || root.join("crates").is_dir();

    if has_deep_structure && !self.config.force_sequential {
        return true;
    }

    // Fall back to entry count heuristic
    // ...
}
```

**Verification:**
```bash
./scripts/perf/budget-check.sh
# Compare cold run times before/after threshold change
```

---

### Phase 5: Pattern Combining for Escapes Check

**Goal:** Use Aho-Corasick for multi-pattern matching in escape hatches check.

**Problem:** Escape patterns are matched individually:
```rust
// Pseudocode of current approach
for pattern in escape_patterns {
    if content.contains(pattern) {
        violations.push(...);
    }
}
```

For files with many escape patterns configured, this is O(patterns × content_length).

**Fix:** Combine literal patterns into single Aho-Corasick automaton:

```rust
use aho_corasick::AhoCorasick;

/// Pre-compiled escape pattern matcher.
pub struct EscapePatternMatcher {
    /// For literal patterns: single-pass multi-pattern matching
    literals: Option<AhoCorasick>,
    /// For regex patterns: fall back to individual matching
    regexes: Vec<(String, regex::Regex)>,
}

impl EscapePatternMatcher {
    pub fn new(patterns: &[EscapePattern]) -> Self {
        let literals: Vec<_> = patterns
            .iter()
            .filter(|p| p.is_literal())
            .map(|p| p.pattern.as_str())
            .collect();

        let regexes: Vec<_> = patterns
            .iter()
            .filter(|p| !p.is_literal())
            .map(|p| (p.name.clone(), p.compile_regex()))
            .collect();

        Self {
            literals: if literals.is_empty() {
                None
            } else {
                Some(AhoCorasick::new(&literals).unwrap())
            },
            regexes,
        }
    }

    pub fn find_matches<'a>(&'a self, content: &'a str) -> impl Iterator<Item = &'a str> {
        // Single pass for all literal patterns
        let literal_matches = self.literals.as_ref()
            .map(|ac| ac.find_iter(content).map(|m| &content[m.start()..m.end()]))
            .into_iter()
            .flatten();

        // Individual passes for regexes (can't be combined)
        let regex_matches = self.regexes.iter()
            .filter(|(_, r)| r.is_match(content))
            .map(|(name, _)| name.as_str());

        literal_matches.chain(regex_matches)
    }
}
```

**Note:** Only implement if profiling shows pattern matching is a bottleneck (>30% of check time).

**Verification:**
```bash
cargo bench --bench escapes -- pattern_matching
```

---

### Phase 6: Validation and Documentation

**Goal:** Verify improvements and document results.

**Create:** `reports/profiling/quickwins-validation.md`

```markdown
# Quick Wins Validation Report

Date: YYYY-MM-DD
Commit: XXXXXXX

## Summary

| Optimization | Expected Impact | Actual Impact |
|--------------|-----------------|---------------|
| Cache ref instead of clone | -20% warm | |
| Async cache persist | -10ms cold | |
| Pre-sized collections | -5% warm | |
| Walker threshold | ±5% cold | |
| Pattern combining | TBD | |

## Measurements

### Before (Checkpoint 17E)

| Metric | Value |
|--------|-------|
| Cold run | 316.5ms |
| Warm run | 47.1ms |
| Memory | 14.5MB |

### After (Checkpoint 17F)

| Metric | Value | Change |
|--------|-------|--------|
| Cold run | | |
| Warm run | | |
| Memory | | |

## Applied Optimizations

1. [List what was actually applied]

## Deferred Optimizations

1. [List what was deferred and why]
```

**Verification:**
```bash
# Full validation suite
make check
./scripts/perf/budget-check.sh
cargo bench -- --baseline checkpoint-17e

# Generate validation report
./scripts/perf/regression-check.sh
```

## Key Implementation Details

### Priority Order

Apply optimizations in order of impact/effort ratio:

1. **Cache cloning (P1)** - Highest impact on warm runs, easy change
2. **Runner allocations (P1)** - Compound effect with cache changes
3. **Async cache write (P2)** - Improves perceived cold run time
4. **Walker threshold (P2)** - Only if profiling shows walker bottleneck
5. **Pattern combining (P3)** - Only if escape check is a bottleneck

### Measurement Protocol

For each optimization:
1. Baseline: `cargo bench -- --save-baseline before-$OPT`
2. Apply change
3. Measure: `cargo bench -- --baseline before-$OPT`
4. Verify regression tests pass: `cargo test --bench regression`
5. Commit if improvement > 5%, revert if regression > 2%

### Non-Goals

These are explicitly **not** in scope for quick wins:

- Architectural changes (different check execution model)
- New dependencies for marginal gains
- Micro-optimizations without profiling evidence
- Changes that risk correctness for speed

## Verification Plan

### Phase 1 Verification
```bash
cargo test -p quench -- cache
cargo bench --bench cache -- lookup
```

### Phase 2 Verification
```bash
cargo test -p quench -- cache
# Verify cache file created after async write
./target/release/quench check tests/fixtures/bench-medium && \
    ls tests/fixtures/bench-medium/.quench/cache.bin
```

### Phase 3 Verification
```bash
cargo test -p quench -- runner
cargo bench --bench cache -- warm
```

### Phase 4 Verification
```bash
cargo test -p quench -- walker
./scripts/perf/budget-check.sh
```

### Phase 5 Verification
```bash
cargo test -p quench -- escapes
cargo bench --bench escapes 2>/dev/null || echo "No escapes bench yet"
```

### Phase 6 (Final) Verification
```bash
make check
./scripts/perf/budget-check.sh
cargo test --bench regression -- --nocapture
cat reports/profiling/quickwins-validation.md
```

## Exit Criteria

- [ ] Cache lookup returns references (no cloning on hit)
- [ ] Cache persistence is async (non-blocking on exit)
- [ ] Runner pre-sizes collections appropriately
- [ ] Walker threshold tuned if profiling justifies
- [ ] Pattern combining applied if escapes is bottleneck
- [ ] Warm run time improved ≥10% (target: <42ms from 47.1ms)
- [ ] Cold run time not regressed >5% (must stay <332ms)
- [ ] Memory usage not increased significantly
- [ ] All regression tests passing
- [ ] Validation report completed
- [ ] `make check` passes
