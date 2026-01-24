# Checkpoint 7E: Performance - Docs Check

**Root Feature:** `quench-0862`
**Depends on:** `checkpoint-7d-benchmark` (benchmarks must exist first)

## Overview

Optimize the docs check for performance based on benchmark analysis. The current implementation has several inefficiencies:

1. **Regex recompilation** - Link pattern compiled on every file
2. **Sequential processing** - Files processed one at a time
3. **Redundant path checks** - Same paths resolved multiple times
4. **No caching** - Docs check doesn't use the file cache infrastructure
5. **Eager file reading** - Content loaded before filter checks

**Performance Targets** (from `docs/specs/20-performance.md`):
| Mode | Target | Acceptable | Unacceptable |
|------|--------|------------|--------------|
| Fast checks (cold) | < 500ms | < 1s | > 2s |
| Fast checks (warm) | < 100ms | < 200ms | > 500ms |
| CI checks | < 5s | < 15s | > 30s |

## Project Structure

```
crates/cli/src/
├── checks/docs/
│   ├── mod.rs           # MODIFY: Parallel processing, cache integration
│   ├── links.rs         # MODIFY: Lazy-static regex compilation
│   ├── specs.rs         # MODIFY: Path existence caching
│   ├── toc/
│   │   ├── mod.rs       # MODIFY: Minor optimizations
│   │   └── resolve.rs   # MODIFY: Glob result caching
│   └── commit.rs        # Unchanged (CI-only, uses git subprocess)
└── cache.rs             # MODIFY: Add docs check fields to CachedViolation
```

## Dependencies

No new dependencies required. Uses existing:
- `rayon` (already available via `ignore` crate)
- `dashmap` (already a dependency)
- `once_cell` or `std::sync::LazyLock` (Rust 1.80+) for lazy statics

## Implementation Phases

### Phase 1: Lazy Regex Compilation

Compile the link extraction regex once globally instead of per-file.

**File: `crates/cli/src/checks/docs/links.rs`**

```rust
use std::sync::LazyLock;

/// Pre-compiled regex for markdown link extraction.
static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(LINK_PATTERN).expect("valid regex pattern")
});

/// Extract all markdown links from content, skipping links inside fenced code blocks.
pub(super) fn extract_links(content: &str) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    let mut in_fenced_block = false;

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx as u32 + 1;
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_fenced_block = !in_fenced_block;
            continue;
        }

        if in_fenced_block {
            continue;
        }

        // Use pre-compiled regex
        for cap in LINK_REGEX.captures_iter(line) {
            if let Some(target) = cap.get(1) {
                links.push(ExtractedLink {
                    line: line_num,
                    target: target.as_str().to_string(),
                });
            }
        }
    }
    links
}
```

**Verification**: Run `cargo bench --bench docs links` and compare against baseline.

---

### Phase 2: Path Existence Cache

Add a per-run path existence cache to avoid redundant filesystem checks.

**File: `crates/cli/src/checks/docs/mod.rs`**

```rust
use std::sync::Arc;
use dashmap::DashMap;

/// Per-run cache for path existence checks.
///
/// Shared across all docs sub-checks to avoid redundant filesystem calls.
pub(super) struct PathCache {
    /// Maps canonical paths to existence result.
    exists: DashMap<PathBuf, bool>,
}

impl PathCache {
    pub fn new() -> Self {
        Self {
            exists: DashMap::new(),
        }
    }

    /// Check if a path exists, using cache.
    pub fn exists(&self, path: &Path) -> bool {
        // Use path as-is for cache key (canonicalization is expensive)
        if let Some(result) = self.exists.get(path) {
            return *result;
        }
        let result = path.exists();
        self.exists.insert(path.to_path_buf(), result);
        result
    }

    /// Pre-populate cache with known existing files.
    pub fn populate(&self, files: &[&crate::walker::WalkedFile]) {
        for file in files {
            self.exists.insert(file.path.clone(), true);
        }
    }
}
```

**Update validators** to accept `&PathCache`:

```rust
// In links.rs
fn validate_file_links(
    ctx: &CheckContext,
    relative_path: &Path,
    content: &str,
    violations: &mut Vec<Violation>,
    path_cache: &PathCache,  // NEW
) {
    // ...
    if !path_cache.exists(&resolved) {  // Use cache
        violations.push(...);
    }
}
```

**Verification**: Run `cargo bench --bench docs e2e_stress` on `many-links` fixture.

---

### Phase 3: Parallel File Processing

Process markdown files in parallel using rayon.

**File: `crates/cli/src/checks/docs/mod.rs`**

