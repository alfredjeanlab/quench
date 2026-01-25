# Benchmarking Guide

## Quick Start

```bash
# Run dogfood benchmarks
./scripts/benchmark

# View detailed HTML reports
open target/criterion/report/index.html
```

## Performance Targets

From `docs/specs/20-performance.md`:

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Warm run | < 100ms | < 200ms | > 500ms |
| Cold run | < 500ms | < 1s | > 2s |

## Benchmark Suites

### Dogfood (`benches/dogfood.rs`)

Quench checking itself - the primary benchmark.

- `dogfood/fast` - Default mode with text output
- `dogfood/fast_json` - JSON output mode
- `dogfood_checks/*` - Individual check isolation

### Regression (`benches/regression.rs`)

Hard time limits that fail if exceeded:

- `cold_run_under_2s` - Cold run must complete <2s
- `warm_run_under_500ms` - Warm run must complete <500ms
- `cache_provides_speedup` - Cache must provide >=2x speedup

### Stress (`benches/stress.rs`)

Pathological inputs for edge case testing:

- Large files (10K-50K lines)
- Many `#[cfg(test)]` blocks
- Large workspaces (50 packages)
- Deep nesting (20 levels)

## Baseline Management

The baseline file `reports/benchmark-baseline.json` tracks expected performance:

```bash
# Update baseline after confirmed improvements
./scripts/update-baseline

# View current baseline
cat reports/benchmark-baseline.json | jq '.benchmarks'
```

## CI Integration

Benchmarks run on every PR:

1. **Regression tests** - Hard limits (fail if exceeded)
2. **Dogfood benchmarks** - Track against baseline

A 20% regression fails CI.

## Troubleshooting

### High Variance

If benchmarks show high variance (>20% stddev):

1. Close other applications
2. Disable CPU throttling: `sudo cpupower frequency-set --governor performance`
3. Run more samples: `cargo bench --bench dogfood -- --sample-size 100`

### Unexpected Slowdown

1. Check if cache is working: `ls -la .quench/cache.bin`
2. Profile: `cargo flamegraph -- check`
3. Compare against baseline: `./scripts/benchmark`

### Fixture Not Found

```bash
# Generate benchmark fixtures
./scripts/fixtures/generate-bench-fixtures
```
