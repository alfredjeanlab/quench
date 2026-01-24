# Checkpoint Go-1D: Benchmark and Identify Bottlenecks

**Root Feature:** `quench-6449`

## Overview

Profile Go adapter performance and identify bottlenecks. This is a diagnostic checkpoint focused on measurement and analysis, not optimization.

Key goals:
1. **Measure end-to-end check times** on Go fixtures (go-simple, go-multi)
2. **Profile adapter operations** (go.mod parsing, package enumeration, file classification)
3. **Profile escape pattern matching** for Go-specific patterns
4. **Profile //nolint parsing** overhead
5. **Identify bottlenecks** (operations taking >100ms)
6. **Document findings** in `reports/checkpoint-go-1-perf.md`

## Project Structure

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── adapter.rs        # Add Go adapter benchmarks
│   └── src/adapter/go/
│       ├── mod.rs            # GoAdapter, enumerate_packages, parse_go_mod
│       ├── policy.rs         # Lint policy checking
│       └── suppress.rs       # parse_nolint_directives
├── tests/fixtures/
│   ├── go-simple/            # ~4 files, ~30 LOC
│   ├── go-multi/             # ~8 files, ~75 LOC
│   └── golang/               # Larger fixture with escapes
└── reports/
    └── checkpoint-go-1-perf.md   # NEW: Performance report
```

## Dependencies

No new external dependencies. Uses existing infrastructure:
- `criterion` crate for benchmarks (already in dev-dependencies)
- `hyperfine` CLI for wall-clock measurements (optional)
- `cargo flamegraph` for CPU profiling (optional)

## Implementation Phases

### Phase 1: Add Go Adapter Benchmarks to adapter.rs

**Goal:** Add Go-specific benchmarks to the existing adapter benchmark suite.

**File:** `crates/cli/benches/adapter.rs`

Add the following benchmark functions:

```rust
use quench::adapter::go::{GoAdapter, parse_go_mod, enumerate_packages, parse_nolint_directives};

/// Benchmark GoAdapter creation.
fn bench_go_adapter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("go_adapter_creation");

    group.bench_function("GoAdapter::new", |b| {
        b.iter(|| black_box(GoAdapter::new()))
    });

    group.finish();
}

