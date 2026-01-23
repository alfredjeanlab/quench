# Checkpoint 3D: Benchmark - Escapes Works

**Root Feature:** `quench-0dc9`

## Overview

Performance benchmark and profiling checkpoint to measure escapes check performance on representative workloads. Validates that pattern matching, file classification, and comment searching meet the performance targets specified in `docs/specs/20-performance.md`:

| Mode | Target | Acceptable |
|------|--------|------------|
| Cold | < 500ms | < 1s |
| Warm | < 100ms | < 200ms |

This checkpoint builds on the CLOC benchmarking infrastructure from checkpoint-2d-benchmark and focuses specifically on the escapes check's unique performance characteristics: regex pattern matching, line deduplication, and justification comment searching.

## Project Structure

Key files involved:

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── check.rs              # End-to-end check benchmarks (existing)
│   └── src/
│       ├── checks/
│       │   ├── escapes.rs        # Pattern matching (profiling target)
│       │   └── escapes_tests.rs  # Micro-benchmarks
│       └── pattern/
│           └── matcher.rs        # CompiledPattern (profiling target)
├── tests/fixtures/
│   └── bench-medium/             # 500 files, 50K LOC (reuse from 2D)
├── reports/
│   └── checkpoint-3-benchmarks.md  # Benchmark results (output)
└── scripts/
    └── gen-bench-fixture         # Fixture generation (existing from 2D)
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

### Phase 1: Verify/Generate bench-medium Fixture with Escapes Patterns

**Goal:** Ensure bench-medium exists and contains representative escape hatch patterns for realistic benchmarking.

The bench-medium fixture may already exist from checkpoint-2d. If so, verify it; otherwise generate it. Additionally, ensure the fixture includes escape hatch patterns to exercise the escapes check.

**Fixture configuration** (`bench-medium/quench.toml`):

```toml
[check.escapes]
check = "warn"

[[check.escapes.patterns]]
name = "unwrap"
pattern = r"\.unwrap\(\)"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
name = "expect"
pattern = r"\.expect\("
action = "comment"
comment = "// REASON:"

[[check.escapes.patterns]]
name = "unsafe"
pattern = r"\bunsafe\b"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
name = "todo"
pattern = r"\b(TODO|FIXME|XXX)\b"
action = "count"
threshold = 100
```

**Enhance gen-bench-fixture** to include escape patterns in generated files:

```bash
# Add to gen-bench-fixture: include ~2% of lines with escape patterns
# Example modifications to source file template:
RUST_SOURCE='//! Auto-generated module for benchmarking.

pub fn process_item(item: &str) -> String {
    // Some functions have justified escapes
    let val = item.parse::<i32>().unwrap();  // SAFETY: test data is valid
    item.to_uppercase()
}

pub fn risky_transform(data: &[i32]) -> Vec<i32> {
    // TODO: optimize this later
    data.iter().map(|x| x * 2).collect()
}
'
```

**Verification:**
```bash
# Check fixture exists with expected size
ls tests/fixtures/bench-medium/quench.toml
find tests/fixtures/bench-medium -name '*.rs' | wc -l  # ~500
find tests/fixtures/bench-medium -name '*.rs' -exec cat {} + | wc -l  # ~50000

# Verify escape patterns present
grep -r "\.unwrap()" tests/fixtures/bench-medium --include="*.rs" | wc -l  # > 0
grep -r "// SAFETY:" tests/fixtures/bench-medium --include="*.rs" | wc -l  # > 0
```

**Milestone:** `tests/fixtures/bench-medium` exists with ~500 files, ~50K LOC, and escape hatch patterns.

**Status:** [ ] Pending

---

### Phase 2: Run Escapes Benchmarks on bench-medium

**Goal:** Establish baseline performance numbers for the escapes check.

**Criterion Benchmarks:**
```bash
cargo build --release
cargo bench --bench check -- "check_cold/check/bench-medium"
```

**Hyperfine (escapes-specific):**
```bash
# Build release binary
cargo build --release

# Cold run (clear cache first)
rm -rf tests/fixtures/bench-medium/.quench
hyperfine --warmup 0 --runs 5 \
    './target/release/quench check tests/fixtures/bench-medium --escapes'

# Warm run (cache populated)
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-medium --escapes'

# Compare escapes vs cloc vs full check
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-medium --escapes' \
    './target/release/quench check tests/fixtures/bench-medium --cloc' \
    './target/release/quench check tests/fixtures/bench-medium' \
    --export-markdown reports/escapes-vs-cloc.md
```

**Expected Results:**

| Run Type | Target | Acceptable |
|----------|--------|------------|
| Cold | < 500ms | < 1s |
| Warm | < 100ms | < 200ms |

**Milestone:** Benchmark numbers recorded for escapes check on bench-medium.

**Status:** [ ] Pending

---

### Phase 3: Profile Pattern Matching Performance

