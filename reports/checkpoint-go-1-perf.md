# Go Adapter Performance Report

Generated: 2026-01-23
Hardware: Apple M3 Max, 36 GB RAM, macOS 26.2

## Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| go-simple warm | < 100ms | 9.8ms | PASS |
| go-multi warm | < 100ms | 10.2ms | PASS |
| golang/auto-detect warm | < 100ms | 9.6ms | PASS |
| GoAdapter::new() | < 100us | 19.9us | PASS |
| classify() per file | < 0.1us | 0.062us | PASS |
| enumerate_packages() go-simple | < 10ms | 0.106ms | PASS |
| enumerate_packages() go-multi | < 10ms | 0.141ms | PASS |
| enumerate_packages() golang | < 10ms | 0.445ms | PASS |
| parse_nolint 100 lines | < 10us | 7.23us | PASS |
| parse_nolint 1000 lines | < 100us | 69.9us | PASS |
| parse_go_mod simple | < 10us | 0.034us | PASS |
| parse_go_mod complex | < 10us | 0.043us | PASS |

**Overall: All targets met. No bottlenecks identified.**

## Detailed Results

### 1. End-to-End Benchmarks (hyperfine, 20 runs)

| Fixture | Files | Mean Time | Std Dev | Min | Max |
|---------|-------|-----------|---------|-----|-----|
| go-simple | 4 .go | 9.8ms | 0.3ms | 9.2ms | 10.3ms |
| go-multi | 8 .go | 10.2ms | 0.4ms | 9.5ms | 11.0ms |
| golang/auto-detect | ~24 .go | 9.6ms | 0.5ms | 9.0ms | 10.5ms |

All fixtures complete in < 12ms, well under the 100ms target.

### 2. Adapter Micro-Benchmarks (criterion)

#### Adapter Creation (GlobSet compilation)

| Adapter | Time | Patterns |
|---------|------|----------|
| GoAdapter::new() | 19.95us | 3 (source, test, ignore) |
| GenericAdapter::with_defaults() | 33.86us | variable |
| RustAdapter::new() | 59.66us | 6 (source, test, ignore, bench) |
| ShellAdapter::new() | 78.84us | 5-6 (source, test variants) |

**Go adapter creation is 3x faster than Shell and 3x faster than Rust** due to simpler pattern set (only 3 GlobSets).

#### File Classification (1000 files)

| Operation | Total Time | Per-File Time |
|-----------|------------|---------------|
| go_1k_source | 62.3us | 0.062us |
| go_1k_test | 45.2us | 0.045us |
| go_1k_vendor_ignored | 24.0us | 0.024us |

**Classification is extremely fast** - vendor paths are fastest because they match the ignore pattern early.

#### go.mod Parsing

| Input | Time |
|-------|------|
| simple (3 lines) | 33.9ns |
| complex (18 lines, multiple require blocks) | 43.3ns |

**go.mod parsing is essentially free** - simple string iteration, no regex.

#### Package Enumeration (filesystem traversal)

| Fixture | Packages Found | Time |
|---------|----------------|------|
| go-simple | 4 | 105.6us |
| go-multi | 5 | 141.3us |
| golang | ~18 | 444.9us |

**Package enumeration is I/O-bound** but still well under 1ms for all fixtures.

#### Nolint Directive Parsing

| Content | Lines | Time | Per-Line |
|---------|-------|------|----------|
| with_nolint_100_lines | 100 | 7.23us | 72ns |
| without_nolint_100_lines | 100 | 4.72us | 47ns |
| with_nolint_100_lines_pattern | 100 | 7.25us | 72ns |
| with_nolint_1000_lines | 1000 | 69.9us | 70ns |

**Nolint parsing scales linearly** - O(lines) complexity confirmed.

### 3. Comparison with Other Adapters

| Operation | Go | Rust | Shell |
|-----------|-----|------|-------|
| Adapter creation | 19.9us | 59.7us | 78.8us |
| classify() per file | 0.062us | 0.11us | 0.043us |
| Suppress parse/100 lines | 7.2us | 5.7us | 3.6us |

**Go adapter is faster than Shell for creation** (fewer patterns) but slightly slower for suppress parsing (nolint is more complex than shellcheck directives).

### 4. Bottlenecks Identified

**None.** All operations complete well under their target thresholds:
- No operations > 100ms (critical threshold)
- No operations > 10ms (investigation threshold)
- End-to-end checks complete in ~10ms

### 5. Performance Breakdown

Estimated time distribution for a typical go-multi check:

| Phase | Estimated Time | % of Total |
|-------|----------------|------------|
| Process startup | ~5ms | 50% |
| Adapter creation | 0.02ms | 0.2% |
| File discovery (ignore crate) | ~3ms | 30% |
| File classification | < 0.01ms | < 0.1% |
| Pattern matching (escapes) | ~1ms | 10% |
| Nolint parsing | < 0.1ms | < 1% |
| Output formatting | ~1ms | 10% |

**Most time is spent in process startup and file discovery** - the Go adapter itself adds negligible overhead.

## Recommendations

1. **No optimization needed** - All metrics are well within targets
2. **Pattern set is optimal** - 3 patterns (source, test, vendor) is minimal
3. **String parsing is efficient** - No regex in hot paths (go.mod, nolint basic check)
4. **File enumeration could be lazy** - Currently eager, but not a bottleneck

## Conclusion

The Go adapter performs excellently:
- **Creation time**: 19.9us (best among all adapters)
- **Classification**: 0.062us/file (comparable to Shell)
- **End-to-end**: ~10ms for small-to-medium fixtures

The adapter is production-ready with no performance concerns. The simplicity of Go's file organization (single .go extension, clear _test.go convention, vendor/ ignore) translates to efficient pattern matching and fast checks.

### Comparison to Baseline Targets

From checkpoint-5d-benchmarks.md baseline:

| Metric | Baseline Target | Go Actual | Delta |
|--------|----------------|-----------|-------|
| Adapter creation | ~60-80us | 19.9us | 3-4x faster |
| classify/file | 0.04-0.11us | 0.062us | On par |
| Suppress parse/100 | 3.6-5.7us | 7.2us | ~1.5x slower |

Go adapter creation is notably faster due to fewer patterns. Nolint parsing is slightly slower due to additional logic (code extraction, inline comment detection), but still well within acceptable bounds at 7.2us for 100 lines.
