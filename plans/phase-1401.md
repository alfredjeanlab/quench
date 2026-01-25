# Phase 1401: Performance - Optimization Backlog

**Root Feature:** `quench-a490`

## Overview

Apply P1-P4 performance optimizations from `docs/specs/20-performance.md` only when profiling justifies them. Core performance infrastructure (P0 caching, size-gated reading, timeouts) was implemented in earlier phases. This phase establishes profiling workflows and conditionally implements optimizations based on measured bottlenecks.

**Guiding principle:** Profile before optimizing. The bottleneck is rarely where you expect.

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── stress.rs              # EXISTING: Pathological cases
│   │   └── real_world.rs          # NEW: Large repo profiling
│   └── src/
│       ├── cache.rs               # EXISTING: DashMap cache
│       ├── walker.rs              # EXISTING: Parallel file walking
│       └── pattern/
│           └── matcher.rs         # EXISTING: Pattern hierarchy
├── scripts/
│   └── profile-repo              # NEW: Profiling helper script
└── reports/
    └── phase-1401-profile.md     # NEW: Profiling results and decisions
```

## Dependencies

**Already available:**
- `criterion = "0.5"` - Benchmarking framework
- `dashmap = "6"` - Concurrent HashMap (current cache)
- `ignore = "0.4"` - Parallel gitignore-aware walking
- `rayon = "1.11"` - Data parallelism
- `memchr = "2.7"` - SIMD byte searching
- `aho-corasick = "1"` - Multi-pattern matching

**Potentially needed (only if profiling justifies):**
- `moka = "0.12"` - Bounded concurrent cache with eviction (P3)

## Implementation Phases

### Phase 1: Establish Profiling Baseline

Profile quench on real-world large repositories to identify actual bottlenecks before implementing any optimizations.

**Profiling targets:**
1. A large open-source Rust project (e.g., ripgrep, rust-analyzer)
2. A monorepo with 10K+ files
3. The quench codebase itself (dogfooding)

**File:** `scripts/profile-repo`

```bash
#!/usr/bin/env bash
# Profile quench on a repository with flamegraph
set -euo pipefail

REPO="${1:-.}"
OUTPUT="${2:-flamegraph.svg}"

cargo build --release
flamegraph -o "$OUTPUT" -- ./target/release/quench check --ci "$REPO"

