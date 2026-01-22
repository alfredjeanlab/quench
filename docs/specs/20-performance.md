# Performance Specification

Performance is a core design constraint for Quench. This document defines a performance strategy, not a grab bag of techniques.

## Performance Targets

| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Fast checks (cold) | < 500ms | < 1s | > 2s |
| Fast checks (warm) | < 100ms | < 200ms | > 500ms |
| CI checks | < 5s | < 15s | > 30s |

**Cold vs Warm:**
- **Cold:** First run, no cache. File walking + reading + checking all files.
- **Warm:** Subsequent run, cache populated. Only re-check files with changed mtime.

Targets assume a typical 50K LOC codebase on modern hardware (4+ cores, SSD).

**The warm run is the common case.** Agents iterate repeatedly; cold runs happen once per session.

## Performance Model

Before optimizing, understand where time goes. For a quality linting tool:

```
Total Time = File Discovery + File Reading + Pattern Matching + Aggregation
```

**Typical breakdown (estimated):**

| Phase | % of Time | Dominant Factor |
|-------|-----------|-----------------|
| File discovery | 30-50% | Directory traversal, gitignore matching |
| File reading | 20-30% | I/O, filesystem latency |
| Pattern matching | 20-40% | CPU, pattern complexity |
| Aggregation/output | <5% | Negligible unless pathological |

**Key insight:** File discovery and reading are often the bottleneck, not pattern matching. Parallelizing traversal and avoiding unnecessary files matters more than micro-optimizing regex.

## Edge Cases & Design Constraints

These scenarios may not appear during development but must be handled correctly. Design for them upfront.

### Large File Counts

**Scenario:** Monorepos with 50K+ files, or accidentally scanning `node_modules`/`target`.

**Risks:**
- Memory exhaustion building file lists
- Slow gitignore matching (O(files × patterns))
- Output overwhelmed with violations

**Design constraints:**
- Never build unbounded in-memory file lists
- Stream file discovery, don't collect-then-process
- Gitignore filtering must happen during traversal, not after
- Violation output must be bounded (default: 15)

**Detection:** `--ci` mode should report file count; warn if > 10K files scanned.

### Large Files

**Scenario:** Generated code, minified bundles, vendored dependencies, binary files misidentified as text.

**Risks:**
- Memory exhaustion reading file into memory
- Slow pattern matching on multi-MB files
- Meaningless violations (line 50,000 of generated.rs)

**Design constraints:**
- Check file size before reading (from metadata)
- Hard limit: skip files > 10MB with a warning
- Soft limit: report files > 1MB as potential violations
- Use memory-mapped I/O for files > 64KB

**Thresholds:**

| Size | Strategy |
|------|----------|
| < 64KB | Direct read into buffer |
| 64KB - 1MB | Memory-mapped, full processing |
| 1MB - 10MB | Memory-mapped, report as oversized |
| > 10MB | Skip with warning, don't read |

### Pathological Patterns

**Scenario:** User-configured regex that causes catastrophic backtracking or matches entire files.

**Risks:**
- Single file takes minutes to process
- Regex engine memory explosion
- Appears to hang