**Goal:** Identify performance hotspots in the escapes check, particularly regex pattern matching.

**Key functions to profile** (from `crates/cli/src/checks/escapes.rs`):

1. **Pattern compilation** (`compile_patterns`, line 318):
   - Called once per check run
   - Compiles regex patterns for each configured escape pattern

2. **Pattern matching** (`find_all_with_lines`, called at line 203):
   - Called for every source file × every pattern
   - Most expensive operation

3. **Line deduplication** (lines 205-212):
   ```rust
   let mut seen_lines = HashSet::new();
   let unique_matches: Vec<_> = matches
       .into_iter()
       .filter(|m| seen_lines.insert(m.line))
       .collect();
   ```

4. **Justification comment search** (`has_justification_comment`, line 413):
   - Called for each match with `action = "comment"`
   - Searches upward through file content

**Profiling with flamegraph:**
```bash
# Linux
cargo flamegraph --bench check -- "check_cold/check/bench-medium"

# macOS (requires dtrace)
sudo cargo flamegraph --root --bench check -- "check_cold/check/bench-medium"
```

**Profiling with Instruments (macOS):**
```bash
xcrun xctrace record --template 'Time Profiler' \
    --launch ./target/release/quench -- check tests/fixtures/bench-medium --escapes
```

**Analysis checklist:**
- [ ] What % of time in `CompiledPattern::compile()`?
- [ ] What % of time in `find_all_with_lines()`?
- [ ] What % of time in regex vs iterator processing?
- [ ] What % of time in `has_justification_comment()`?
- [ ] What % of time in file classification (`classify_file`)?

**Micro-benchmark** (add to `escapes_tests.rs`):

```rust
#[test]
#[ignore = "benchmark only"]
fn bench_pattern_matching_performance() {
    use std::time::Instant;

    // Generate content with ~100 lines, some with escape patterns
    let content: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                format!("let x = foo.unwrap();  // line {}\n", i)
            } else {
                format!("let x = normal_code();  // line {}\n", i)
            }
        })
        .collect();

    let pattern = CompiledPattern::compile(r"\.unwrap\(\)").unwrap();

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = pattern.find_all_with_lines(&content);
    }
    let elapsed = start.elapsed();

    println!("10K pattern matches on 100-line file: {:?}", elapsed);
    println!("Per match: {:?}", elapsed / 10_000);
    // Target: < 1ms per 100-line file
}

#[test]
#[ignore = "benchmark only"]
fn bench_comment_search_performance() {
    use std::time::Instant;

    // Generate content with justification comments
    let content: String = (0..100)
        .map(|i| {
            if i % 20 == 0 {
                "// SAFETY: this is safe\n".to_string()
            } else if i % 10 == 0 {
                "let x = foo.unwrap();\n".to_string()
            } else {
                format!("let x = code();  // line {}\n", i)
            }
        })
        .collect();

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = has_justification_comment(&content, 50, "// SAFETY:");
    }
    let elapsed = start.elapsed();

    println!("10K comment searches: {:?}", elapsed);
    println!("Per search: {:?}", elapsed / 10_000);
    // Target: < 0.1ms per search
}
```

**Milestone:** Flame graph generated, pattern matching hotspots identified and documented.

**Status:** [ ] Pending

---

### Phase 4: Analyze Line Deduplication and File Classification

**Goal:** Verify that deduplication and file classification don't introduce performance bottlenecks.

**Line deduplication analysis:**

The current implementation uses `HashSet::insert()` for O(1) deduplication:

```rust
let mut seen_lines = HashSet::new();
let unique_matches: Vec<_> = matches
    .into_iter()
    .filter(|m| seen_lines.insert(m.line))
    .collect();
```

**Verify:**
- HashSet allocation overhead for small match counts
- Memory usage for files with many matches

**File classification analysis:**

The `classify_file` function (line 503) creates a new `GenericAdapter` for each file:

```rust
fn classify_file(path: &Path, root: &Path, test_patterns: &[String]) -> FileKind {
    let adapter = GenericAdapter::new(&[], test_patterns);
    let relative = path.strip_prefix(root).unwrap_or(path);
    adapter.classify(relative)
}
```

**Potential optimization:** Create adapter once and reuse across all files.

**Micro-benchmark:**

```rust
#[test]
#[ignore = "benchmark only"]
fn bench_file_classification() {
    use std::time::Instant;
    use std::path::Path;

    let root = Path::new("/project");
    let test_patterns = default_test_patterns();
    let paths: Vec<_> = (0..1000)
        .map(|i| PathBuf::from(format!("/project/src/module_{}.rs", i)))
        .collect();

    // Current approach: new adapter per file
    let start = Instant::now();
    for path in &paths {
        let _ = classify_file(path, root, &test_patterns);
    }
    let elapsed = start.elapsed();
    println!("1K classifications (new adapter each): {:?}", elapsed);

    // Optimized: reuse adapter
    let adapter = GenericAdapter::new(&[], &test_patterns);
    let start = Instant::now();
    for path in &paths {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let _ = adapter.classify(relative);
    }
    let elapsed = start.elapsed();
    println!("1K classifications (reused adapter): {:?}", elapsed);
}
```

