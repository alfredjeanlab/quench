# Checkpoint 4E: Performance - Rust Adapter

**Root Feature:** `quench-4e5f`

## Overview

Performance hardening checkpoint for the Rust language adapter. Checkpoint 4D validated that the adapter meets all performance targets with significant margin (5x+ better than spec). This checkpoint focuses on:

1. **Stress testing** - Validate performance on pathological inputs
2. **Regression infrastructure** - Ensure performance gains are maintained
3. **Profiling documentation** - Record performance characteristics for maintainers
4. **Edge case handling** - Verify adapter handles large/pathological inputs gracefully

The 4D benchmark report established these baselines:

| Metric | Target | Actual (4D) | Margin |
|--------|--------|-------------|--------|
| Cold (50K LOC) | < 500ms | 91ms | 5.5x |
| Warm (50K LOC) | < 100ms | 19ms | 5.3x |
| Adapter overhead | < 10% | -13% (faster) | N/A |

**Key question:** Does the adapter maintain performance under stress conditions (large files, deep nesting, many packages)?

## Project Structure

Key files involved:

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── adapter.rs            # Micro-benchmarks (from 4D)
│   │   └── stress.rs             # Stress benchmarks (new)
│   └── src/adapter/rust/
│       ├── mod.rs                # Core adapter
│       ├── cfg_test.rs           # #[cfg(test)] parser
│       ├── workspace.rs          # Cargo workspace parser
│       └── suppress.rs           # Suppress attribute parser
├── tests/fixtures/
│   ├── bench-rust/               # 510 files, 50K LOC (from 4D)
│   └── stress-rust/              # Stress test fixtures (new)
│       ├── large-files/          # Single files 10K-100K lines
│       ├── many-packages/        # 50+ package workspace
│       └── deep-nesting/         # 20+ nested mod files
├── reports/
│   ├── checkpoint-4d-benchmarks.md  # Baseline (existing)
│   └── checkpoint-4e-stress.md      # Stress test results (new)
└── scripts/
    └── gen-stress-fixture        # Stress fixture generation (new)
```

## Dependencies

**Existing (no new dependencies):**
- `criterion` - Benchmarking framework
- `hyperfine` - CLI benchmarking (installed)
- `globset` - Pattern matching

**Optional profiling tools:**
```bash
# If not already installed
cargo install flamegraph
cargo install cargo-instruments  # macOS only
```

## Implementation Phases

### Phase 1: Create Stress Test Fixtures

**Goal:** Generate fixtures that exercise edge cases from `docs/specs/20-performance.md`.

**Stress scenarios to test:**

| Scenario | Fixture | Characteristics |
|----------|---------|-----------------|
| Large files | `stress-rust/large-files/` | Files with 10K, 50K, 100K lines |
| Many #[cfg(test)] blocks | `stress-rust/many-cfg-test/` | 50+ inline test blocks per file |
| Large workspace | `stress-rust/many-packages/` | 50 packages, 1000 total files |
| Deep module nesting | `stress-rust/deep-nesting/` | 20+ levels of mod.rs |
| Many suppress attrs | `stress-rust/many-suppresses/` | 100+ #[allow] per file |

**Script:** `scripts/gen-stress-fixture`

```bash
#!/usr/bin/env bash
set -euo pipefail

FIXTURE_DIR="${1:-tests/fixtures/stress-rust}"
mkdir -p "$FIXTURE_DIR"

# === Large Files ===
mkdir -p "$FIXTURE_DIR/large-files/src"
cat > "$FIXTURE_DIR/large-files/Cargo.toml" << 'TOML'
[package]
name = "large-files"
version = "0.1.0"
edition = "2021"
TOML

# 10K line file
for lines in 10000 50000; do
    FILE="$FIXTURE_DIR/large-files/src/file_${lines}.rs"
    echo "//! File with $lines lines" > "$FILE"
    for i in $(seq 1 $((lines / 10))); do
        cat >> "$FILE" << 'RUST'
pub fn func() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
}

#[cfg(teilcode)]
mod tests {
    #[test]
    fn test() {}
}
RUST
    done
done

# Create lib.rs
echo "pub mod file_10000;" > "$FIXTURE_DIR/large-files/src/lib.rs"
echo "pub mod file_50000;" >> "$FIXTURE_DIR/large-files/src/lib.rs"

