# Checkpoint 1D: Benchmark Analysis

Generated: 2026-01-22

## Summary

| Component | Target | Measured | Status |
|-----------|--------|----------|--------|
| CLI startup (--version) | <50ms | 3.2ms ± 0.2ms | ✓ |
| CLI startup (--help) | <50ms | 3.3ms ± 0.2ms | ✓ |
| Config discovery (present) | <10ms | 3.3ms ± 0.3ms | ✓ |
| Config discovery (absent) | <10ms | 3.3ms ± 0.3ms | ✓ |
| File walking (50K LOC) | <200ms | 1.48ms ± 0.09ms | ✓ |
| Full check (50K LOC cold) | <500ms | 19.2ms ± 0.9ms | ✓ |

**All performance targets met.** The CLI is significantly faster than specified targets across all metrics.

## Detailed Results

### Criterion Benchmarks

#### CLI Startup (baseline.rs)

| Benchmark | Time (mean) | Range |
|-----------|-------------|-------|
| cli_startup | 3.01ms | [2.99ms, 3.02ms] |
| version_check | 3.03ms | [3.01ms, 3.04ms] |

#### File Walking (file_walking.rs)

**Single-threaded:**

| Fixture | Files | Time (mean) | Files/sec |
|---------|-------|-------------|-----------|
| bench-small | 52 | 361µs | 144K |
| bench-medium | 530 | 1.48ms | 358K |
| bench-large | 5138 | 12.6ms | 408K |
| bench-deep | 1059 | 9.90ms | 107K |
| bench-large-files | 102 | 401µs | 254K |

**Parallel:**

| Fixture | Files | Time (mean) | Speedup vs Single |
|---------|-------|-------------|-------------------|
| bench-small | 52 | 3.11ms | 0.12x (overhead) |
| bench-medium | 530 | 3.65ms | 0.41x (overhead) |
| bench-large | 5138 | 8.63ms | 1.46x |
| bench-deep | 1059 | 9.61ms | 1.03x |
| bench-large-files | 102 | 3.03ms | 0.13x (overhead) |

**Observation:** Parallel walking only benefits large codebases (>5K files). For smaller codebases, thread spawning overhead exceeds benefits.

#### End-to-End Check (check.rs)

| Fixture | Files | LOC | Time (mean) | Status |
|---------|-------|-----|-------------|--------|
| bench-small | 52 | ~5K | 12.0ms | ✓ |
| bench-medium | 530 | ~50K | 19.2ms | ✓ |
| bench-large | 5138 | ~500K | 112.8ms | ✓ |
| bench-deep | 1059 | ~10K | 41.2ms | ✓ |
| bench-large-files | 102 | ~10MB | 14.3ms | ✓ |

### Hyperfine Measurements

Statistical measurements with 3 warmup runs:

```
CLI --version:     3.2ms ± 0.2ms  [User: 1.8ms, System: 1.0ms]
CLI --help:        3.3ms ± 0.2ms  [User: 1.8ms, System: 1.0ms]

Config (present):  3.3ms ± 0.3ms  [User: 1.8ms, System: 1.1ms]
Config (absent):   3.3ms ± 0.3ms  [User: 1.8ms, System: 1.1ms]

Check bench-small:       12.2ms ± 0.8ms  [User: 5.8ms, System: 6.9ms]
Check bench-medium:      19.2ms ± 0.9ms  [User: 10.0ms, System: 17.5ms]
Check bench-large:      111.9ms ± 2.1ms  [User: 46.7ms, System: 141.5ms]
Check bench-deep:        41.1ms ± 1.0ms  [User: 14.7ms, System: 42.0ms]
Check bench-large-files: 14.4ms ± 0.9ms  [User: 9.9ms, System: 8.9ms]
```

## Performance Model Validation

Expected vs measured time distribution for bench-medium (full check):

| Phase | Expected % | Observed Contribution |
|-------|------------|----------------------|
| CLI startup | ~15% | ~3ms of ~19ms |
| Config discovery | ~1% | <0.5ms |
| File discovery | 30-50% | ~3.7ms (parallel) |
| File reading + checks | 50-70% | ~12ms |
| Aggregation/output | <5% | <1ms |

The performance model is validated - file reading and checks dominate runtime as expected.

## Profiling Findings

No profiling was required as all targets were comfortably met. Key observations:

1. **CLI startup is efficient** - 3ms startup indicates minimal static initialization overhead from clap/tracing.

2. **Config discovery is fast** - No measurable difference between config present vs absent cases, both ~3.3ms (dominated by process startup).

3. **File walking scales linearly** - Single-threaded walker shows ~350K-400K files/sec throughput.

4. **Parallel overhead exists** - For small fixtures (<1K files), parallel walking is slower due to thread pool initialization.

## Bottlenecks Identified

**None requiring immediate action.** All metrics are well under target thresholds.

### Minor Observations (P2 - Future consideration)

1. **Parallel walking threshold**: Consider disabling parallel walking for <1000 files to avoid thread overhead.

2. **System time dominance**: For large checks, system time (I/O) exceeds user time, indicating I/O-bound workload. Future caching could help with warm runs.

## Recommendations

No performance optimizations required at this time. The implementation exceeds all targets by significant margins:

| Target | Margin |
|--------|--------|
| CLI startup (<50ms) | 15x faster |
| Config discovery (<10ms) | 3x faster |
| File walking (<200ms) | 135x faster |
| Full check (<500ms) | 26x faster |

## Environment

- **Platform:** Darwin 25.2.0 (macOS)
- **CPU:** Apple M3 Max
- **Architecture:** arm64
- **Rust version:** rustc 1.92.0 (ded5c06cf 2025-12-08)
- **Build profile:** release (LTO enabled)

## Fixtures Used

| Fixture | Files | Description |
|---------|-------|-------------|
| bench-small | 52 | 50 source files, ~5K LOC |
| bench-medium | 530 | 500 source files, ~50K LOC |
| bench-large | 5138 | 5000 source files, ~500K LOC |
| bench-deep | 1059 | 1000 files, 55+ directory levels |
| bench-large-files | 102 | 100 files including 5 files >1MB |

## Appendix: Raw Criterion Output

Baseline saved to `.bench-baseline` for future regression detection.

HTML reports available at `target/criterion/report/index.html`.
