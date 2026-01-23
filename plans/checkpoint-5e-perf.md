# Checkpoint 5E: Performance - Shell Adapter

**Root Feature:** `quench-68fa`

## Overview

Performance optimization checkpoint for the Shell adapter based on benchmark findings from checkpoint-5d. While overall Shell adapter performance exceeds all targets (14.1ms cold vs 500ms target), the 5D benchmark revealed an anomaly worth investigating:

> ShellAdapter::new() takes 78.3µs (6 patterns) vs RustAdapter::new() at 58.2µs (~9 patterns)
> Shell adapter creation is **26% slower** despite having fewer patterns.

This checkpoint investigates the root cause and implements optimizations if warranted. Given the excellent overall performance (35x headroom), this is an investigative optimization focused on understanding rather than necessity.

**Goals:**
1. Investigate why Shell adapter creation is slower than Rust
2. Identify optimization opportunities in GlobSet pattern construction
3. Implement low-cost improvements without over-engineering
4. Document findings for future adapter development

**Non-Goals:**
- Major architectural changes (current design is proven efficient)
- Premature optimization of sub-millisecond operations
- Adding complexity for marginal gains

## Project Structure

Key files involved:

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── adapter.rs              # Existing benchmarks
│   └── src/adapter/
│       ├── glob.rs                 # GlobSet builder (investigate)
│       ├── shell/
│       │   └── mod.rs              # Shell adapter patterns
│       └── rust/
│           └── mod.rs              # Rust adapter patterns (baseline)
├── reports/
│   ├── checkpoint-5d-benchmarks.md # Input: baseline numbers
│   └── checkpoint-5e-perf.md       # Output: optimization findings
└── plans/
    └── checkpoint-5e-perf.md       # This plan