**Milestone:** Deduplication and classification overhead characterized.

**Status:** [ ] Pending

---

### Phase 5: Document Results in Report

**Goal:** Create comprehensive benchmark report at `reports/checkpoint-3-benchmarks.md`.

**Report template:**

```markdown
# Checkpoint 3D: Benchmark Report - Escapes Works

Generated: YYYY-MM-DD
Hardware: [CPU, RAM, OS]

## Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Cold (bench-medium, escapes) | < 500ms | XXXms | pass/fail |
| Warm (bench-medium, escapes) | < 100ms | XXms | pass/fail |
| Pattern match per file | < 1ms | X.Xms | pass/fail |
| Comment search per match | < 0.1ms | X.Xms | pass/fail |

## Detailed Results

### 1. End-to-End Benchmarks

**bench-medium (500 files, 50K LOC) - Escapes Check:**

| Run | Mean | Std Dev | Min | Max |
|-----|------|---------|-----|-----|
| Cold | | | | |
| Warm | | | | |

**Comparison with CLOC:**

| Check | Cold | Warm |
|-------|------|------|
| escapes | | |
| cloc | | |
| full | | |

### 2. Pattern Matching Profile

**Flame graph:** [link or inline SVG]

| Function | % of Total | Notes |
|----------|------------|-------|
| compile_patterns | | One-time cost |
| find_all_with_lines | | Per file × per pattern |
| has_justification_comment | | Per match with comment action |
| classify_file | | Per file |

### 3. Micro-benchmark Results

| Operation | Time | Target | Notes |
|-----------|------|--------|-------|
| Pattern match (100 lines) | | < 1ms | |
| Comment search | | < 0.1ms | |
| File classification | | < 0.01ms | |
| Line deduplication | | negligible | |

### 4. Memory Usage

| Metric | Value |
|--------|-------|
| Peak RSS (cold) | |
| Peak RSS (warm) | |

## Conclusions

[Summary of findings]

## Recommendations

[Any optimizations needed, or confirmation that performance is acceptable]
```

**Milestone:** Report complete at `reports/checkpoint-3-benchmarks.md`.

**Status:** [ ] Pending

## Key Implementation Details

### Performance Model for Escapes Check

From the escapes check implementation (`escapes.rs`):

```
Total Time = File Discovery + File Reading + Pattern Matching + Deduplication + Comment Search + Metrics
```

| Phase | Expected % | Notes |
|-------|------------|-------|
| File discovery | 10-20% | Shared with other checks |
| File reading | 20-30% | `read_to_string` per file |
| Pattern matching | 30-50% | Regex search, main hotspot |
| Deduplication | < 5% | HashSet operations |
| Comment search | 10-20% | Depends on action=comment usage |
| Metrics/output | < 5% | HashMap operations |

### Pattern Matching Complexity

For `n` files, `p` patterns, and `m` average matches per pattern per file:

- Pattern compilation: O(p) - done once
- Pattern matching: O(n × p × file_size)
- Deduplication: O(n × p × m)
- Comment search: O(n × p × m × comment_search_lines)

The dominant cost is regex matching, which is O(file_size) per pattern per file.

### Comparison with CLOC Check

| Aspect | CLOC | Escapes |
|--------|------|---------|
| File reading | Once per file | Once per file |
| Per-file processing | Line counting | Regex matching |
| Complexity | O(lines) | O(lines × patterns) |
| Expected overhead | Baseline | 2-4x baseline |

The escapes check should be roughly 2-4x slower than CLOC due to regex operations, but still well within targets for bench-medium.

## Verification Plan

1. **Fixture verification:**
   ```bash
   ls tests/fixtures/bench-medium/quench.toml
   grep -c "\.unwrap" tests/fixtures/bench-medium/src/*.rs | head -5
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

4. **Micro-benchmarks:**
   ```bash
   cargo test --package quench -- bench_ --ignored --nocapture
   ```

5. **Report generation:**
   ```bash
   ls reports/checkpoint-3-benchmarks.md
   ```

6. **Quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Verify/generate bench-medium with escapes patterns | [ ] Pending |
| 2 | Run escapes benchmarks | [ ] Pending |
| 3 | Profile pattern matching | [ ] Pending |
| 4 | Analyze deduplication and classification | [ ] Pending |
| 5 | Document in report | [ ] Pending |

## Notes

- The bench-medium fixture should be in `.gitignore` to avoid repository bloat
- Benchmark results vary by hardware; document machine specs in the report
- If targets are not met, consider:
  - Lazy pattern compilation
  - Parallel file processing
  - Compiled pattern caching
  - Reducing redundant `GenericAdapter` creation
