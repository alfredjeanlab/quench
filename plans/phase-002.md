# Phase 002: Benchmark Fixtures

**Root Feature:** `quench-2904`

## Overview

Create benchmark fixtures and performance tests for quench. This phase establishes the infrastructure needed to measure and track performance against the targets defined in `docs/specs/20-performance.md`:

- **Cold run**: <500ms target, <1s acceptable (50K LOC)
- **Warm run**: <100ms target, <200ms acceptable (50K LOC)

The fixtures cover different stress scenarios: file count, file size, and directory depth. Benchmarks measure both file discovery (walker) and the full check pipeline.

## Project Structure

```
quench/
├── tests/
│   └── fixtures/
│       ├── bench-small/        # 50 files, ~5K LOC (baseline)
│       ├── bench-medium/       # 500 files, ~50K LOC (target case)
│       ├── bench-large/        # 5K files, ~500K LOC (stress test)
│       ├── bench-deep/         # 1K files, 50+ levels deep
│       └── bench-large-files/  # 100 files, several >1MB
├── crates/
│   └── cli/
│       └── benches/
│           ├── baseline.rs     # (exists) CLI startup benchmarks
│           ├── file_walking.rs # File discovery benchmarks
│           └── check.rs        # Full check pipeline benchmarks
└── scripts/
    ├── gen-fixtures            # Fixture generation script
    └── bench-ci                # CI benchmark runner with tracking
```

## Dependencies

No new runtime dependencies required. The benchmarks use existing criterion setup from Phase 001.

### Dev Dependencies (no changes needed)

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

### Benchmark Configuration

Add to root `Cargo.toml`:

```toml
[[bench]]
name = "file_walking"
harness = false
path = "crates/cli/benches/file_walking.rs"

[[bench]]
name = "check"
harness = false
path = "crates/cli/benches/check.rs"
```

## Implementation Phases

### Phase 2.1: Fixture Generation Script

**Goal**: Create a script that generates deterministic, reproducible benchmark fixtures.

**Tasks**:
1. Create `scripts/gen-fixtures` shell script
2. Generate fixtures with realistic Rust file structure
3. Include `.gitignore` files to test ignore handling
4. Make fixtures deterministic (no timestamps, reproducible content)

**Script Design**:

```bash
#!/usr/bin/env bash
# scripts/gen-fixtures - Generate benchmark fixtures

# Fixture specifications:
# - bench-small:       50 files,    5K LOC,   flat structure
# - bench-medium:     500 files,   50K LOC,   3-level nesting
# - bench-large:     5000 files,  500K LOC,   5-level nesting
# - bench-deep:      1000 files,   50K LOC,  50+ levels deep
# - bench-large-files: 100 files, ~10MB total, includes 1MB+ files
```

**Generated File Content**:

Files will be realistic Rust code with:
- Module declarations
- Struct/enum definitions
- Function implementations
- Comments and documentation
- Varying line lengths (realistic distribution)

**Verification**:
```bash
./scripts/gen-fixtures
find tests/fixtures/bench-* -name "*.rs" | wc -l  # Verify file counts
wc -l tests/fixtures/bench-medium/**/*.rs         # Verify LOC
```

### Phase 2.2: Benchmark Fixture Creation

**Goal**: Generate all five benchmark fixtures.

**Tasks**:
1. Generate `bench-small/` (50 files, ~5K LOC)
2. Generate `bench-medium/` (500 files, ~50K LOC)
3. Generate `bench-large/` (5K files, ~500K LOC)
4. Generate `bench-deep/` (1K files, 50+ levels)
5. Generate `bench-large-files/` (100 files, several >1MB)
6. Add `.gitignore` patterns in each fixture

**Fixture Details**:

| Fixture | Files | LOC | Avg LOC/file | Structure |
|---------|-------|-----|--------------|-----------|
| bench-small | 50 | 5K | 100 | `src/{mod}.rs` flat |
| bench-medium | 500 | 50K | 100 | `src/{a..z}/{mod}.rs` 3-level |
| bench-large | 5,000 | 500K | 100 | `src/{a..z}/{0..9}/{mod}.rs` 5-level |
| bench-deep | 1,000 | 50K | 50 | `src/a/b/c/.../mod.rs` 50+ levels |
| bench-large-files | 100 | ~200K | 2K | Mix of normal + 5x 1MB files |

**Each fixture includes**:
- `Cargo.toml` (valid workspace member)
- `.gitignore` with common patterns (`target/`, etc.)
- `src/` directory with generated code
- `build/` directory with ignored files (tests ignore handling)

