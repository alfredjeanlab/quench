# Checkpoint 2D: Benchmark Analysis - CLOC Works

**Root Feature:** `quench-fda7`

## Overview

Performance benchmark and profiling checkpoint to measure CLOC check performance on representative workloads. Validates that line counting, pattern matching, and caching meet the performance targets specified in `docs/specs/20-performance.md`:

| Mode | Target | Acceptable |
|------|--------|------------|
| Cold | < 500ms | < 1s |
| Warm | < 100ms | < 200ms |

## Project Structure

Key files involved:

```
quench/
├── crates/cli/
│   ├── benches/
│   │   ├── check.rs              # End-to-end check benchmarks
│   │   └── file_walking.rs       # File discovery benchmarks
│   └── src/
│       ├── checks/cloc.rs        # CLOC implementation (profiling target)
│       └── cache.rs              # File-level cache
├── tests/fixtures/
│   └── bench-medium/             # 500 files, 50K LOC (to be generated)
├── reports/
│   └── checkpoint-2-benchmarks.md  # Benchmark results (output)
└── scripts/
    └── gen-bench-fixture         # Fixture generation script
```

## Dependencies

**Existing:**
- `criterion` - Benchmarking framework (already configured)
- `flamegraph` - Flame graph profiling
- `hyperfine` - CLI benchmarking tool

**Install if needed:**
```bash
cargo install flamegraph
brew install hyperfine  # or apt-get install hyperfine
```

## Implementation Phases

### Phase 1: Generate bench-medium Fixture

The `bench-medium` fixture (500 files, 50K LOC) doesn't exist yet. Generate it following the spec from `docs/specs/20-performance.md`.

**Script:** `scripts/gen-bench-fixture`

```bash
#!/usr/bin/env bash
set -euo pipefail

FIXTURE_DIR="${1:-tests/fixtures/bench-medium}"
FILE_COUNT="${2:-500}"
TARGET_LOC="${3:-50000}"

mkdir -p "$FIXTURE_DIR/src" "$FIXTURE_DIR/tests"

# Calculate lines per file (distribute evenly)
LINES_PER_FILE=$((TARGET_LOC / FILE_COUNT))
SOURCE_FILES=$((FILE_COUNT * 80 / 100))  # 80% source
TEST_FILES=$((FILE_COUNT - SOURCE_FILES))

echo "Generating $SOURCE_FILES source files + $TEST_FILES test files..."

# Generate source files (80%)
for i in $(seq 1 "$SOURCE_FILES"); do
    FILE="$FIXTURE_DIR/src/module_$i.rs"
    cat > "$FILE" << 'RUST'
//! Auto-generated module for benchmarking.

pub fn process_item(item: &str) -> String {
    item.to_uppercase()
}

pub fn transform_data(data: &[i32]) -> Vec<i32> {
    data.iter().map(|x| x * 2).collect()
}
RUST
    # Pad to target line count
    for _ in $(seq 1 "$LINES_PER_FILE"); do
        echo "// padding line" >> "$FILE"
    done
done

# Generate test files (20%)
for i in $(seq 1 "$TEST_FILES"); do
    FILE="$FIXTURE_DIR/tests/test_$i.rs"
    cat > "$FILE" << 'RUST'
#![allow(clippy::unwrap_used)]

#[test]
fn test_example() {
    assert!(true);
}
RUST
    for _ in $(seq 1 "$LINES_PER_FILE"); do
        echo "// padding line" >> "$FILE"
    done
done

# Create minimal quench.toml
cat > "$FIXTURE_DIR/quench.toml" << 'TOML'
[check.cloc]
check = "error"
max_lines = 1000
max_lines_test = 1500
TOML

echo "Generated fixture at $FIXTURE_DIR"
echo "Files: $(find "$FIXTURE_DIR" -name '*.rs' | wc -l)"
echo "LOC: $(find "$FIXTURE_DIR" -name '*.rs' -exec cat {} + | wc -l)"
```

**Milestone:** `tests/fixtures/bench-medium` exists with ~500 files and ~50K LOC.

**Verification:**
```bash
./scripts/gen-bench-fixture tests/fixtures/bench-medium 500 50000
find tests/fixtures/bench-medium -name '*.rs' | wc -l  # ~500
find tests/fixtures/bench-medium -name '*.rs' -exec cat {} + | wc -l  # ~50000
```

**Status:** [ ] Pending

---

### Phase 2: Run CLOC Benchmarks on bench-medium

Run end-to-end benchmarks using the existing criterion setup and hyperfine for wall-clock timing.

**Criterion Benchmarks:**
```bash
cargo build --release
cargo bench --bench check -- "check_cold/check/bench-medium"
```