# === Many #[cfg(test)] blocks ===
mkdir -p "$FIXTURE_DIR/many-cfg-test/src"
cat > "$FIXTURE_DIR/many-cfg-test/Cargo.toml" << 'TOML'
[package]
name = "many-cfg-test"
version = "0.1.0"
edition = "2021"
TOML

FILE="$FIXTURE_DIR/many-cfg-test/src/lib.rs"
echo "//! File with many #[cfg(test)] blocks" > "$FILE"
for i in $(seq 1 50); do
    cat >> "$FILE" << RUST

pub fn func_$i() -> i32 { $i }

#[cfg(test)]
mod tests_$i {
    use super::*;
    #[test]
    fn test_func_$i() {
        assert_eq!(func_$i(), $i);
    }
}
RUST
done

# === Large Workspace (50 packages) ===
mkdir -p "$FIXTURE_DIR/many-packages/crates"
cat > "$FIXTURE_DIR/many-packages/Cargo.toml" << 'TOML'
[workspace]
members = ["crates/*"]
resolver = "2"
TOML

for pkg in $(seq 1 50); do
    PKG_DIR="$FIXTURE_DIR/many-packages/crates/pkg_$pkg"
    mkdir -p "$PKG_DIR/src"
    cat > "$PKG_DIR/Cargo.toml" << TOML
[package]
name = "pkg_$pkg"
version = "0.1.0"
edition = "2021"
TOML
    # 20 source files per package = 1000 total
    for f in $(seq 1 20); do
        cat > "$PKG_DIR/src/mod_$f.rs" << RUST
pub fn func() -> i32 { $f }

#[cfg(test)]
mod tests {
    #[test]
    fn test() { assert_eq!(super::func(), $f); }
}
RUST
    done
    # lib.rs
    echo "//! Package $pkg" > "$PKG_DIR/src/lib.rs"
    for f in $(seq 1 20); do
        echo "pub mod mod_$f;" >> "$PKG_DIR/src/lib.rs"
    done
done

# === Deep Nesting ===
mkdir -p "$FIXTURE_DIR/deep-nesting/src"
cat > "$FIXTURE_DIR/deep-nesting/Cargo.toml" << 'TOML'
[package]
name = "deep-nesting"
version = "0.1.0"
edition = "2021"
TOML

# Create 20 levels of nested modules
CURRENT="$FIXTURE_DIR/deep-nesting/src"
echo "pub mod level_1;" > "$CURRENT/lib.rs"
for level in $(seq 1 20); do
    NEXT="$CURRENT/level_$level"
    mkdir -p "$NEXT"
    if [ $level -lt 20 ]; then
        echo "pub mod level_$((level + 1));" > "$NEXT/mod.rs"
    else
        echo "pub fn deepest() -> i32 { $level }" > "$NEXT/mod.rs"
    fi
    CURRENT="$NEXT"
done

# === quench.toml for each ===
for dir in large-files many-cfg-test many-packages deep-nesting; do
    cat > "$FIXTURE_DIR/$dir/quench.toml" << 'TOML'
[check.cloc]
check = "warn"
max_lines = 100000

[check.escapes]
check = "warn"
TOML
done

echo "Generated stress fixtures at $FIXTURE_DIR"
find "$FIXTURE_DIR" -name '*.rs' | wc -l
```

**Verification:**
```bash
chmod +x scripts/gen-stress-fixture
./scripts/gen-stress-fixture tests/fixtures/stress-rust
ls tests/fixtures/stress-rust/
```

**Milestone:** Stress fixtures exist with expected structure.

**Status:** [ ] Pending

---

### Phase 2: Add Stress Benchmarks

**Goal:** Create `crates/cli/benches/stress.rs` with benchmarks targeting edge cases.

**New benchmark file:**

```rust
//! Stress benchmarks for Rust adapter edge cases.
//!
//! Tests performance under pathological conditions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::path::{Path, PathBuf};

use quench::adapter::rust::{CfgTestInfo, CargoWorkspace, RustAdapter};
use quench::adapter::Adapter;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/stress-rust")
        .join(name)
}