```

## Dependencies

No new dependencies required. All optimization work uses existing tooling:
- `criterion` - Micro-benchmarking (existing)
- `globset` - Pattern matching (existing, may tune usage)

## Implementation Phases

### Phase 1: Profile GlobSet Pattern Compilation

**Goal:** Understand why Shell adapter creation (6 patterns) is slower than Rust (9 patterns).

**Investigation steps:**

1. Add targeted benchmarks for GlobSet pattern types:

```rust
// In benches/adapter.rs - add pattern-level benchmarks
fn bench_globset_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("globset_patterns");

    // Shell patterns
    group.bench_function("shell_source_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.sh").unwrap());
            builder.add(Glob::new("**/*.bash").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("shell_test_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("tests/**/*.bats").unwrap());
            builder.add(Glob::new("test/**/*.bats").unwrap());
            builder.add(Glob::new("*_test.sh").unwrap());
            builder.add(Glob::new("**/*_test.sh").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    // Rust patterns for comparison
    group.bench_function("rust_source_pattern", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("**/*.rs").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.bench_function("rust_test_patterns", |b| {
        b.iter(|| {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new("tests/**/*.rs").unwrap());
            builder.add(Glob::new("test/**/*.rs").unwrap());
            builder.add(Glob::new("*_test.rs").unwrap());
            builder.add(Glob::new("**/*_test.rs").unwrap());
            black_box(builder.build().unwrap())
        })
    });

    group.finish();
}
```

2. Analyze patterns for complexity differences:
   - `*.sh` vs `*.rs` extension matching
   - `.bats` extension (longer, less common)
   - Multiple wildcards (`**/*.bats` vs `**/*.rs`)

**Expected findings:**
- Shell patterns may require more regex compilation for `.bats`/`.bash` extensions
- The `globset` crate may optimize common extensions (`.rs`) differently
- Two separate GlobSet builds (source + test) vs potential optimizations in Rust adapter

**Verification:**
```bash
cargo bench --bench adapter -- globset_patterns
```

**Milestone:** Root cause identified with benchmark evidence.

**Status:** [ ] Pending

---

### Phase 2: Optimize Pattern Construction

**Goal:** Apply findings from Phase 1 to reduce Shell adapter creation time.

**Potential optimizations (depends on Phase 1 findings):**

**Option A: Single GlobSet with tagging**

If two separate GlobSet builds are the issue, combine into one:

```rust
// Current: Two GlobSets
pub struct ShellAdapter {
    source_patterns: GlobSet,  // Build 1
    test_patterns: GlobSet,    // Build 2
}

// Alternative: Single GlobSet with index-based classification
pub struct ShellAdapter {
    patterns: GlobSet,
    // First N patterns are source, rest are test
    source_count: usize,
}

impl ShellAdapter {
    pub fn new() -> Self {
        let mut builder = GlobSetBuilder::new();
        // Source patterns (index 0-1)
        builder.add(Glob::new("**/*.sh").unwrap());
        builder.add(Glob::new("**/*.bash").unwrap());
        // Test patterns (index 2-5)
        builder.add(Glob::new("tests/**/*.bats").unwrap());
        builder.add(Glob::new("test/**/*.bats").unwrap());
        builder.add(Glob::new("*_test.sh").unwrap());
        builder.add(Glob::new("**/*_test.sh").unwrap());

        Self {
            patterns: builder.build().unwrap(),
            source_count: 2,
        }
    }

    pub fn classify(&self, path: &Path) -> FileKind {
        let matches: Vec<usize> = self.patterns.matches(path);
        // Test patterns take precedence (index >= source_count)
        if matches.iter().any(|&i| i >= self.source_count) {
            return FileKind::Test;
        }
        if !matches.is_empty() {
            return FileKind::Source;
        }
        FileKind::Other
    }
}
```

**Option B: Pattern simplification**

Consolidate overlapping patterns if possible:

```rust
// Current: 4 test patterns
"tests/**/*.bats"
"test/**/*.bats"
"*_test.sh"
"**/*_test.sh"

// Could simplify "*_test.sh" + "**/*_test.sh" to just:
"**/*_test.sh"  // Already covers root-level files
```

**Option C: Lazy pattern compilation**

Defer GlobSet compilation until first use (only beneficial if adapter is often created but rarely used):

```rust
use once_cell::sync::Lazy;

static SHELL_SOURCE_PATTERNS: Lazy<GlobSet> = Lazy::new(|| {
    build_glob_set(&["**/*.sh".to_string(), "**/*.bash".to_string()])
});
```

**Decision criteria:**
- Option A: Use if separate builds are the bottleneck
- Option B: Use if patterns are unnecessarily complex
- Option C: Use if adapter creation happens frequently without use

**Verification:**
```bash
cargo bench --bench adapter -- adapter_creation
# Compare ShellAdapter::new() before/after
```

**Milestone:** Optimization implemented, adapter creation time reduced.

**Status:** [ ] Pending

---

### Phase 3: Validate Classification Performance

**Goal:** Ensure optimizations don't regress classification performance.

After any changes to pattern structure, validate that:

1. **Classification correctness** - All existing tests pass
2. **Classification speed** - No regression in per-file classification

**Validation steps:**

```bash
# Run all Shell adapter tests
cargo test -p quench --lib shell

# Run classification benchmarks
cargo bench --bench adapter -- shell_classify

# Expected results (from 5D baseline):
# shell_1k_source_scripts: ~43µs (0.043µs/file)
# shell_1k_bats_tests: ~43µs (0.043µs/file)
```

**Acceptance criteria:**
- All tests pass
- Classification time ≤ baseline (43µs per 1K files)
- If using single GlobSet (Option A), classification may be slightly slower due to index checking - acceptable if adapter creation improves significantly

**Milestone:** No performance regression in classification.

**Status:** [ ] Pending

---

### Phase 4: End-to-End Validation

**Goal:** Confirm overall performance remains within targets.

**Validation:**

```bash
cargo build --release

# Run end-to-end benchmarks
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-shell'

# Expected: ~12-14ms (matching 5D baseline)
```

**Acceptance criteria:**
- bench-shell cold: < 500ms (target)
- bench-shell warm: < 100ms (target)
- No regression from 5D baseline (~12.6ms warm)

**Milestone:** End-to-end performance validated.

**Status:** [ ] Pending

---

### Phase 5: Document Findings

**Goal:** Create performance report documenting investigation and results.

**Create:** `reports/checkpoint-5e-perf.md`

**Report structure:**

```markdown
# Checkpoint 5E: Performance Report - Shell Adapter

Generated: YYYY-MM-DD

## Investigation Summary

### Root Cause Analysis
[Why Shell adapter creation was slower despite fewer patterns]

### Optimization Applied
[What change was made, or why no change was needed]

## Results

| Metric | Before (5D) | After (5E) | Change |
|--------|-------------|------------|--------|
| ShellAdapter::new() | 78.3µs | XXµs | X% |
| classify() per 1K files | 43µs | XXµs | X% |
| bench-shell warm | 12.6ms | XXms | X% |

## Conclusions

[Key findings and recommendations for future adapter development]
```

**Milestone:** Report complete at `reports/checkpoint-5e-perf.md`.

**Status:** [ ] Pending

## Key Implementation Details

### GlobSet Internals

The `globset` crate compiles patterns into a regex-based automaton. Key factors affecting compilation time:

1. **Pattern complexity** - `**` wildcards are more expensive than `*`
2. **Extension matching** - Common extensions may have optimized paths
3. **Number of patterns** - Linear scaling with pattern count
4. **GlobSet builds** - Each `build()` call creates a new automaton

### Shell vs Rust Pattern Comparison

| Adapter | Source Patterns | Test Patterns | Builds |
|---------|-----------------|---------------|--------|
| Shell | `**/*.sh`, `**/*.bash` | 4 patterns | 2 |
| Rust | `**/*.rs` | 4 patterns + crate paths | 2 |

The Rust adapter has more patterns but may benefit from:
- Single extension (`*.rs`) vs two extensions (`*.sh`, `*.bash`)
- More common extension with potential crate-level optimization

### Optimization Trade-offs

| Approach | Creation Time | Classification Time | Complexity |
|----------|---------------|---------------------|------------|
| Current (two GlobSets) | 78µs | 43µs/1K | Low |
| Single GlobSet | ~50µs (est.) | ~50µs/1K (est.) | Medium |
| Lazy patterns | 0µs (deferred) | Same | Low |
| Pattern simplification | ~70µs (est.) | Same | Low |

Given the sub-millisecond times involved, simplicity should be preferred over marginal gains.

### When NOT to Optimize

The current Shell adapter creation time (78µs) is:
- 0.6% of total check time for bench-shell
- A one-time cost per check invocation
- Well within acceptable performance

Optimization should only proceed if:
1. Investigation reveals a clear, simple fix
2. The fix doesn't add significant complexity
3. The change provides educational value for future adapter development

## Verification Plan

1. **Phase 1 verification:**
   ```bash
   cargo bench --bench adapter -- globset_patterns
   ```

2. **Phase 2 verification:**
   ```bash
   cargo bench --bench adapter -- adapter_creation
   # Compare ShellAdapter::new() before/after
   ```

3. **Phase 3 verification:**
   ```bash
   cargo test -p quench --lib shell
   cargo bench --bench adapter -- shell_classify
   ```

4. **Phase 4 verification:**
   ```bash
   cargo build --release
   hyperfine './target/release/quench check tests/fixtures/bench-shell'
   ```

5. **Quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Profile GlobSet pattern compilation | [ ] Pending |
| 2 | Optimize pattern construction | [ ] Pending |
| 3 | Validate classification performance | [ ] Pending |
| 4 | End-to-end validation | [ ] Pending |
| 5 | Document findings | [ ] Pending |

## Notes

- This is an investigative optimization - findings may conclude "no change needed"
- Priority is understanding the behavior, not achieving specific speedup
- Any optimization must maintain code simplicity
- Results inform best practices for future language adapters