**Hyperfine (cold vs warm):**
```bash
# Build release binary
cargo build --release

# Cold run (clear cache first)
rm -rf tests/fixtures/bench-medium/.quench
hyperfine --warmup 0 --runs 5 \
    './target/release/quench check tests/fixtures/bench-medium --cloc'

# Warm run (cache populated)
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-medium --cloc'
```

**Expected Results:**

| Run Type | Target | Acceptable |
|----------|--------|------------|
| Cold | < 500ms | < 1s |
| Warm | < 100ms | < 200ms |

**Milestone:** Benchmark numbers recorded for bench-medium.

**Status:** [ ] Pending

---

### Phase 3: Profile Line Counting Performance

Profile `count_nonblank_lines` and `count_tokens` functions to identify hotspots.

**Target functions** (`crates/cli/src/checks/cloc.rs`):

```rust
// Line 313-320: count_nonblank_lines
fn count_nonblank_lines(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    Ok(text.lines().filter(|l| !l.trim().is_empty()).count())
}

// Line 324-331: count_tokens
fn count_tokens(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    Ok(text.chars().count() / 4)
}
```

**Profiling with flamegraph:**
```bash
# Linux
cargo flamegraph --bench check -- "check_cold/check/bench-medium"

# macOS (requires dtrace)
cargo flamegraph --root --bench check -- "check_cold/check/bench-medium"
```

**Profiling with Instruments (macOS):**
```bash
xcrun xctrace record --template 'Time Profiler' \
    --launch ./target/release/quench check tests/fixtures/bench-medium --cloc
```

**Analysis checklist:**
- [ ] What % of time in `count_nonblank_lines`?
- [ ] What % of time in `count_tokens`?
- [ ] What % of time in file I/O vs string processing?
- [ ] Is UTF-8 lossy fallback triggered often?

**Milestone:** Flame graph generated, hotspots identified and documented.

**Status:** [ ] Pending

---

### Phase 4: Profile Pattern Matching Performance

Profile `PatternMatcher` to ensure pattern matching isn't a bottleneck.

**Target code** (`crates/cli/src/checks/cloc.rs:238-264`):

```rust
struct PatternMatcher {
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl PatternMatcher {
    fn is_test_file(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.test_patterns.is_match(relative)
    }
}
```

**Analysis checklist:**
- [ ] What % of time in `is_test_file`?
- [ ] What % of time in `is_excluded`?
- [ ] Is `GlobSet` performance acceptable for 500+ files?
- [ ] Is `strip_prefix` called unnecessarily?

**Micro-benchmark (add to `cloc_tests.rs`):**

```rust
#[test]
#[ignore = "benchmark only"]
fn bench_pattern_matching() {
    let matcher = PatternMatcher::new(
        &[
            "**/tests/**".into(),
            "**/test/**".into(),
            "**/*_test.*".into(),
            "**/*_tests.*".into(),
            "**/*.test.*".into(),
            "**/*.spec.*".into(),
            "**/test_*.*".into(),
        ],
        &["**/vendor/**".into()],
    );

    let root = Path::new("/project");
    let paths: Vec<_> = (0..1000)
        .map(|i| PathBuf::from(format!("/project/src/module_{}.rs", i)))
        .collect();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        for path in &paths {
            let _ = matcher.is_test_file(path, root);
        }
    }
    let elapsed = start.elapsed();
    println!("100K pattern matches: {:?}", elapsed);
    // Target: < 100ms for 100K matches
}
```

**Milestone:** Pattern matching performance characterized.

**Status:** [ ] Pending

---

### Phase 5: Verify Caching Speedup

Validate that the file-level cache in `cache.rs` provides the expected 5-10x speedup on warm runs.

**Cache implementation** (`crates/cli/src/cache.rs`):
- Key: `(path, mtime_secs, mtime_nanos, size)`
- Storage: Binary serialized to `.quench/cache.bin`
- Expected speedup: 10x (spec target)

**Test procedure:**
```bash
# Clear cache
rm -rf tests/fixtures/bench-medium/.quench

# Cold run
hyperfine --warmup 0 --runs 3 \
    './target/release/quench check tests/fixtures/bench-medium --cloc' \
    --export-json cold.json

# Warm run (cache populated)
hyperfine --warmup 1 --runs 5 \
    './target/release/quench check tests/fixtures/bench-medium --cloc' \
    --export-json warm.json

# Calculate speedup
cold_ms=$(jq '.results[0].mean * 1000' cold.json)
warm_ms=$(jq '.results[0].mean * 1000' warm.json)
echo "Speedup: $(echo "$cold_ms / $warm_ms" | bc -l)x"
```

