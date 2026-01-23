# Checkpoint 3E: Performance Fixes - Escapes Works

**Root Feature:** `quench-0dc9`

## Overview

Performance optimization checkpoint to address identified bottlenecks in the escapes check. While checkpoint 3D showed all performance targets are already met (78ms cold vs 500ms target, 14.5ms warm vs 100ms target), there are optimization opportunities that would improve scalability for large codebases:

1. **Aho-Corasick activation** - Currently only activates for pure alternations (`"TODO|FIXME|XXX"`), not for word-boundary patterns (`"\b(TODO|FIXME|XXX)\b"`) which fall back to regex
2. **File classification overhead** - Creates a new `GenericAdapter` per file (365x speedup potential with reuse)
3. **Re-validate benchmarks** - Confirm optimizations maintain or improve performance

## Project Structure

Key files:

```
quench/
├── crates/cli/
│   ├── benches/
│   │   └── check.rs              # End-to-end benchmarks
│   └── src/
│       ├── checks/
│       │   ├── escapes.rs        # classify_file() optimization
│       │   └── escapes_tests.rs  # Micro-benchmarks
│       └── pattern/
│           ├── matcher.rs        # Aho-Corasick expansion
│           └── matcher_tests.rs  # Matcher unit tests
├── reports/
│   └── checkpoint-3-benchmarks.md  # Update with new results
└── tests/fixtures/
    └── bench-medium/             # 530 files, ~58K LOC
```

## Dependencies

No new dependencies. Existing:
- `aho-corasick = "1"` - Already present for multi-literal matching
- `memchr` - Already present for literal matching
- `regex` - Already present for complex patterns
- `criterion` - Already present for benchmarks

## Implementation Phases

### Phase 1: Audit Aho-Corasick Activation

**Goal:** Understand exactly when aho-corasick is used vs regex fallback.

**Investigation:**
1. Add debug logging to `CompiledPattern::compile()` to trace matcher selection
2. Run escapes check with typical patterns to see which matcher is selected
3. Document which escape patterns use which matcher

Current activation logic in `matcher.rs`:

```rust
pub fn compile(pattern: &str) -> Result<Self, PatternError> {
    if is_literal(pattern) {
        Ok(CompiledPattern::Literal(LiteralMatcher::new(pattern)))
    } else if let Some(literals) = extract_alternation_literals(pattern) {
        Ok(CompiledPattern::MultiLiteral(MultiLiteralMatcher::new(&literals)?))
    } else {
        Ok(CompiledPattern::Regex(RegexMatcher::new(pattern)?))
    }
}
```

**Issue:** `extract_alternation_literals()` returns `None` for patterns with regex syntax like word boundaries:
- `"TODO|FIXME|XXX"` → Uses Aho-Corasick
- `"\b(TODO|FIXME|XXX)\b"` → Falls back to Regex

**Test:**
```bash
cargo test --package quench -- pattern --nocapture
# Verify matcher selection for typical patterns
```

**Milestone:** Document which patterns use Aho-Corasick vs Regex.

**Status:** [ ] Pending

---

### Phase 2: Evaluate Aho-Corasick Expansion (Optional)

**Goal:** Determine if expanding Aho-Corasick usage would provide measurable benefit.

**Analysis:**

Given that the current regex performance is 2.56µs per 100-line file and targets are exceeded by 390x, expanding Aho-Corasick may not be necessary. However, for documentation purposes:

**Potential approaches:**
1. **Extract literals + post-filter**: Use Aho-Corasick to find candidate matches, then apply word boundary checks in post-processing
2. **Hybrid matcher**: Combine Aho-Corasick for literal matching with boundary validation

**Example hybrid approach:**

```rust
/// Matcher that uses Aho-Corasick for literals with post-filter for word boundaries.
pub struct WordBoundaryMatcher {
    automaton: AhoCorasick,
    check_start_boundary: bool,
    check_end_boundary: bool,
}

impl WordBoundaryMatcher {
    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.automaton
            .find_iter(content)
            .filter(|m| self.is_word_boundary(content, m))
            .map(|m| PatternMatch { start: m.start(), end: m.end() })
            .collect()
    }

    fn is_word_boundary(&self, content: &str, m: &aho_corasick::Match) -> bool {
        let before_ok = !self.check_start_boundary ||
            m.start() == 0 || !content.as_bytes()[m.start() - 1].is_ascii_alphanumeric();
        let after_ok = !self.check_end_boundary ||
            m.end() >= content.len() || !content.as_bytes()[m.end()].is_ascii_alphanumeric();
        before_ok && after_ok
    }
}
```

**Decision criteria:**
- If pattern matching is <5% of total time, skip this optimization
- If pattern matching is >20% of total time, consider implementing

**Current state:** Pattern matching is 47% of warm time (~8ms of 17ms), but absolute time is already excellent. Likely not worth the complexity.

**Milestone:** Document decision: implement hybrid matcher or keep regex fallback.

**Status:** [ ] Pending

---

### Phase 3: Optimize File Classification

**Goal:** Reuse `GenericAdapter` across files instead of creating new one per file.

**Current code in `escapes.rs`:**

```rust
fn classify_file(path: &Path, root: &Path, test_patterns: &[String]) -> FileKind {
    let adapter = GenericAdapter::new(&[], test_patterns);  // NEW ADAPTER PER FILE
    let relative = path.strip_prefix(root).unwrap_or(path);
    adapter.classify(relative)
}
```

**Fix:** Create adapter once at check start and pass as parameter.