/// Benchmark large file parsing.
fn bench_large_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_large_files");
    group.sample_size(20); // Fewer samples for slow benchmarks

    // Generate content inline for predictable sizing
    for lines in [10_000, 50_000] {
        let content: String = (0..lines)
            .map(|i| {
                if i % 100 == 50 {
                    "#[cfg(test)]\nmod tests { #[test] fn t() {} }\n".to_string()
                } else {
                    format!("pub fn func_{}() {{ }}\n", i)
                }
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("cfg_test_parse", format!("{}_lines", lines)),
            &content,
            |b, content| b.iter(|| CfgTestInfo::parse(content)),
        );
    }

    group.finish();
}

/// Benchmark many #[cfg(test)] blocks in single file.
fn bench_many_cfg_test_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_many_cfg_test");

    // 50 separate #[cfg(test)] blocks
    let content: String = (0..50)
        .map(|i| {
            format!(
                "pub fn func_{}() {{}}\n\n#[cfg(test)]\nmod tests_{} {{\n    #[test]\n    fn test() {{}}\n}}\n\n",
                i, i
            )
        })
        .collect();

    group.bench_function("50_blocks", |b| {
        b.iter(|| CfgTestInfo::parse(&content))
    });

    group.finish();
}

/// Benchmark large workspace detection.
fn bench_large_workspace(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_workspace");
    group.sample_size(20);

    let fixture = fixture_path("many-packages");
    if fixture.exists() {
        group.bench_function("50_packages", |b| {
            b.iter(|| CargoWorkspace::from_root(&fixture))
        });
    }

    group.finish();
}

/// Benchmark file classification on large workspace.
fn bench_large_workspace_classify(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_classify");

    let adapter = RustAdapter::new();

    // 1000 paths (50 packages × 20 files)
    let paths: Vec<PathBuf> = (1..=50)
        .flat_map(|pkg| {
            (1..=20).map(move |f| {
                PathBuf::from(format!("crates/pkg_{}/src/mod_{}.rs", pkg, f))
            })
        })
        .collect();

    group.bench_function("1000_files", |b| {
        b.iter(|| {
            for path in &paths {
                let _ = adapter.classify(path);
            }
        })
    });

    group.finish();
}