echo "Flamegraph written to $OUTPUT"
echo "Open in browser to analyze"
```

**Metrics to capture:**
- Total time breakdown: discovery_ms, checking_ms, output_ms
- Per-phase percentage of total time
- Cache hit rate on warm runs
- Peak memory usage (via `/usr/bin/time -v` or `hyperfine --show-output`)

**Verification:**
- `./scripts/profile-repo /path/to/large-repo` generates flamegraph
- `quench check --ci --timing` shows phase breakdown
- Results documented in `reports/phase-1401-profile.md`

### Phase 2: P1 - File Walking Optimizations (Conditional)

**Trigger:** Only implement if profiling shows >50% of time in file discovery.

**Current state:** `walker.rs` uses `ignore` crate with parallel/sequential heuristic (1000 file threshold).

**Optimization options (in order of impact):**

1. **Increase parallel thread count** - Currently uses default rayon pool
   ```rust
   WalkBuilder::new(root)
       .threads(num_cpus::get().max(4))  // Ensure minimum parallelism
       .build_parallel()
   ```

2. **Cache file list across runs** - Store discovered paths with mtime
   ```rust
   struct FileListCache {
       root_mtime: SystemTime,
       paths: Vec<(PathBuf, FileCacheKey)>,
   }
   ```

3. **Pre-filter by extension** - Skip non-matching extensions before gitignore
   ```rust
   walker.types(TypesBuilder::new()
       .add("rust", "*.rs")
       .add("go", "*.go")
       .build()?)
   ```

**File changes:**
- `crates/cli/src/walker.rs` - Add optimizations
- `crates/cli/src/cache.rs` - Extend cache for file list (if option 2)

**Verification:**
- Benchmark shows >20% improvement in discovery phase
- `cargo bench --bench file_walking` passes
- No regression in warm run performance

### Phase 3: P2 - Pattern Matching Optimizations (Conditional)

**Trigger:** Only implement if profiling shows >50% of time in pattern matching.

**Current state:** `pattern/matcher.rs` has three-tier hierarchy (literal → Aho-Corasick → regex). Patterns are compiled individually.

**Optimization options:**

1. **Combine multiple literal patterns** into single Aho-Corasick automaton
   ```rust
   // Before: Each pattern compiled separately
   let finders: Vec<CompiledPattern> = patterns.iter().map(compile).collect();

   // After: Combine literals into single automaton
   let (literals, complex): (Vec<_>, Vec<_>) = patterns
       .iter()
       .partition(|p| is_literal(p));

   let combined = AhoCorasick::builder()
       .build(literals.iter().map(|p| p.pattern()))
       .unwrap();
   ```

2. **Extract literal prefixes** for prefiltering
   ```rust
   // For regex like `unsafe\s*\{`, extract "unsafe" as prefilter
   fn extract_literal_prefix(pattern: &str) -> Option<&str> {
       // Return longest literal prefix before first metachar
   }
   ```

3. **Parallelize across patterns** (not just files)
   ```rust
   // If single large file with many patterns
   patterns.par_iter().flat_map(|p| p.find_all(content)).collect()
   ```

**File changes:**
- `crates/cli/src/pattern/matcher.rs` - Add combined automaton
- `crates/cli/src/pattern/mod.rs` - Update pattern compilation

**Verification:**
- `cargo bench --bench check` shows improvement on pattern-heavy fixtures
- No change in match correctness (same violations found)

### Phase 4: P3 - Memory Optimizations (Conditional)

**Trigger:** Only implement if profiling shows high memory usage (>500MB) or OOM on large repos.

**Current state:** `cache.rs` uses unbounded `DashMap`. No eviction policy.

**Optimization options:**

1. **Replace DashMap with moka** for bounded cache
   ```rust
   use moka::sync::Cache;

   let cache: Cache<PathBuf, CachedFileResult> = Cache::builder()
       .max_capacity(10_000)  // Limit entries
       .time_to_idle(Duration::from_secs(3600))  // Evict stale
       .build();
   ```

2. **Batch processing** for very large repos
   ```rust
   const BATCH_SIZE: usize = 1000;

   for batch in files.chunks(BATCH_SIZE) {
       let violations = batch.par_iter()
           .flat_map(|f| check_file(f))
           .collect::<Vec<_>>();
       emit_violations(&violations);
   }
   ```

3. **Reduce mmap threshold** to trade memory for I/O
   ```rust
   // Current: mmap files > 64KB
   // Reduce to: mmap files > 16KB (less resident memory)
   const MMAP_THRESHOLD: u64 = 16 * 1024;
   ```

**File changes:**
- `crates/cli/Cargo.toml` - Add `moka` dependency (if option 1)
- `crates/cli/src/cache.rs` - Replace DashMap with moka
- `crates/cli/src/runner.rs` - Add batch processing (if option 2)

**Verification:**
- Peak memory reduced by >30%
- No regression in cache hit rate
- `cargo bench --bench stress` passes memory constraints

### Phase 5: P4 - Micro-Optimizations (Conditional)

**Trigger:** Only implement if profiling identifies a specific, measurable bottleneck.

**Potential optimizations (implement only with evidence):**

| Optimization | Crate | When to Use |
|--------------|-------|-------------|
| String interning | `lasso` | >10% time in string allocation |
| Arena allocation | `bumpalo` | >10% time in per-file temp allocs |
| Small string opt | `smol_str` | Many short strings (<24 bytes) |
| PGO | rustc | Final release optimization |

**Example: String interning for repeated paths**
```rust
use lasso::{Rodeo, Spur};

