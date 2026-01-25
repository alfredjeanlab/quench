# Checkpoint 17: Performance Complete

Date: 2026-01-24
Commit: 638060f

## Summary

**5 of 6 checkpoint criteria validated.** One criterion fails (large file skip >10MB not implemented).

## Environment

- Hardware: Apple Silicon (M-series)
- OS: Darwin 25.2.0
- Rust: 1.92.0 (ded5c06cf 2025-12-08)

## Checkpoint Criteria

### 1. Cold Run Performance (< 500ms on 50K LOC)

| Fixture | LOC | Target | Measured | Status |
|---------|-----|--------|----------|--------|
| stress-monorepo | ~85K | < 500ms | 316.5ms ± 13.4ms | **PASS** |

Tested with hyperfine (5 runs, cache cleared before each):
```
Time (mean ± σ): 316.5 ms ± 13.4 ms [User: 96.7 ms, System: 451.3 ms]
Range (min … max): 302.2 ms … 330.2 ms
```

### 2. Warm Run Performance (< 100ms on 50K LOC)

| Fixture | LOC | Target | Measured | Status |
|---------|-----|--------|----------|--------|
| stress-monorepo | ~85K | < 100ms | 47.1ms ± 2.1ms | **PASS** |

Tested with hyperfine (1 warmup, 10 runs):
```
Time (mean ± σ): 47.1 ms ± 2.1 ms [User: 25.5 ms, System: 27.1 ms]
Range (min … max): 43.5 ms … 50.7 ms
```

### 3. Large File Handling (>10MB skipped with warning)

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| 15MB file skipped | Warning emitted, file not processed | File fully processed, line count violation reported | **FAIL** |

**Finding:** Files >10MB are NOT skipped with a warning as required by `docs/specs/20-performance.md` (lines 72-83). Instead, they are fully processed and report `file_too_large` violations based on line/token counts.

**Evidence:**
- `Error::FileTooLarge` type exists in `error.rs` but is never constructed
- No `MAX_FILE_SIZE` constant or size-gating logic found in walker or file reading code
- Tested with 15MB file: processed and reported 629,146 lines (vs 750 limit)

**Action Required:** Implement size-gated file reading per spec:
- Check file size from metadata before reading
- Hard limit: skip files > 10MB with warning
- Soft limit: report files > 1MB as potential violations

### 4. Cache Invalidation

| Trigger | Test | Status |
|---------|------|--------|
| File mtime | `touch` file + re-check | **PASS** |
| File size | Unit test `cache_miss_on_size_change` | **PASS** |
| Config change | Modify `max_lines` in quench.toml | **PASS** |
| Version mismatch | Unit test `cache_rejects_version_mismatch` | **PASS** |
| CACHE_VERSION | Documented at v22 in `cache.rs` | **PASS** |

All 13 unit tests pass. Integration tests `modified_file_causes_cache_miss` and `config_change_invalidates_cache` also pass.

### 5. Optimization Justification

| Optimization | Status | Evidence |
|-------------|--------|----------|
| P0: File caching | DONE | `cache.rs` using DashMap, 10x warm run speedup |
| P1: Walker tuning | DEFERRED | Cold < 500ms, no profiling bottleneck |
| P2: Pattern combining | DEFERRED | Warm < 100ms, three-tier hierarchy sufficient |
| P3: Memory limits | DEFERRED | Peak 14.5MB << 100MB target |
| P4: Micro-opts | DEFERRED | No specific bottleneck identified |

**Evidence:** `reports/phase-1401-profile.md` documents profiling results and deferral justification.

**No P4 micro-optimization crates** (lasso, bumpalo, moka, smol_str) in direct dependencies. Bumpalo appears only as transitive dependency from wasm-bindgen.

## Conclusion

**Checkpoint 17 partially validated.** 5 of 6 criteria pass.

### Blocking Issue

- **Large file (>10MB) skip with warning not implemented.** This is a spec requirement that must be addressed before checkpoint completion.

### Next Steps

1. Implement size-gated file reading in walker/file reader
2. Add `MAX_FILE_SIZE = 10 * 1024 * 1024` constant
3. Use existing `Error::FileTooLarge` type for warning
4. Add test fixture with >10MB file
5. Re-validate criterion 3

## Test Commands

```bash
# Cold run benchmark
hyperfine --warmup 0 --runs 5 -i \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'

# Warm run benchmark
hyperfine --warmup 1 --runs 10 -i \
    './target/release/quench check tests/fixtures/stress-monorepo'

# Cache tests
cargo test cache

# Full test suite
make check
```
