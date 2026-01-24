# Benchmark Milestone 1 Report

Date: 2026-01-23

## Summary

First performance benchmark milestone - measuring quench on itself (dogfooding).
All performance targets have been exceeded.

## Environment

- Hardware: Apple M3 Max
- OS: Darwin 25.2.0
- Rust: 1.92.0 (ded5c06cf 2025-12-08)
- quench commit: 378d701

## Results

### Dogfooding (quench on quench)

| Mode | Target | Measured | Status |
|------|--------|----------|--------|
| fast | <1s | 18.97ms | PASS |
| fast (json) | <1s | 18.98ms | PASS |
| ci | <5s | 3.20ms | PASS |

### Individual Checks

| Check | Target | Measured | Status |
|-------|--------|----------|--------|
| cloc | <200ms | 14.50ms | PASS |
| escapes | <200ms | 14.47ms | PASS |
| agents | <100ms | 14.21ms | PASS |

### Fixture Benchmarks

| Fixture | Files | Lines | Time |
|---------|-------|-------|------|
| bench-small | ~10 | ~500 | 9.77ms |
| bench-medium | ~50 | ~5k | 9.78ms |
| bench-large | ~200 | ~20k | 11.32ms |
| bench-deep | ~40 | ~1k | 10.19ms |
| bench-large-files | ~5 | ~5k | 9.53ms |

### CLI Baseline

| Operation | Measured |
|-----------|----------|
| cli_startup (--help) | 3.39ms |
| version_check (--version) | 6.72ms |

## Observations

1. **Exceptional Performance**: All benchmarks complete well under their targets.
   - Fast mode at ~19ms is 50x faster than the 1s target
   - CI mode at ~3.2ms is 1500x faster than the 5s target
   - Individual checks at ~14ms are 7-14x faster than targets

2. **Consistent Scaling**: Benchmark fixtures show minimal time increase from
   small (~10 files) to large (~200 files), indicating efficient file walking
   and parallel processing.

3. **CLI Overhead**: The ~3ms startup time shows minimal CLI initialization
   overhead, leaving room for actual work.

4. **No Bottlenecks Identified**: At these performance levels, there are no
   obvious optimization targets for this milestone.

## Baseline Established

Criterion baselines saved as `milestone-1` for regression tracking.

```bash
# To compare future runs against this baseline:
cargo bench --bench dogfood -- --baseline milestone-1
cargo bench --bench check -- --baseline milestone-1
cargo bench --bench baseline -- --baseline milestone-1
```

## Next Steps

1. Monitor for performance regressions as features are added
2. Consider adding memory profiling in future milestones
3. Add benchmarks for edge cases (very large files, many small files)
4. Consider CI integration for automated regression detection