```rust
use rayon::prelude::*;
use std::sync::Mutex;

/// Process markdown files matching include/exclude patterns in parallel.
pub(super) fn process_markdown_files_parallel<F>(
    ctx: &CheckContext,
    include: &[String],
    exclude: &[String],
    path_cache: &PathCache,
    validator: F,
) -> Vec<Violation>
where
    F: Fn(&CheckContext, &Path, &str, &PathCache) -> Vec<Violation> + Sync,
{
    let include_set = build_glob_set(include);
    let exclude_set = build_glob_set(exclude);

    // Collect matching files first (fast filter pass)
    let matching_files: Vec<_> = ctx.files
        .iter()
        .filter(|walked| {
            let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
            let path_str = relative_path.to_string_lossy();
            include_set.is_match(&*path_str) && !exclude_set.is_match(&*path_str)
        })
        .collect();

    // Process in parallel
    let violations: Vec<Violation> = matching_files
        .par_iter()
        .flat_map(|walked| {
            let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);

            // Read file content
            let content = match std::fs::read_to_string(&walked.path) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            validator(ctx, relative_path, &content, path_cache)
        })
        .collect();

    violations
}
```

**Update DocsCheck::run**:

```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    // Create shared path cache
    let path_cache = PathCache::new();
    path_cache.populate(ctx.files);

    // Collect violations from parallel checks
    let mut violations = Vec::new();

    // TOC validation (parallel)
    if is_toc_enabled(ctx) {
        violations.extend(toc::validate_toc_parallel(ctx, &path_cache));
    }

    // Link validation (parallel)
    if is_links_enabled(ctx) {
        violations.extend(links::validate_links_parallel(ctx, &path_cache));
    }

    // Specs validation (may use path cache internally)
    if is_specs_enabled(ctx) {
        specs::validate_specs(ctx, &mut violations, &path_cache);
    }

    // Commit validation (CI mode only, not parallelized - uses git subprocess)
    if ctx.ci_mode {
        commit::validate_commit_docs(ctx, &mut violations);
    }

    // ...
}
```

**Verification**: Run `cargo bench --bench docs e2e_stress` and verify >2x improvement on `many-files`.

---

### Phase 4: Specs Validation Optimization

Optimize BFS traversal in linked mode and TOC parsing.

**File: `crates/cli/src/checks/docs/specs.rs`**

```rust
/// Validate specs using linked mode with batched I/O.
fn validate_linked_mode(
    root: &Path,
    index_file: &Path,
    specs_dir: &Path,
    all_specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
    limit: Option<usize>,
    path_cache: &PathCache,  // NEW
) {
    // ... setup ...

    // Pre-collect all spec file paths for faster lookup
    // (HashSet::contains is O(1) vs file system check)

    // BFS optimization: batch file reads when queue is large
    const BATCH_SIZE: usize = 16;

    while !queue.is_empty() {
        // Process in batches for better locality
        let batch: Vec<_> = queue.drain(..queue.len().min(BATCH_SIZE)).collect();

        for current in batch {
            // ... existing logic with path_cache.exists() ...
        }
    }
}
```

**Optimization for collect_spec_files**:

```rust
/// Collect all spec files in the specs directory.
/// Returns canonicalized paths for consistent comparison.
fn collect_spec_files(
    root: &Path,
    specs_path: &str,
    extension: &str,
    path_cache: &PathCache,
) -> HashSet<PathBuf> {
    let specs_dir = root.join(specs_path);

    // Use walker's parallel mode for large directories
    let walker = ignore::WalkBuilder::new(&specs_dir)
        .threads(num_cpus::get().min(4))  // Limit threads for subdirectory walk
        .build_parallel();

    let results = DashMap::new();

    walker.run(|| {
        let results = &results;
        let extension = extension;
        Box::new(move |entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_some_and(|t| t.is_file())
                    && matches_extension(entry.path(), extension)
                {
                    if let Ok(canonical) = entry.path().canonicalize() {
                        results.insert(canonical, ());
                    }
                }
            }
            ignore::WalkState::Continue
        })
    });

    results.into_iter().map(|(k, _)| k).collect()
}
```

**Verification**: Run `cargo bench --bench docs specs` on `many-files` and `deep-links` fixtures.

---

### Phase 5: Cache Integration

Integrate docs check with the file cache infrastructure.

**File: `crates/cli/src/cache.rs`**

Add docs-specific fields to `CachedViolation`:

```rust
/// Minimal violation data for cache storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedViolation {
    // ... existing fields ...

    /// Path in TOC/link that was broken (for docs violations).
    pub target_path: Option<String>,
}
```

**File: `crates/cli/src/checks/docs/mod.rs`**

Add cache lookup before processing:

```rust
fn process_file_with_cache<F>(
    ctx: &CheckContext,
    file: &WalkedFile,
    check_name: &str,
    validator: F,
) -> Vec<Violation>
where
    F: FnOnce(&str) -> Vec<Violation>,
{
    let cache_key = FileCacheKey::from_walked_file(file);

    // Check cache first
    if let Some(cached) = ctx.cache.lookup(&file.path, &cache_key) {
        let check_violations: Vec<_> = cached
            .iter()
            .filter(|v| v.check == check_name)
            .map(|v| v.to_violation(file.path.clone()))
            .collect();
        return check_violations;
    }

    // Cache miss: read and validate
    let content = match std::fs::read_to_string(&file.path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let violations = validator(&content);

    // Store in cache
    let cached: Vec<_> = violations
        .iter()
        .map(|v| CachedViolation::from_violation(v, check_name))
        .collect();
    ctx.cache.insert(file.path.clone(), cache_key, cached);

    violations
}
```

**Update CACHE_VERSION** in `cache.rs`:

```rust
/// v18: Added target_path for docs cache.
pub const CACHE_VERSION: u32 = 18;
```

**Verification**: Run warm benchmarks and verify cache hit rates > 90%.

---

### Phase 6: Final Optimizations

Apply micro-optimizations based on profiling.

**A. Pre-filter markdown files in walker**

```rust
// In walker.rs or check context setup
// Only walk .md files for docs check
let md_files: Vec<_> = ctx.files
    .iter()
    .filter(|f| f.path.extension().map(|e| e == "md").unwrap_or(false))
    .collect();
```

**B. String interning for repeated paths** (if profiling shows benefit)

```rust
use lasso::{Rodeo, Spur};

/// Shared string interner for path components.
static PATH_INTERNER: LazyLock<Mutex<Rodeo>> = LazyLock::new(|| Mutex::new(Rodeo::new()));
```

**C. Early termination with atomic limit**

```rust
use std::sync::atomic::AtomicUsize;

/// Process files with atomic violation limit.
fn process_with_limit(
    files: &[&WalkedFile],
    limit: usize,
    processor: impl Fn(&WalkedFile) -> Vec<Violation> + Sync,
) -> Vec<Violation> {
    let count = AtomicUsize::new(0);

    files.par_iter()
        .filter(|_| count.load(Ordering::Relaxed) < limit)
        .flat_map(|f| {
            let v = processor(f);
            count.fetch_add(v.len(), Ordering::Relaxed);
            v
        })
        .take(limit)
        .collect()
}
```

**Verification**: Full benchmark suite, compare against baseline.

## Key Implementation Details

### Thread Safety

- Use `Arc<PathCache>` for shared access across parallel iterators
- Use `DashMap` for concurrent path existence cache
- Collect violations into thread-local vectors, merge at end

### Glob Matching

Pre-compile glob patterns once per check run:

```rust
let include_set = build_glob_set(include);  // Compile once
let exclude_set = build_glob_set(exclude);  // Compile once
// Then use in filter
```

### File Reading Strategy

Follow the performance spec guidelines:

| Size | Strategy |
|------|----------|
| < 64KB | Direct read into buffer |
| 64KB - 1MB | Memory-mapped, full processing |
| > 1MB | Skip with warning for docs check |

Markdown files are typically small, so direct reads are fine.

### Cache Invalidation

The file cache uses mtime+size as the key. For docs check:
- Cache key: file mtime + size
- Cache value: list of violations
- Invalidation: automatic on file modification

## Verification Plan

### Phase 1 Verification
```bash
# Verify lazy regex compiles correctly
cargo test -p quench --lib checks::docs::links

# Benchmark link extraction
cargo bench --bench docs link
# Expect: No regression (compilation moved to startup)
```

### Phase 2 Verification
```bash
# Test path cache
cargo test -p quench --lib checks::docs::mod

# Benchmark with many links
cargo bench --bench docs e2e_stress -- many-links
# Expect: >20% improvement due to cached existence checks
```

### Phase 3 Verification
```bash
# Verify parallel processing produces same results
cargo test -p quench --lib checks::docs

# Benchmark parallel processing
cargo bench --bench docs e2e_stress -- many-files
# Expect: >2x improvement on multi-core systems
```

### Phase 4 Verification
```bash
# Verify specs validation correctness
cargo test -p quench --lib checks::docs::specs

# Benchmark specs modes
cargo bench --bench docs specs
# Expect: BFS traversal scales linearly with depth
```

### Phase 5 Verification
```bash
# Verify cache integration
cargo test -p quench --lib cache

# Benchmark warm runs
# First run (cold)
cargo bench --bench docs e2e_small
# Second run (warm) - should show cache benefits
cargo bench --bench docs e2e_small

# Expect: >5x improvement on warm runs
```

### Phase 6 Verification
```bash
# Full benchmark suite
cargo bench --bench docs

# Generate comparison report
cargo bench --bench docs -- --save-baseline perf-optimized

# Profile to verify no new bottlenecks
cargo flamegraph --bench docs -- --bench
```

### Full Verification
```bash
# Run complete test suite
make check

# Verify against performance targets
# many-files: < 500ms (fast mode)
# deep-links: < 500ms (fast mode)
# warm runs: < 100ms
```

## Success Criteria

1. All existing tests pass
2. Benchmark improvements vs baseline:
   - Link extraction: No regression
   - Path resolution: >20% improvement
   - Parallel processing: >2x improvement on 4+ core systems
   - Warm runs: >5x improvement (cache hits)
3. End-to-end benchmarks within performance targets:
   - Small fixtures (cold): < 100ms
   - 500-file fixture (cold): < 500ms
   - Warm runs: < 100ms
4. No increase in memory usage beyond cache overhead
5. CACHE_VERSION bumped to 18

## Rollback Plan

If performance regressions are introduced:
1. Each phase is independently revertable
2. Feature flags can disable parallel processing:
   ```rust
   if cfg!(feature = "no-parallel") {
       process_markdown_files_sequential(...)
   } else {
       process_markdown_files_parallel(...)
   }
   ```
3. Cache integration can be disabled via config