struct InternedPaths {
    rodeo: Rodeo,
}

impl InternedPaths {
    fn intern(&mut self, path: &str) -> Spur {
        self.rodeo.get_or_intern(path)
    }
}
```

**Do NOT implement without profiling evidence:**
- "Seems like it would help" is not evidence
- Flamegraph must show >5% of time in the specific operation
- Micro-benchmark must confirm measurable improvement

**Verification:**
- Flamegraph shows reduction in target operation
- End-to-end benchmark shows net improvement
- Code complexity justified by measured gain

### Phase 6: Document Results and Update Spec

Capture all profiling results and optimization decisions.

**File:** `reports/phase-1401-profile.md`

```markdown
# Phase 1401: Profiling Results

## Baseline Measurements

| Repo | Files | Cold (ms) | Warm (ms) | Peak Memory |
|------|-------|-----------|-----------|-------------|
| quench | ~400 | X | X | X |
| ripgrep | ~500 | X | X | X |
| large-mono | 10K+ | X | X | X |

## Phase Breakdown (% of total)

| Repo | Discovery | Checking | Output |
|------|-----------|----------|--------|
| quench | X% | X% | X% |
| ripgrep | X% | X% | X% |
| large-mono | X% | X% | X% |

## Optimizations Applied

### Implemented
- [x/n] P1: File walking - Reason: ...
- [x/n] P2: Pattern matching - Reason: ...
- [x/n] P3: Memory - Reason: ...
- [x/n] P4: Micro - Reason: ...

### Deferred
- [ ] ... - Reason: profiling showed <X% impact

## Before/After Comparison

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Cold run (50K LOC) | X ms | X ms | X% |
| Warm run (50K LOC) | X ms | X ms | X% |
| Peak memory | X MB | X MB | X% |
```

**Verification:**
- Report completed with actual measurements
- Decisions documented with evidence
- Future work items captured for later phases

## Key Implementation Details

### Profiling Workflow

1. Build release binary: `cargo build --release`
2. Generate flamegraph: `flamegraph -- ./target/release/quench check --ci /path/to/repo`
3. Capture timing: `quench check --ci --timing -o json | jq .timing`
4. Capture memory: `/usr/bin/time -v quench check --ci /path/to/repo 2>&1 | grep "Maximum resident"`
5. Document in `reports/phase-1401-profile.md`

### Decision Thresholds

| Optimization | Trigger | Evidence Required |
|--------------|---------|-------------------|
| P1 (walking) | >50% in discovery | Flamegraph |
| P2 (patterns) | >50% in matching | Flamegraph |
| P3 (memory) | >500MB peak | `/usr/bin/time -v` |
| P4 (micro) | >5% in specific op | Flamegraph + micro-benchmark |

### Performance Targets (from spec)

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Fast (cold) | <500ms | <1s | >2s |
| Fast (warm) | <100ms | <200ms | >500ms |
| CI | <5s | <15s | >30s |

If current performance meets targets, optimizations may be deferred.

## Verification Plan

1. **Phase 1:** `./scripts/profile-repo .` generates flamegraph and timing data
2. **Phase 2:** If P1 triggered: `cargo bench --bench file_walking` shows improvement
3. **Phase 3:** If P2 triggered: `cargo bench --bench check` shows improvement
4. **Phase 4:** If P3 triggered: memory usage reduced, cache hit rate maintained
5. **Phase 5:** If P4 triggered: specific bottleneck eliminated
6. **Phase 6:** `reports/phase-1401-profile.md` complete with measurements

**Final verification:**
```bash
# Ensure no regressions
cargo bench --bench stress
make check

# Compare against baseline
quench check --ci --timing -o json | jq .timing
# Cold: <500ms, Warm: <100ms for typical projects
```