/// Benchmark deep module nesting path classification.
fn bench_deep_nesting(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_deep_nesting");

    let adapter = RustAdapter::new();

    // 20 levels of nesting
    let deep_paths: Vec<PathBuf> = (1..=20)
        .map(|level| {
            let mut path = PathBuf::from("src");
            for l in 1..=level {
                path.push(format!("level_{}", l));
            }
            path.push("mod.rs");
            path
        })
        .collect();

    group.bench_function("20_levels", |b| {
        b.iter(|| {
            for path in &deep_paths {
                let _ = adapter.classify(path);
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_large_files,
    bench_many_cfg_test_blocks,
    bench_large_workspace,
    bench_large_workspace_classify,
    bench_deep_nesting,
);
criterion_main!(benches);
```

**Update `Cargo.toml`:**
```toml
[[bench]]
name = "stress"
harness = false
```

**Verification:**
```bash
cargo bench --bench stress
```

**Milestone:** Stress benchmarks run and produce numbers.

**Status:** [ ] Pending

---

### Phase 3: Run Stress Tests and Validate Limits

**Goal:** Verify adapter handles pathological inputs within acceptable limits.

**Performance limits from spec:**

| Scenario | Acceptable | Unacceptable |
|----------|------------|--------------|
| 50K line file | < 50ms parse | > 200ms |
| 50 package workspace | < 500ms detect | > 2s |
| 1000 file classify | < 5ms | > 50ms |
| 20 level nesting | < 1ms classify | > 10ms |

**Test commands:**

```bash
cargo build --release

# Large file stress test
hyperfine --warmup 1 --runs 5 \
    './target/release/quench check tests/fixtures/stress-rust/large-files' \
    --export-markdown reports/stress-large-files.md

# Large workspace stress test
hyperfine --warmup 1 --runs 5 \
    './target/release/quench check tests/fixtures/stress-rust/many-packages' \
    --export-markdown reports/stress-workspace.md

# Deep nesting stress test
hyperfine --warmup 1 --runs 5 \
    './target/release/quench check tests/fixtures/stress-rust/deep-nesting' \
    --export-markdown reports/stress-nesting.md

# Many cfg(test) blocks
hyperfine --warmup 1 --runs 5 \
    './target/release/quench check tests/fixtures/stress-rust/many-cfg-test' \
    --export-markdown reports/stress-cfg-test.md
```

**Memory validation:**
```bash
# Check memory usage doesn't explode
/usr/bin/time -l ./target/release/quench check tests/fixtures/stress-rust/large-files 2>&1 | grep "maximum resident"
/usr/bin/time -l ./target/release/quench check tests/fixtures/stress-rust/many-packages 2>&1 | grep "maximum resident"
```

**Expected results:**

| Fixture | Expected Time | Memory Limit |
|---------|---------------|--------------|
| large-files (60K lines) | < 200ms | < 100MB |
| many-packages (1000 files) | < 500ms | < 100MB |
| deep-nesting (20 levels) | < 50ms | < 50MB |
| many-cfg-test (50 blocks) | < 50ms | < 50MB |

**Milestone:** All stress tests complete within limits.

**Status:** [ ] Pending

---

### Phase 4: Document Performance Characteristics

**Goal:** Create `reports/checkpoint-4e-stress.md` with stress test results and performance characteristics.

**Report template:**

```markdown
# Checkpoint 4E: Stress Test Report - Rust Adapter

Generated: YYYY-MM-DD
Hardware: [CPU, RAM, OS]

## Summary

| Scenario | Limit | Actual | Status |
|----------|-------|--------|--------|
| 50K line file parse | < 50ms | Xms | PASS/FAIL |
| 50 package workspace | < 500ms | Xms | PASS/FAIL |
| 1000 file classify | < 5ms | Xms | PASS/FAIL |
| 20 level nesting | < 1ms | Xms | PASS/FAIL |
| Memory (large files) | < 100MB | XMB | PASS/FAIL |

## Detailed Results

### Large File Parsing

| File Size | Parse Time | Per-Line | Notes |
|-----------|------------|----------|-------|
| 10K lines | | | |
| 50K lines | | | |

**Scaling:** O(n) linear with file size.

### Large Workspace Detection

| Packages | Detection Time | Per-Package |
|----------|----------------|-------------|
| 50 | | |

**Scaling:** O(packages) with directory scan overhead.

### Path Classification

| Files | Total Time | Per-File |
|-------|------------|----------|
| 1000 | | |

**Scaling:** O(files × patterns), sub-microsecond per file.

### Deep Nesting

| Depth | Classify Time |
|-------|---------------|
| 20 levels | |

**Scaling:** O(path_components), negligible.

### Memory Usage

| Fixture | Peak Memory |
|---------|-------------|
| large-files | |
| many-packages | |

## Performance Characteristics

### CfgTestInfo::parse()

- **Complexity:** O(lines × line_length)
- **Scaling:** Linear, ~0.6-0.7µs per line
- **Memory:** O(test_blocks) for storing ranges
- **Edge cases:** Handles escaped strings, nested braces

### CargoWorkspace::from_root()

- **Complexity:** O(packages) with I/O per package
- **Scaling:** ~3µs per package for detection
- **Memory:** O(packages) for package names
- **Edge cases:** Handles glob patterns, missing Cargo.toml

### RustAdapter::classify()

- **Complexity:** O(patterns × path_components)
- **Scaling:** Sub-microsecond per file
- **Memory:** Constant (GlobSet is shared)
- **Edge cases:** Deep paths, unusual extensions

## Regression Testing

To detect performance regressions, run:

```bash
cargo bench --bench adapter -- --save-baseline 4e
cargo bench --bench stress -- --save-baseline 4e

# Later, compare against baseline
cargo bench --bench adapter -- --baseline 4e
cargo bench --bench stress -- --baseline 4e
```

## Recommendations

[Based on stress test results]
```

**Milestone:** Stress test report complete.

**Status:** [ ] Pending

---

### Phase 5: Add CI Performance Gate

**Goal:** Ensure performance regressions are caught in CI.

**Add to Makefile or CI script:**

```makefile
# In Makefile
bench-baseline:
	cargo bench --bench adapter -- --save-baseline main
	cargo bench --bench stress -- --save-baseline main

bench-check:
	cargo bench --bench adapter -- --baseline main --noplot
	cargo bench --bench stress -- --baseline main --noplot
```

**Add performance test to `scripts/bootstrap`:**

```bash
# Performance smoke test (fast sanity check)
echo "Running performance smoke test..."
RUST_LOG=warn timeout 5s ./target/release/quench check tests/fixtures/bench-rust > /dev/null || {
    echo "ERROR: Performance smoke test failed (timeout or error)"
    exit 1
}
echo "Performance smoke test passed"
```

**Verification:**
```bash
make bench-baseline
make bench-check
./scripts/bootstrap
```

**Milestone:** CI catches performance regressions.

**Status:** [ ] Pending

---

### Phase 6: Run Full Test Suite

Execute `make check` to ensure all quality gates pass.

```bash
make check
```

**Checklist:**
- [ ] `cargo fmt --all -- --check` - no formatting issues
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - no warnings
- [ ] `cargo test --all` - all tests pass
- [ ] `cargo build --all` - builds successfully
- [ ] `./scripts/bootstrap` - conventions pass (including perf smoke test)
- [ ] `cargo audit` - no vulnerabilities
- [ ] `cargo deny check` - licenses/bans OK

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Stress Test Design Rationale

Each stress test targets a specific concern from the performance spec:

| Stress Test | Spec Concern | What It Validates |
|-------------|--------------|-------------------|
| Large files | "Large Files" section | Linear scaling, no quadratic blowup |
| Many cfg_test | Adapter-specific | Multiple block detection efficiency |
| Many packages | "Large File Counts" | Workspace detection scales |
| Deep nesting | "Deep Directory Trees" | Path classification handles depth |

### Performance Scaling Models

Document expected complexity for each operation:

```
CfgTestInfo::parse(content)
  Time: O(lines) where lines = content.lines().count()
  Memory: O(blocks) where blocks = number of #[cfg(test)] blocks
  Expected: ~0.7µs per line (from 4D benchmarks)

CargoWorkspace::from_root(path)
  Time: O(packages) with I/O overhead per package
  Memory: O(packages) for package name strings
  Expected: ~3µs per package + fixed overhead

RustAdapter::classify(path)
  Time: O(patterns × components) but patterns and components are bounded
  Memory: O(1) - GlobSet is pre-compiled and shared
  Expected: ~0.1µs per file
```

### Regression Detection Strategy

Use Criterion's baseline comparison feature:

```bash
# Save baseline after known-good state
cargo bench --bench adapter -- --save-baseline good

# After changes, compare
cargo bench --bench adapter -- --baseline good

# Criterion reports:
# - "Performance has regressed" if >5% slower
# - "Performance has improved" if >5% faster
# - "No change in performance" otherwise
```

### Memory Budget Validation

The spec defines memory limits. Validate with:

```bash
# macOS
/usr/bin/time -l ./target/release/quench check <fixture>

# Linux
/usr/bin/time -v ./target/release/quench check <fixture>

# Look for "maximum resident set size"
```

Expected memory usage:
- Small project: < 20MB
- Medium project (50K LOC): < 50MB
- Large project (500K LOC): < 200MB
- Stress fixtures: < 100MB each

## Verification Plan

### Phase-by-Phase Verification

```bash
# Phase 1: Fixtures exist
ls tests/fixtures/stress-rust/{large-files,many-packages,deep-nesting,many-cfg-test}

# Phase 2: Benchmarks run
cargo bench --bench stress

# Phase 3: Stress tests pass limits
hyperfine './target/release/quench check tests/fixtures/stress-rust/large-files'

# Phase 4: Report exists
ls reports/checkpoint-4e-stress.md

# Phase 5: CI gate works
make bench-baseline && make bench-check

# Phase 6: Full quality gates
make check
```

### Success Criteria

1. **All stress tests within limits:** No operation exceeds unacceptable thresholds
2. **Linear scaling confirmed:** Large files and workspaces show O(n) behavior
3. **Memory bounded:** No fixture exceeds 100MB peak memory
4. **Regression detection working:** Baseline comparison catches 5%+ regressions
5. **Quality gates pass:** `make check` succeeds

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Create stress test fixtures | [ ] Pending |
| 2 | Add stress benchmarks | [ ] Pending |
| 3 | Run stress tests and validate limits | [ ] Pending |
| 4 | Document performance characteristics | [ ] Pending |
| 5 | Add CI performance gate | [ ] Pending |
| 6 | Run full test suite | [ ] Pending |

## Notes

- Stress fixtures should be added to `.gitignore` to avoid repository bloat
- Benchmark baselines are machine-specific; document hardware in reports
- The 4D results already show excellent performance; 4E confirms robustness under stress
- If any stress test fails limits, investigate before proceeding (may indicate algorithmic issue)