/// Benchmark Go file classification.
fn bench_go_classify(c: &mut Criterion) {
    let go_adapter = GoAdapter::new();

    // Generate test paths
    let source_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("pkg/module_{}/handler.go", i)))
        .collect();
    let test_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("pkg/module_{}/handler_test.go", i)))
        .collect();
    let vendor_paths: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("vendor/github.com/pkg/lib_{}.go", i)))
        .collect();

    let mut group = c.benchmark_group("go_classify");

    group.bench_function("go_1k_source", |b| {
        b.iter(|| {
            for path in &source_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.bench_function("go_1k_test", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.bench_function("go_1k_vendor_ignored", |b| {
        b.iter(|| {
            for path in &vendor_paths {
                black_box(go_adapter.classify(path));
            }
        })
    });

    group.finish();
}

/// Benchmark go.mod parsing.
fn bench_go_mod_parse(c: &mut Criterion) {
    let simple_go_mod = "module github.com/example/project\n\ngo 1.21\n";
    let complex_go_mod = r#"
module github.com/example/complex-project

go 1.22

require (
    github.com/pkg/errors v0.9.1
    github.com/stretchr/testify v1.8.4
    golang.org/x/sync v0.3.0
)

require (
    github.com/davecgh/go-spew v1.1.1 // indirect
    github.com/pmezard/go-difflib v1.0.0 // indirect
    gopkg.in/yaml.v3 v3.0.1 // indirect
)
"#;

    let mut group = c.benchmark_group("go_mod_parse");

    group.bench_function("simple_go_mod", |b| {
        b.iter(|| black_box(parse_go_mod(simple_go_mod)))
    });

    group.bench_function("complex_go_mod", |b| {
        b.iter(|| black_box(parse_go_mod(complex_go_mod)))
    });

    group.finish();
}

/// Benchmark package enumeration on fixtures.
fn bench_package_enumeration(c: &mut Criterion) {
    let go_simple = fixture_path("go-simple");
    let go_multi = fixture_path("go-multi");
    let golang = fixture_path("golang");

    let mut group = c.benchmark_group("go_package_enumeration");

    if go_simple.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "go-simple"),
            &go_simple,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    if go_multi.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "go-multi"),
            &go_multi,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    if golang.exists() {
        group.bench_with_input(
            BenchmarkId::new("enumerate_packages", "golang"),
            &golang,
            |b, path| b.iter(|| black_box(enumerate_packages(path))),
        );
    }

    group.finish();
}

/// Benchmark //nolint directive parsing.
fn bench_nolint_parse(c: &mut Criterion) {
    // Content with various nolint directives
    let content_with_nolint: String = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                "//nolint:errcheck // OK: error is logged\nfunc process() error {\n".to_string()
            } else if i % 15 == 0 {
                "//nolint:gosec,govet\n// REASON: legacy code\nfunc legacy() {}\n".to_string()
            } else if i % 20 == 0 {
                "//nolint\nfunc skip() {}\n".to_string()
            } else {
                format!("func handler_{}() {{}}\n", i)
            }
        })
        .collect();

    let content_without: String = (0..100)
        .map(|i| format!("func handler_{}() {{}}\n", i))
        .collect();

    let mut group = c.benchmark_group("nolint_parse");

    group.bench_function("with_nolint_100_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&content_with_nolint, None)))
    });

    group.bench_function("without_nolint_100_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&content_without, None)))
    });

    group.bench_function("with_nolint_100_lines_pattern", |b| {
        b.iter(|| black_box(parse_nolint_directives(&content_with_nolint, Some("// REASON:"))))
    });

    // Larger file
    let large_content: String = content_with_nolint.repeat(10);
    group.bench_function("with_nolint_1000_lines", |b| {
        b.iter(|| black_box(parse_nolint_directives(&large_content, None)))
    });

    group.finish();
}
```

Register new benchmark functions in `criterion_group!`:

```rust
criterion_group!(
    benches,
    bench_adapter_creation,
    bench_globset_patterns,
    bench_classify,
    bench_cfg_test_parse,
    bench_classify_lines,
    bench_workspace_detection,
    bench_suppress_parse,
    bench_shellcheck_suppress_parse,
    // New Go benchmarks
    bench_go_adapter_creation,
    bench_go_classify,
    bench_go_mod_parse,
    bench_package_enumeration,
    bench_nolint_parse,
);
```

**Verification:**
```bash
cargo bench --bench adapter -- go
# Should show Go-specific benchmark results
```

### Phase 2: End-to-End Check Time Measurement

**Goal:** Measure wall-clock time for `quench check` on Go fixtures.

**Manual measurements using hyperfine (if available):**
```bash
# Build release binary
cargo build --release

# Measure go-simple (cold + warm)
hyperfine --warmup 3 --runs 10 \
    'cargo run --release -q -- check tests/fixtures/go-simple' \
    --export-json reports/go-simple-timing.json

# Measure go-multi (cold + warm)
hyperfine --warmup 3 --runs 10 \
    'cargo run --release -q -- check tests/fixtures/go-multi' \
    --export-json reports/go-multi-timing.json

# Measure golang fixture (larger)
hyperfine --warmup 3 --runs 10 \
    'cargo run --release -q -- check tests/fixtures/golang' \
    --export-json reports/golang-timing.json
```

**Alternative using shell timing:**
```bash
# If hyperfine not available
for fixture in go-simple go-multi golang; do
    echo "=== $fixture ==="
    for i in 1 2 3 4 5; do
        time cargo run --release -q -- check "tests/fixtures/$fixture" 2>&1 | grep real
    done
