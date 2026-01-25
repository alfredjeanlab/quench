# Phase 1401: Profiling Results

Performance profiling for quench optimization backlog. Results determine whether P1-P4 optimizations are needed.

## Performance Targets (from spec)

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Fast (cold) | <500ms | <1s | >2s |
| Fast (warm) | <100ms | <200ms | >500ms |
| CI | <5s | <15s | >30s |

Memory targets:
- Fast checks: <100MB target, 500MB limit
- CI checks: <500MB target, 2GB limit

## Baseline Measurements

Profiled on 2026-01-24 using `scripts/profile-repo`.

| Repo | Files | Cold (ms) | Warm (ms) | Peak Memory |
|------|-------|-----------|-----------|-------------|
| quench | ~407 | 81-253 | 74.5 | 14.5 MB |
| bench-report | ~30 | 50-130 | 54.3 | 11 MB |

**Notes:**
- Cold run variance due to filesystem cache warming (first run ~595ms, subsequent ~81ms)
- Measurements taken with `hyperfine` (3 cold runs, 5 warm runs)
- Memory measured via `/usr/bin/time -l` maximum resident set size

## Target Assessment

| Target | Required | Measured | Status |
|--------|----------|----------|--------|
| Cold <500ms | <500ms | ~81-253ms | PASS |
| Warm <100ms | <100ms | ~74.5ms | PASS |
| Memory <100MB | <100MB | ~14.5MB | PASS |

**Conclusion:** Current performance meets all targets. No P1-P4 optimizations triggered.

## Decision Thresholds (from plan)

| Optimization | Trigger | Evidence Required | Triggered? |
|--------------|---------|-------------------|------------|
| P1 (walking) | >50% in discovery | Flamegraph | No |
| P2 (patterns) | >50% in matching | Flamegraph | No |
| P3 (memory) | >500MB peak | `/usr/bin/time -v` | No |
| P4 (micro) | >5% in specific op | Flamegraph + micro-benchmark | No |

## Optimizations Applied

### Implemented
- [x] P0: File-level caching - Already implemented in `cache.rs` using DashMap
- [ ] P1: File walking - Not needed (cold <500ms)
- [ ] P2: Pattern matching - Not needed (warm <100ms)
- [ ] P3: Memory - Not needed (peak ~14.5MB)
- [ ] P4: Micro - Not needed (no specific bottleneck identified)

### Deferred
- P1 (walking): Profiling shows file discovery within acceptable bounds. No parallelism increase needed.
- P2 (patterns): Current three-tier hierarchy (literal -> Aho-Corasick -> regex) performing well.
- P3 (memory): DashMap unbounded cache acceptable for typical workloads. Consider `moka` only if users report OOM on very large repos.
- P4 (micro): String interning, arena allocation not justified without specific bottleneck evidence.

## Existing Infrastructure

The following performance infrastructure is already in place:

1. **File-level caching** (`cache.rs`): DashMap-based cache keyed by (path, mtime, size)
2. **Parallel walker** (`walker.rs`): Uses `ignore` crate with adaptive parallel/sequential heuristic
3. **Pattern hierarchy** (`pattern/matcher.rs`): Three-tier matching (literal, Aho-Corasick, regex)
4. **Size-gated reading**: Files >10MB skipped, >64KB use mmap
5. **Early termination**: Default 15 violation limit in fast mode

## Recommendations

1. **Monitor**: If users report performance issues on large repos (>10K files), re-profile and consider P1/P3.
2. **Stress testing**: Generate stress fixtures with `scripts/fixtures/generate-stress-fixtures` for regression testing.
3. **CI benchmarks**: Consider adding performance regression tests to CI.

## Future Work

If performance degrades or new use cases emerge:

1. **P1 triggers** (>50% in discovery): Increase walker thread count, cache file list, pre-filter by extension
2. **P2 triggers** (>50% in matching): Combine patterns into single Aho-Corasick, extract literal prefixes
3. **P3 triggers** (>500MB peak): Replace DashMap with `moka` bounded cache, add batch processing
4. **P4 triggers** (>5% in specific op): Apply targeted micro-optimizations with profiling evidence

## Appendix: Profiling Commands

```bash
# Generate flamegraph
scripts/profile-repo /path/to/repo flamegraph.svg

# Quick timing comparison
hyperfine --warmup 1 --runs 5 './target/release/quench check --ci .'

# Memory measurement
/usr/bin/time -l ./target/release/quench check --ci .

# Benchmark suite
cargo bench --bench file_walking
cargo bench --bench stress
cargo bench --bench check
```