**Design constraints:**
- Use only non-backtracking regex engines (Rust's `regex` crate is safe)
- Timeout per-file processing (default: 5s)
- Prefer literal matching over regex when possible
- Validate patterns at config load time

### Deep Directory Trees

**Scenario:** Deeply nested structures (generated, legacy, or pathological).

**Risks:**
- Stack overflow in recursive traversal
- File path length exceeds OS limits
- Symlink loops

**Design constraints:**
- Use iterative traversal, not recursive
- Limit directory depth (default: 100 levels)
- Detect and skip symlink loops
- The `ignore` crate handles these correctly

### Slow Filesystems

**Scenario:** Network mounts, cloud storage, encrypted filesystems, spinning disks.

**Risks:**
- I/O latency dominates, parallelism helps less
- Memory-mapped files may behave poorly
- Timeouts on individual file operations

**Design constraints:**
- Don't assume SSD latency
- Parallel I/O helps even on slow FS (hides latency)
- Memory-mapped I/O is still correct, just slower
- Per-file timeouts catch stuck operations

## Core Architecture

The minimal architecture that hits performance targets:

### 1. Parallel Gitignore-Aware File Walking

Use the [`ignore`](https://docs.rs/ignore) crate (from ripgrep):

```rust
use ignore::WalkBuilder;

WalkBuilder::new(root)
    .hidden(true)
    .git_ignore(true)
    .git_exclude(true)
    .max_depth(Some(100))       // Constraint: limit depth
    .threads(num_cpus::get())
    .build_parallel()
```

This single choice provides:
- Parallel directory traversal with work-stealing
- Gitignore filtering during traversal (not after)
- Symlink loop detection
- Respects `.gitignore`, `.ignore`, global ignores

**This is the highest-impact decision.** Everything else is secondary.

### 2. Streaming Pipeline with Early Termination

Don't collect all files, then process. Stream through a pipeline:

```
Walker → Filter → Check → Collect (bounded)
           ↓        ↓           ↓
        (size)   (limit)    (stop early)
```

```rust
let (tx, rx) = crossbeam_channel::bounded(1000);

// Producer: parallel file walking
walker.run(|| {
    Box::new(|entry| {
        if let Ok(entry) = entry {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let _ = tx.send(entry.into_path());
            }
        }
        WalkState::Continue
    })
});

// Consumer: parallel checking with early termination
let violations: Vec<_> = rx.iter()
    .par_bridge()
    .filter_map(|path| check_file(&path).ok())
    .flatten()
    .take(VIOLATION_LIMIT)  // Early termination
    .collect();
```

### 3. Size-Gated File Reading

Check size before reading:

```rust
fn read_file(path: &Path) -> Result<FileContent> {
    let meta = fs::metadata(path)?;
    let size = meta.len();

    if size > MAX_FILE_SIZE {
        return Err(FileTooLarge(size));
    }

    if size > MMAP_THRESHOLD {
        read_mmap(path)
    } else {
        read_direct(path)
    }
}
```

### 4. Pattern Matching Hierarchy

Analyze patterns at startup, use the fastest applicable matcher:

| Pattern Type | Matcher | Example |
|--------------|---------|---------|
| Single literal | `memchr::memmem` | `"TODO"` |
| Multiple literals | `aho-corasick` | `"unsafe"`, `"unwrap"` |
| Simple regex | `regex` with literals | `unsafe\s*\{` |
| Complex regex | `regex` | `(?i)fixme.*later` |

```rust
enum CompiledPattern {
    Literal(memchr::memmem::Finder<'static>),
    MultiLiteral(aho_corasick::AhoCorasick),
    Regex(regex::Regex),
}

impl CompiledPattern {
    fn from_config(pattern: &str) -> Self {
        if is_literal(pattern) {
            CompiledPattern::Literal(memmem::Finder::new(pattern))
        } else if is_alternation_of_literals(pattern) {
            CompiledPattern::MultiLiteral(AhoCorasick::new(extract_literals(pattern)))
        } else {
            CompiledPattern::Regex(Regex::new(pattern).unwrap())
        }
    }
}
```

### 5. Bounded Output

Never produce unbounded output:

```rust
const DEFAULT_VIOLATION_LIMIT: usize = 15;

// Fast mode: stop early
violations.into_iter().take(limit).collect()

// CI mode: collect all, but stream output
for (i, v) in violations.iter().enumerate() {
    if i < display_limit {
        println!("{}", v);
    }
}
if violations.len() > display_limit {
    println!("... and {} more", violations.len() - display_limit);
}
```

## Primary Use Case: Iterative Development

Quench is designed for AI agents iterating on code fixes:

```
Agent runs quench → finds violations → fixes code → runs quench again → repeat
```

This means **most runs are re-runs where few files changed**. A 500ms cold run is acceptable, but subsequent runs should be much faster since 95%+ of files are unchanged.

**Implication:** Caching is not a micro-optimization—it's essential for the core use case.

## Optimization Backlog

Ordered by impact for the iterative development use case.

### P0: File-Level Caching (Implement Early)

**Why P0:** Directly serves the primary use case. Most files don't change between runs.

**Strategy:** Cache check results keyed by `(path, mtime, size)`:

```rust
struct FileCache {
    // path → (mtime, size, check_result)
    cache: DashMap<PathBuf, CachedResult>,
}

struct CachedResult {
    mtime: SystemTime,
    size: u64,
    violations: Vec<Violation>,
}

fn check_file_cached(path: &Path, cache: &FileCache) -> Vec<Violation> {
    let meta = fs::metadata(path)?;

    if let Some(cached) = cache.get(path) {
        if cached.mtime == meta.modified()? && cached.size == meta.len() {
            return cached.violations.clone();
        }
    }

    let violations = check_file_uncached(path)?;
    cache.insert(path.clone(), CachedResult {
        mtime: meta.modified()?,
        size: meta.len(),
        violations: violations.clone(),
    });
    violations
}
```

**Cache location:** In-memory for single session. Optionally persist to `.quench/cache.bin` for cross-session caching.

**Cache invalidation:**
- File mtime changed → re-check
- File size changed → re-check
- Config changed → invalidate all
- Quench version changed → invalidate all

**Expected impact:** 10x speedup on iterative runs (500ms → 50ms).

### P1: Apply When File Walking is Slow

### P1: Apply When File Walking is Slow

**Symptom:** >50% of time in file discovery on large repos.

**Options:**
- Increase parallelism (more threads)
- Cache file list across runs
- Pre-filter by file extension before gitignore matching

### P2: Apply When Pattern Matching is Slow

**Symptom:** >50% of time in pattern matching, especially with many patterns.

**Options:**
- Combine patterns into single Aho-Corasick automaton
- Extract literal prefixes for prefiltering
- Parallelize across patterns (not just files)

### P3: Apply When Memory is Constrained

**Symptom:** High memory usage on large repos or in CI.

**Options:**
- Reduce mmap threshold (more streaming, less resident memory)
- Use bounded cache with eviction (`moka` instead of `DashMap`)
- Process files in batches, not all at once

### P4: Micro-Optimizations (Likely Not Needed)

**Apply only if profiling shows specific bottleneck:**

- String interning (`lasso`) for repeated paths
- Arena allocation (`bumpalo`) for per-file temporaries
- Small string optimization (`smol_str`) for short strings
- Profile-guided optimization (PGO)

**Don't use Salsa.** Salsa is for IDE/LSP tools (rust-analyzer) with complex inter-file dependencies. CLI linters (ripgrep, ruff, biome) don't use it. Simple file-level caching is the right granularity for Quench.

## Memory Strategy

### Budget

| Mode | Target | Hard Limit |
|------|--------|------------|
| Fast checks | < 100MB | 500MB |
| CI checks | < 500MB | 2GB |

### Core Principle: Don't Buffer What You Can Stream

```rust
// ❌ Collects all files into memory
let files: Vec<_> = walker.collect();
let violations: Vec<_> = files.par_iter().flat_map(check).collect();

// ✅ Streams files through pipeline
walker.run(|| check_and_emit);
```

### When to Use What

| Need | Solution |
|------|----------|
| Concurrent cache, unbounded | `DashMap` |
| Concurrent cache, bounded | `moka` with max_capacity |
| Per-file temporary allocations | `bumpalo` arena (if profiling shows benefit) |
| Repeated strings | `lasso` interner (if profiling shows benefit) |

**Default:** Use simple owned types (`String`, `Vec`, `PathBuf`). Only reach for specialized allocators when measurement proves they help.

### Arc vs Clone Guidelines

Incorrect sharing causes bugs (data races, deadlocks) and perf issues (contention, cache thrashing). Follow these rules:

**Use `Arc<T>` for:**
- Config and compiled patterns (immutable, shared across all threads)
- Cached file content (read by multiple checks)

**Use owned types for:**
- Violations (collected per-file, merged at end)
- Metrics (accumulated per-thread, reduced once)
- Intermediate results (don't outlive the check)

**Anti-patterns:**

```rust
// ❌ Shared mutable state - causes contention
let violations = Arc<Mutex<Vec<Violation>>>::new(...);
files.par_iter().for_each(|f| {
    violations.lock().push(check(f));  // Lock on every file!
});

// ✅ Owned per-thread, merge at end
let violations: Vec<_> = files
    .par_iter()
    .map(|f| check(f))
    .flatten()
    .collect();
```

```rust
// ❌ Cloning large data per-file
files.par_iter().for_each(|f| {
    let patterns = config.patterns.clone();  // Clones Vec of regex!
    check(f, &patterns);
});

// ✅ Share immutable data via Arc
let patterns = Arc::new(compile_patterns(&config));
files.par_iter().for_each(|f| {
    check(f, &patterns);  // Just increments refcount
});
```

**Rule of thumb:** If it's immutable and used by multiple threads, wrap in `Arc`. If it's accumulated/mutated, keep it owned and merge at the end.

## Build Configuration

### Release Profile

```toml
[profile.release]
lto = "thin"           # Link-time optimization
codegen-units = 1      # Better optimization
panic = "abort"        # Smaller binary
strip = true           # Strip symbols
```

### Target Features (Optional)

For maximum performance on specific CPUs:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

For portable binaries, omit this (the regex crate does runtime SIMD detection).

## Benchmarking

### What to Measure

1. **End-to-end time** on representative repos (small, medium, large)
2. **File discovery time** (walker only, no checking)
3. **Pattern matching time** (synthetic benchmark)
4. **Memory high-water mark**

### Representative Test Cases

| Case | Files | LOC | Characteristics |
|------|-------|-----|-----------------|
| Small project | 50 | 5K | Simple structure |
| Medium project | 500 | 50K | Typical monorepo package |
| Large project | 5K | 500K | Full monorepo |
| Stress: many files | 50K | 500K | Flat structure, many small files |
| Stress: large files | 100 | 500K | Few files, several >1MB |
| Stress: deep nesting | 1K | 50K | 50+ levels deep |

### Profiling

```bash
# Linux
perf record -g ./target/release/quench check /path/to/repo
perf report

# macOS
xcrun xctrace record --template 'Time Profiler' --launch ./target/release/quench check /path/to/repo

# Flame graph
cargo install flamegraph
flamegraph -- ./target/release/quench check /path/to/repo
```

**Profile before optimizing.** The bottleneck is rarely where you expect.

## Summary

**The primary use case:** Agent iterating on fixes, running quench repeatedly on the same branch.

**The strategy:**

1. **Parallel gitignore-aware walking** (`ignore` crate) - highest impact for cold runs
2. **File-level caching** by mtime/size - highest impact for warm runs (the common case)
3. **Streaming pipeline** with early termination - handles scale
4. **Size-gated file reading** - handles edge cases
5. **Pattern matching hierarchy** - literal → aho-corasick → regex
6. **Bounded output** - protects against pathological cases

**The edge cases** (design for these upfront):
- Large file counts: stream, don't collect
- Large files: check size, skip >10MB
- Pathological patterns: non-backtracking regex, timeouts
- Deep trees: iterative traversal, depth limit
- Slow filesystems: parallel I/O, per-file timeouts

**The optimization approach:**
- File caching is P0—it serves the core use case, not a micro-optimization
- Everything else: measure first, optimize second
- Start simple (owned types, no fancy allocators)
- Apply P1+ backlog items only when profiling justifies them

## References

- [ripgrep is faster than {grep, ag, git grep, ucg, pt, sift}](https://blog.burntsushi.net/ripgrep/)
- [Regex engine internals as a library](https://burntsushi.net/regex-internals/)

### Key Crates

| Crate | Purpose | When to Use |
|-------|---------|-------------|
| `ignore` | Parallel gitignore-aware walking | Always (core architecture) |
| `rayon` | Data parallelism | Always (core architecture) |
| `memchr` | SIMD byte/substring search | Literal patterns |
| `aho-corasick` | Multi-pattern matching | Multiple literal patterns |
| `regex` | Regular expressions | Complex patterns |
| `memmap2` | Memory-mapped file I/O | Files > 64KB |
| `moka` | Bounded concurrent cache | When memory constrained |
| `dashmap` | Unbounded concurrent map | Simple caching needs |
| `bumpalo` | Arena allocation | Per-file temps (if profiled) |
| `crossbeam-channel` | Multi-producer channels | Streaming pipeline |