done
```

**Verification:**
```bash
# All fixtures should complete in < 100ms
ls reports/*-timing.json 2>/dev/null || echo "Manual timing recorded"
```

### Phase 3: CPU Profiling with Flamegraph

**Goal:** Generate CPU flamegraphs to identify hot paths in Go adapter.

**Prerequisites:**
```bash
# Install flamegraph tool (if not present)
cargo install flamegraph
```

**Generate flamegraphs:**
```bash
# Profile go-simple check
cargo flamegraph --bin quench -- check tests/fixtures/go-simple \
    -o flamegraph-go-simple.svg

# Profile go-multi check
cargo flamegraph --bin quench -- check tests/fixtures/go-multi \
    -o flamegraph-go-multi.svg

# Profile golang fixture (larger, more representative)
cargo flamegraph --bin quench -- check tests/fixtures/golang \
    -o flamegraph-golang.svg
```

**Analysis focus areas:**
1. `GoAdapter::new()` - GlobSet compilation time
2. `enumerate_packages()` - Directory traversal time
3. `parse_go_mod()` - String parsing time
4. `classify()` - GlobSet matching time
5. `parse_nolint_directives()` - Line-by-line parsing time
6. Pattern matching in escape check

**Verification:**
```bash
ls flamegraph-*.svg
# Should have 3 flamegraph files
```

### Phase 4: Targeted Micro-Profiling

**Goal:** Isolate and measure specific Go adapter operations.

**Add timing instrumentation (temporary, for profiling only):**

```rust
// In a test or benchmark, measure individual operations:
use std::time::Instant;

fn profile_go_operations() {
    let root = Path::new("tests/fixtures/go-multi");

    // 1. Measure adapter creation
    let start = Instant::now();
    let adapter = GoAdapter::new();
    let creation_time = start.elapsed();
    println!("GoAdapter::new(): {:?}", creation_time);

    // 2. Measure package enumeration
    let start = Instant::now();
    let packages = enumerate_packages(root);
    let enum_time = start.elapsed();
    println!("enumerate_packages(): {:?} ({} packages)", enum_time, packages.len());

    // 3. Measure go.mod parsing
    let go_mod_content = std::fs::read_to_string(root.join("go.mod")).unwrap();
    let start = Instant::now();
    let _ = parse_go_mod(&go_mod_content);
    let parse_time = start.elapsed();
    println!("parse_go_mod(): {:?}", parse_time);

    // 4. Measure file classification (1000 paths)
    let paths: Vec<_> = (0..1000).map(|i| PathBuf::from(format!("pkg/mod_{}.go", i))).collect();
    let start = Instant::now();
    for p in &paths {
        adapter.classify(p);
    }
    let classify_time = start.elapsed();
    println!("classify() x 1000: {:?} ({:?}/file)", classify_time, classify_time / 1000);
}
```

**Target thresholds (based on Rust/Shell adapter baselines):**

| Operation | Target | Based On |
|-----------|--------|----------|
| GoAdapter::new() | < 100µs | ShellAdapter ~78µs |
| classify() per file | < 0.1µs | Shell ~0.043µs, Rust ~0.11µs |
| enumerate_packages() | < 10ms | FS traversal dependent |
| parse_go_mod() | < 10µs | Simple string parsing |
| parse_nolint_directives() 100 lines | < 10µs | Shell suppress ~3.6µs |

**Verification:**
```bash
cargo test profile_go_operations -- --nocapture 2>&1 | grep "::new\|enumerate\|parse\|classify"
```

### Phase 5: Identify Bottlenecks (>100ms Operations)

**Goal:** Flag any operations exceeding 100ms threshold.

**Bottleneck criteria:**
- Single operation > 100ms = Critical bottleneck
- Single operation > 10ms = Potential bottleneck
- End-to-end check > 500ms on small fixture = Needs investigation

**Analysis checklist:**

1. **File discovery overhead:**
   - Does `ignore` crate add significant overhead for Go projects?
   - Is vendor/ directory properly skipped early?

2. **GlobSet compilation:**
   - How does Go pattern count compare to Rust/Shell?
   - Go: 3 patterns (source, test, ignore)
   - Shell: 6 patterns (2 source, 4 test)
   - Rust: ~9 patterns

3. **Package enumeration:**
   - Is `enumerate_packages()` called unnecessarily?
   - Does recursive directory walk hit slow filesystems?

4. **Escape pattern matching:**
   - Go has 3 escape patterns (unsafe.Pointer, go:linkname, go:noescape)
   - Regex compilation happens per-check or cached?

5. **Nolint parsing:**
   - O(lines) complexity - acceptable
   - Regex vs string matching performance?

**Verification:**
```bash
# Run full check with verbose timing
RUST_LOG=debug cargo run -- check tests/fixtures/golang 2>&1 | grep -i "ms\|time\|duration"
```

### Phase 6: Document Findings

**Goal:** Create comprehensive performance report.

**File:** `reports/checkpoint-go-1-perf.md`

**Report template:**

```markdown
# Go Adapter Performance Report

Generated: YYYY-MM-DD
Hardware: [CPU, RAM, OS]

## Summary

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| go-simple cold | < 500ms | X.Xms | PASS/FAIL |
| go-simple warm | < 100ms | X.Xms | PASS/FAIL |
| go-multi cold | < 500ms | X.Xms | PASS/FAIL |
| go-multi warm | < 100ms | X.Xms | PASS/FAIL |
| GoAdapter::new() | < 100µs | X.Xµs | PASS/FAIL |
| classify() per file | < 0.1µs | X.Xµs | PASS/FAIL |
| enumerate_packages() | < 10ms | X.Xms | PASS/FAIL |
| parse_nolint 100 lines | < 10µs | X.Xµs | PASS/FAIL |

## Detailed Results

### 1. End-to-End Benchmarks

[Table with fixture, files, lines, cold/warm times]

### 2. Adapter Micro-Benchmarks

[Table with operation, time, comparison to Rust/Shell]

### 3. Comparison with Other Adapters

| Operation | Go | Rust | Shell |
|-----------|-----|------|-------|
| Adapter creation | | 62.3µs | 78.3µs |
| classify() per file | | 0.11µs | 0.043µs |
| Suppress parse/100 lines | | 5.7µs | 3.6µs |

### 4. Bottlenecks Identified

[List any operations > 100ms]
[List any operations > 10ms worth investigating]

### 5. Performance Breakdown

[Estimated time distribution for fixture checks]

## Flamegraph Analysis

[Key observations from flamegraph]
- Hot paths identified
- Unexpected time sinks

## Recommendations

1. [Optimization opportunity 1]
2. [Optimization opportunity 2]

## Conclusion

[Overall assessment of Go adapter performance]
[Priority optimizations if any bottlenecks found]
```

**Verification:**
```bash
test -f reports/checkpoint-go-1-perf.md && echo "Report created"
wc -l reports/checkpoint-go-1-perf.md  # Should have substantial content
```

## Key Implementation Details

### Go Adapter Architecture

The Go adapter (`crates/cli/src/adapter/go/mod.rs`) has three main operations:

1. **File Classification** - Uses GlobSet with 3 patterns:
   - `**/*.go` - source files
   - `**/*_test.go` - test files
   - `vendor/**` - ignored paths

2. **Package Enumeration** - Recursive directory traversal:
   - Finds directories containing `.go` files
   - Skips `vendor/` directory
   - Returns relative paths from module root

3. **Nolint Parsing** - Line-by-line string parsing:
   - Finds `//nolint` directives
   - Extracts linter codes
   - Checks for justification comments

### Comparison Baselines (from previous checkpoints)

From `reports/checkpoint-5d-benchmarks.md`:

| Adapter | Creation | Classify/file | Suppress/100 lines |
|---------|----------|---------------|-------------------|
| RustAdapter | 58.2µs | 0.11µs | 5.7µs |
| ShellAdapter | 78.3µs | 0.043µs | 3.6µs |
| GenericAdapter | 33.7µs | varies | N/A |

Expected Go adapter performance should be similar to Shell (simpler patterns, no inline test detection).

### Profiling Tools

1. **Criterion** - Statistical microbenchmarks (primary)
2. **Hyperfine** - CLI wall-clock timing (secondary)
3. **Flamegraph** - CPU profiling visualization (for hot path analysis)

## Verification Plan

### Phase 1 Verification
```bash
cargo bench --bench adapter -- go 2>&1 | head -50
# Should show Go benchmark results
```

### Phase 2 Verification
```bash
# Quick timing check
time cargo run --release -q -- check tests/fixtures/go-simple
# Should complete in < 0.5s
```

### Phase 3 Verification
```bash
ls flamegraph-go*.svg 2>/dev/null | wc -l
# Should show 1-3 flamegraph files
```

### Phase 4 Verification
```bash
cargo bench --bench adapter -- nolint 2>&1 | grep -E "time:|mean"
# Should show nolint parsing benchmarks
```

### Phase 5 Verification
```bash
# No operations should take > 100ms
cargo run --release -q -- check tests/fixtures/golang
# Should complete quickly without visible delays
```

### Phase 6 (Final) Verification
```bash
make check  # All tests pass
test -f reports/checkpoint-go-1-perf.md  # Report exists
grep -c "PASS\|FAIL" reports/checkpoint-go-1-perf.md  # Has results
```

## Exit Criteria

- [ ] Go adapter benchmarks added to `benches/adapter.rs`
- [ ] End-to-end timing measured for go-simple, go-multi
- [ ] Flamegraph generated for at least one fixture
- [ ] All operations complete in < 100ms (no critical bottlenecks)
- [ ] Performance compared to Rust/Shell adapter baselines
- [ ] Report created: `reports/checkpoint-go-1-perf.md`
- [ ] All tests pass: `make check`