```rust
// In check() function, before the file loop:
let test_patterns = default_test_patterns();
let adapter = GenericAdapter::new(&[], &test_patterns);

// Changed signature:
fn classify_file(adapter: &GenericAdapter, path: &Path, root: &Path) -> FileKind {
    let relative = path.strip_prefix(root).unwrap_or(path);
    adapter.classify(relative)
}
```

**Benchmark impact:**
- Current: 46.3µs per file × 530 files = ~24ms overhead
- Optimized: 0.126µs per file × 530 files = ~0.07ms overhead
- Expected saving: ~24ms (significant for cold runs)

**Test:**
```bash
cargo test --package quench -- bench_file_classification --ignored --nocapture
# Before and after comparison
```

**Milestone:** File classification uses shared adapter.

**Status:** [ ] Pending

---

### Phase 4: Re-run Benchmarks

**Goal:** Validate optimizations and update benchmark report.

**Benchmark execution:**

```bash
# Build release
cargo build --release

# Clear cache for cold benchmark
rm -rf tests/fixtures/bench-medium/.quench

# Cold benchmark
hyperfine --warmup 0 --runs 5 \
    './target/release/quench check tests/fixtures/bench-medium'

# Warm benchmark
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-medium'

# Comparison
hyperfine --warmup 2 --runs 10 \
    './target/release/quench check tests/fixtures/bench-medium --escapes --no-limit' \
    './target/release/quench check tests/fixtures/bench-medium --cloc' \
    './target/release/quench check tests/fixtures/bench-medium --no-limit' \
    --export-markdown reports/escapes-vs-cloc.md
```

**Criterion benchmarks:**
```bash
cargo bench --bench check -- "bench-medium"
```

**Micro-benchmarks:**
```bash
cargo test --package quench -- bench_ --ignored --nocapture
```

**Expected results:**

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Cold (bench-medium) | 78.4ms | ~55ms | ~30% |
| Warm (bench-medium) | 14.5ms | ~12ms | ~17% |
| File classification | 46.3µs/file | 0.126µs/file | 365x |

**Milestone:** Updated benchmark results in `reports/checkpoint-3-benchmarks.md`.

**Status:** [ ] Pending

---

### Phase 5: Document Results

**Goal:** Update performance documentation with findings.

**Update `reports/checkpoint-3-benchmarks.md`:**
1. Add section for Phase 3E optimizations
2. Document Aho-Corasick activation status
3. Record before/after file classification performance
4. Update overall performance summary

**Template addition:**

```markdown
## Checkpoint 3E: Performance Fixes

### Aho-Corasick Activation

| Pattern | Matcher Used | Notes |
|---------|--------------|-------|
| `\.unwrap\(\)` | Regex | Contains escapes |
| `\b(TODO|FIXME|XXX)\b` | Regex | Has word boundaries |
| `TODO|FIXME|XXX` | Aho-Corasick | Pure alternation |
| `.expect(` | Literal | No metacharacters |

### File Classification Optimization

| Metric | Before | After |
|--------|--------|-------|
| Per-file overhead | 46.3µs | 0.126µs |
| Total (530 files) | ~24ms | ~0.07ms |

### Updated Performance

| Metric | Checkpoint 3D | Checkpoint 3E | Change |
|--------|---------------|---------------|--------|
| Cold | 78.4ms | XXms | -XX% |
| Warm | 14.5ms | XXms | -XX% |
```

**Milestone:** Documentation complete.

**Status:** [ ] Pending

## Key Implementation Details

### Pattern Matching Hierarchy

The three-tier optimization in `CompiledPattern`:

1. **LiteralMatcher** (memchr) - For patterns without regex metacharacters
2. **MultiLiteralMatcher** (Aho-Corasick) - For pure alternations
3. **RegexMatcher** (regex crate) - For everything else

Current behavior with typical escape patterns:

| Pattern | Type | Reason |
|---------|------|--------|
| `\.unwrap\(\)` | Regex | Has escapes and metacharacters |
| `\.expect\(` | Regex | Has escapes and metacharacters |
| `\bunsafe\b` | Regex | Has word boundaries |
| `\b(TODO\|FIXME\|XXX)\b` | Regex | Has word boundaries and groups |
| `TODO\|FIXME\|XXX` | Aho-Corasick | Pure alternation of literals |
| `FIXME` | Literal | Single literal |

### File Classification Flow

Current (O(n) adapter creation):
```
for each file:
    adapter = GenericAdapter::new(...)  // 46µs
    adapter.classify(file)              // 0.1µs
```

Optimized (O(1) adapter creation):
```
adapter = GenericAdapter::new(...)  // 46µs once
for each file:
    adapter.classify(file)          // 0.1µs
```

## Verification Plan

1. **Matcher selection audit:**
   ```bash
   # Add debug output to compile() and run:
   cargo test --package quench -- test_pattern_matcher_selection
   ```

2. **File classification benchmark:**
   ```bash
   cargo test --package quench -- bench_file_classification --ignored --nocapture
   ```

3. **End-to-end benchmarks:**
   ```bash
   hyperfine './target/release/quench check tests/fixtures/bench-medium'
   ```

4. **Quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Audit Aho-Corasick activation | [ ] Pending |
| 2 | Evaluate Aho-Corasick expansion (optional) | [ ] Pending |
| 3 | Optimize file classification | [ ] Pending |
| 4 | Re-run benchmarks | [ ] Pending |
| 5 | Document results | [ ] Pending |

## Notes

- Phase 2 is optional - regex performance is already excellent (390x under target)
- Phase 3 is the primary optimization with measurable impact
- All performance targets are already exceeded; these are scalability improvements
- If Phase 3 shows <10% improvement, consider skipping for simplicity