**Expected results:**

| Metric | Target | Notes |
|--------|--------|-------|
| Cold time | < 500ms | First run, full processing |
| Warm time | < 100ms | Cache hit, skip re-reading |
| Speedup | > 5x | Ideally 10x |
| Cache hit rate | > 95% | On unchanged files |

**Milestone:** Cache speedup verified and documented.

**Status:** [ ] Pending

---

### Phase 6: Document Results in Report

Create `reports/checkpoint-2-benchmarks.md` with all benchmark results.

**Report template:**

```markdown
# Checkpoint 2D: Benchmark Report - CLOC Works

Generated: YYYY-MM-DD

## Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Cold (bench-medium) | < 500ms | XXXms | ✓/✗ |
| Warm (bench-medium) | < 100ms | XXms | ✓/✗ |
| Cache speedup | > 5x | X.Xx | ✓/✗ |

## Detailed Results

### 1. End-to-End Benchmarks

**bench-medium (500 files, 50K LOC):**

| Run | Mean | Std Dev | Min | Max |
|-----|------|---------|-----|-----|
| Cold | | | | |
| Warm | | | | |

### 2. Line Counting Profile

**Flame graph:** [link to SVG]

| Function | % of Total | Notes |
|----------|------------|-------|
| count_nonblank_lines | | |
| count_tokens | | |
| File I/O | | |

### 3. Pattern Matching Profile

| Operation | Time per 1K files | Notes |
|-----------|-------------------|-------|
| is_test_file | | |
| is_excluded | | |

### 4. Cache Performance

| Metric | Value |
|--------|-------|
| Cache hit rate | |
| Cold → Warm speedup | |
| Cache file size | |

## Conclusions

[Summary of findings, any optimizations needed]
```

**Milestone:** Report complete at `reports/checkpoint-2-benchmarks.md`.

**Status:** [ ] Pending

## Key Implementation Details

### Performance Model

From `docs/specs/20-performance.md`:

```
Total Time = File Discovery + File Reading + Pattern Matching + Aggregation
```

| Phase | Expected % | Target |
|-------|------------|--------|
| File discovery | 30-50% | < 200ms |
| File reading | 20-30% | < 150ms |
| Pattern matching | 20-40% | < 150ms |
| Aggregation | < 5% | < 25ms |

### Caching Strategy

The cache uses file metadata for invalidation:

```rust
struct FileCacheKey {
    mtime_secs: i64,
    mtime_nanos: u32,
    size: u64,
}
```

Cache invalidation triggers:
- File mtime changed
- File size changed
- Config changed (via `hash_config`)
- Cache version mismatch

### Profiling Commands Reference

```bash
# Criterion benchmarks
cargo bench --bench check

# Flame graph (Linux)
cargo flamegraph --bench check

# Flame graph (macOS)
sudo cargo flamegraph --root --bench check

# macOS Instruments
xcrun xctrace record --template 'Time Profiler' --launch ./target/release/quench

# Hyperfine comparison
hyperfine --warmup 2 --runs 10 'cmd1' 'cmd2' --export-markdown bench.md
```

## Verification Plan

1. **Fixture generation:**
   ```bash
   ./scripts/gen-bench-fixture tests/fixtures/bench-medium 500 50000
   ls tests/fixtures/bench-medium/src/*.rs | wc -l  # ~400
   ls tests/fixtures/bench-medium/tests/*.rs | wc -l  # ~100
   ```

2. **Benchmark execution:**
   ```bash
   cargo bench --bench check -- "bench-medium"
   ```

3. **Profiling:**
   ```bash
   cargo flamegraph --bench check -- "bench-medium"
   open flamegraph.svg
   ```

4. **Cache verification:**
   ```bash
   rm -rf tests/fixtures/bench-medium/.quench
   time ./target/release/quench check tests/fixtures/bench-medium --cloc
   time ./target/release/quench check tests/fixtures/bench-medium --cloc
   ```

5. **Quality gates:**
   ```bash
   make check
   ```

## Summary

| Task | Status |
|------|--------|
| Generate bench-medium fixture | [ ] Pending |
| Run CLOC benchmarks | [ ] Pending |
| Profile line counting | [ ] Pending |
| Profile pattern matching | [ ] Pending |
| Verify cache speedup | [ ] Pending |
| Document in report | [ ] Pending |

## Notes

- The bench-medium fixture should be added to `.gitignore` to avoid bloating the repository
- Benchmark results will vary by hardware; document the test machine specs in the report
- If targets are not met, reference `docs/specs/20-performance.md` optimization backlog for next steps