**Verification**:
```bash
# Verify file counts
find tests/fixtures/bench-small -name "*.rs" | wc -l   # ~50
find tests/fixtures/bench-medium -name "*.rs" | wc -l  # ~500
find tests/fixtures/bench-large -name "*.rs" | wc -l   # ~5000
find tests/fixtures/bench-deep -name "*.rs" | wc -l    # ~1000

# Verify LOC
wc -l tests/fixtures/bench-medium/src/**/*.rs | tail -1  # ~50K

# Verify depth
find tests/fixtures/bench-deep -type d | awk -F/ '{print NF}' | sort -n | tail -1  # 50+

# Verify large files
find tests/fixtures/bench-large-files -size +1M | wc -l  # ≥5
```

### Phase 2.3: File Walking Benchmarks

**Goal**: Benchmark file discovery performance in isolation.

**Tasks**:
1. Create `crates/cli/benches/file_walking.rs`
2. Benchmark parallel walker on each fixture
3. Measure with and without gitignore filtering
4. Track file count and timing separately

**Benchmark Code**:

```rust
//! File walking benchmarks.
//!
//! Measures file discovery performance using the `ignore` crate's
//! parallel walker. This isolates walker performance from file reading
//! and checking.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ignore::WalkBuilder;
use std::path::Path;

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures")
        .join(name)
}

fn walk_fixture(path: &Path) -> usize {
    let mut count = 0;
    for entry in WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .build()
    {
        if let Ok(entry) = entry {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                count += 1;
            }
        }
    }
    count
}

fn bench_file_walking(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_walking");

    for fixture in ["bench-small", "bench-medium", "bench-large", "bench-deep"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("walk", fixture),
            &path,
            |b, path| b.iter(|| walk_fixture(path)),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_file_walking);
criterion_main!(benches);
```

**Verification**:
```bash
cargo bench --bench file_walking
# Expected: bench-medium completes in <50ms
```

### Phase 2.4: Check Pipeline Benchmarks

**Goal**: Benchmark the full quench check pipeline end-to-end.

**Tasks**:
1. Create `crates/cli/benches/check.rs`
2. Benchmark `quench check` on each fixture (when check command exists)
3. Measure cold and warm run times separately
4. Add placeholders that activate as features are implemented

**Benchmark Code** (placeholder until check command exists):

```rust
//! Full check pipeline benchmarks.
//!
//! Measures end-to-end quench performance including:
//! - File walking
//! - File reading
//! - Pattern matching
//! - Output generation
//!
//! Note: Benchmarks are ignored until the check command is implemented.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::path::Path;
use std::process::Command;

fn fixture_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("tests/fixtures")
        .join(name)
}

fn bench_check_cold(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");
    let mut group = c.benchmark_group("check_cold");

    for fixture in ["bench-small", "bench-medium", "bench-large"] {
        let path = fixture_path(fixture);
        if !path.exists() {
            continue;
        }

        group.bench_with_input(
            BenchmarkId::new("check", fixture),
            &path,
            |b, path| {
                b.iter(|| {
                    Command::new(quench_bin)
                        .arg("check")
                        .current_dir(path)
                        .output()
                        .expect("quench check should run")
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_check_cold);
criterion_main!(benches);
```

**Verification**:
```bash
cargo bench --bench check
# Expected: bench-medium cold run <500ms
```

### Phase 2.5: CI Benchmark Tracking

**Goal**: Set up infrastructure to track benchmark regressions in CI.

**Tasks**:
1. Create `scripts/bench-ci` script for CI benchmark runs
2. Configure criterion to output JSON for tracking
3. Document benchmark baseline process
4. Add benchmark comparison to Makefile

**CI Script**:

```bash
#!/usr/bin/env bash
# scripts/bench-ci - Run benchmarks and track results
set -euo pipefail

BASELINE_DIR=".bench-baseline"
RESULTS_DIR="target/criterion"

# Run benchmarks with baseline comparison if available
if [[ -d "$BASELINE_DIR" ]]; then
    cargo bench --bench file_walking -- --baseline-lenient
    cargo bench --bench check -- --baseline-lenient
else
    echo "No baseline found. Run './scripts/bench-ci --save-baseline' to create one."
    cargo bench --bench file_walking
    cargo bench --bench check
fi

# Check for regressions (>20% slowdown)
if [[ -d "$RESULTS_DIR" ]]; then
    echo "Benchmark results saved to $RESULTS_DIR"
fi
```

**Makefile Addition**:

```makefile
# Run benchmarks
bench:
	cargo bench --bench baseline
	cargo bench --bench file_walking
	cargo bench --bench check

# Run benchmarks with CI tracking
bench-ci:
	./scripts/bench-ci
```

**Verification**:
```bash
./scripts/bench-ci
make bench
```

### Phase 2.6: Documentation and Validation

**Goal**: Document benchmark usage and validate all targets are achievable.

**Tasks**:
1. Update performance spec with actual baseline measurements
2. Document how to run and interpret benchmarks
3. Verify performance targets are realistic
4. Add benchmark information to project README (if exists)

**Validation Criteria**:

| Fixture | Walker Time | Check Time (Cold) | Status |
|---------|-------------|-------------------|--------|
| bench-small | <10ms | <50ms | Target |
| bench-medium | <50ms | <500ms | Target |
| bench-large | <500ms | <5s | Acceptable |
| bench-deep | <100ms | <1s | Target |
| bench-large-files | <20ms | <200ms | Target |

**Verification**:
```bash
make bench
# All benchmarks should complete
# bench-medium walker should be <50ms
```

## Key Implementation Details

### Fixture Generation Algorithm

Generate realistic Rust code with deterministic content:

```bash
generate_rust_file() {
    local lines=$1
    local seed=$2

    cat <<EOF
//! Module $seed
//!
//! Auto-generated benchmark fixture.

use std::collections::HashMap;

/// A sample struct for benchmarking
pub struct Sample$seed {
    pub id: u64,
    pub name: String,
    pub data: Vec<u8>,
}

impl Sample$seed {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            data: Vec::new(),
        }
    }

    // Generate additional lines to reach target
    $(generate_filler_lines $((lines - 20)))
}
EOF
}
```

### Directory Structure Generation

For `bench-deep`, use recursive directory creation:

```bash
# Generate 50+ level deep structure
path="tests/fixtures/bench-deep/src"
for i in $(seq 1 55); do
    path="$path/level_$i"
done
mkdir -p "$path"
```

For `bench-medium` and `bench-large`, use a balanced tree:

```bash
# 500 files across 26 directories (a-z) × ~20 files each
for dir in {a..z}; do
    mkdir -p "tests/fixtures/bench-medium/src/$dir"
    for i in $(seq 1 20); do
        generate_rust_file 100 "${dir}_$i" > "src/$dir/mod_$i.rs"
    done
done
```

### Large File Generation

For `bench-large-files`, generate files with realistic but verbose content:

```bash
# Generate a 1MB+ Rust file
generate_large_file() {
    local size=$1  # in KB
    local lines=$((size * 10))  # ~100 bytes per line

    echo "//! Large generated file (~${size}KB)"
    for i in $(seq 1 $lines); do
        echo "const LINE_$i: &str = \"$(printf 'x%.0s' {1..80})\";"
    done
}
```

### Benchmark Isolation

Ensure benchmarks measure the right thing:

```rust
// File walking only - no file reads
fn walk_fixture(path: &Path) -> usize {
    WalkBuilder::new(path)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .count()
}

// Full pipeline - includes reads and checks
fn check_fixture(path: &Path) -> std::process::Output {
    Command::new(quench_bin)
        .arg("check")
        .current_dir(path)
        .output()
        .unwrap()
}
```

## Verification Plan

### Phase Completion Checklist

- [ ] `./scripts/gen-fixtures` generates all fixtures
- [ ] `tests/fixtures/bench-small/` has ~50 files, ~5K LOC
- [ ] `tests/fixtures/bench-medium/` has ~500 files, ~50K LOC
- [ ] `tests/fixtures/bench-large/` has ~5K files, ~500K LOC
- [ ] `tests/fixtures/bench-deep/` has ~1K files, 50+ levels
- [ ] `tests/fixtures/bench-large-files/` has files >1MB
- [ ] `cargo bench --bench file_walking` runs successfully
- [ ] `cargo bench --bench check` runs (may skip if check not implemented)
- [ ] `make bench` runs all benchmarks
- [ ] `make check` still passes

### Performance Validation

After implementation, run benchmarks and record baseline:

```bash
# Generate fixtures
./scripts/gen-fixtures

# Run benchmarks
cargo bench --bench file_walking 2>&1 | tee bench-baseline.txt

# Verify performance targets
# bench-medium/walk should be <50ms
# (check benchmarks verified in later phases when check command exists)
```

### Fixture Integrity

```bash
# Verify fixtures exist and have expected sizes
./scripts/gen-fixtures --verify

# Expected output:
# bench-small:       52 files,    5,200 LOC ✓
# bench-medium:     500 files,   50,000 LOC ✓
# bench-large:    5,000 files,  500,000 LOC ✓
# bench-deep:     1,000 files,   50,000 LOC ✓
# bench-large-files: 100 files, 5 files >1MB ✓
```

### CI Integration

```bash
# Simulate CI benchmark run
./scripts/bench-ci

# Verify no regressions from baseline (when baseline exists)
```

## Summary

This phase creates the benchmark infrastructure needed to validate quench's performance against spec targets. The key deliverables are:

1. **Five benchmark fixtures** covering different stress scenarios
2. **File walking benchmarks** to measure discovery performance in isolation
3. **Check pipeline benchmarks** to measure end-to-end performance
4. **CI tracking infrastructure** to prevent performance regressions

The fixtures are deterministic and reproducible, generated by a script that can be re-run as needed. The benchmark suite integrates with criterion for statistical analysis and regression detection.
