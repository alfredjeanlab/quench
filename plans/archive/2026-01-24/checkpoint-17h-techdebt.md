# Checkpoint 17H: Tech Debt - Performance

**Plan:** `checkpoint-17h-techdebt`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17G (Bug Fixes - Performance)

## Overview

Address performance-related tech debt identified in the codebase. Focus areas:

1. **Redundant allocations** - Double clone in pattern compilation, inefficient string conversions
2. **Memory-mapped I/O** - Spec defines thresholds but mmap not implemented
3. **Pre-allocation sizing** - Document rationale for capacity estimates
4. **Code cleanup** - Remove dead code paths, clarify performance-critical sections

**Current State:**
- All tests pass
- `make check` passes
- Performance targets exceeded (124ms cold, 44ms warm per 17G)
- MMAP_THRESHOLD defined but unused
- Several unnecessary allocations in startup path

**Non-Goals (deferred to future checkpoints):**
- Bounded cache with eviction (P3 from spec - only when memory constrained)
- Per-file timeout (edge case - non-backtracking regex handles pathological patterns)
- String interning (P4 from spec - micro-optimization)

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── pattern/
│   │   ├── matcher.rs           # UPDATE: Fix Box::leak double clone
│   │   └── matcher_tests.rs     # UPDATE: Add test for pattern memory
│   ├── file_reader.rs           # CREATE: Centralized file reading with mmap
│   ├── file_size.rs             # VERIFY: Thresholds used correctly
│   ├── walker.rs                # VERIFY: Uses size_class for hints
│   ├── checks/
│   │   ├── cloc.rs              # UPDATE: Use file_reader
│   │   ├── escapes/mod.rs       # UPDATE: Use file_reader
│   │   ├── placeholders/mod.rs  # UPDATE: Use file_reader
│   │   └── docs/mod.rs          # UPDATE: Use file_reader
│   └── adapter/javascript/
│       └── workspace.rs         # UPDATE: Fix string allocations
└── Cargo.toml                   # UPDATE: Add memmap2 dependency
```

## Dependencies

**New:**
- `memmap2` - Memory-mapped file I/O (recommended in spec)

**Existing:**
- `memchr` - Already used for pattern matching

## Implementation Phases

### Phase 1: Fix Box::leak Double Clone

**Goal:** Eliminate redundant allocation in pattern compilation.

**Problem:** `matcher.rs:154-155` allocates twice:

```rust
// Current (wasteful):
let pattern_owned = pattern.to_string();
let pattern_static: &'static str = Box::leak(pattern_owned.clone().into_boxed_str());
```

**Fix:**

```rust
// Fixed (single allocation):
let pattern_owned = pattern.to_string();
let pattern_static: &'static str = Box::leak(pattern_owned.clone().into_boxed_str());
// Wait - we need pattern_owned for the struct field too
// Actually we should clone into_boxed_str directly from the source:
let boxed: Box<str> = pattern.into();
let pattern_static: &'static str = Box::leak(boxed);
// And create pattern_owned separately if needed
```

Actually, looking more carefully:
```rust
// Current code stores both:
Self {
    pattern: pattern_owned,  // Needs String for display/debug
    finder: Finder::new(pattern_static),  // Needs &'static str
}
```

The fix is to avoid cloning before leak:
```rust
pub fn new(pattern: &str) -> Self {
    // Leak first, then clone for the stored String
    let pattern_static: &'static str = Box::leak(pattern.to_string().into_boxed_str());
    Self {
        pattern: pattern_static.to_string(),  // Clone from leaked
        finder: Finder::new(pattern_static),
    }
}
```

Wait, that's still two allocations. The issue is we need both owned and static. Let's do:
```rust
pub fn new(pattern: &str) -> Self {
    // Single allocation that gets leaked
    let pattern_static: &'static str = Box::leak(pattern.to_string().into_boxed_str());
    Self {
        // Store the static reference directly instead of cloning
        pattern: String::new(), // or remove field if unused
        finder: Finder::new(pattern_static),
    }
}
```

Actually the cleanest fix: change `pattern: String` to `pattern: &'static str`:

**File:** `crates/cli/src/pattern/matcher.rs`

**Before:**
```rust
pub struct LiteralMatcher {
    pattern: String,
    finder: Finder<'static>,
}

impl LiteralMatcher {
    pub fn new(pattern: &str) -> Self {
        let pattern_owned = pattern.to_string();
        let pattern_static: &'static str = Box::leak(pattern_owned.clone().into_boxed_str());
        Self {
            pattern: pattern_owned,
            finder: Finder::new(pattern_static),
        }
    }
}
```

**After:**
```rust
pub struct LiteralMatcher {
    pattern: &'static str,
    finder: Finder<'static>,
}

impl LiteralMatcher {
    pub fn new(pattern: &str) -> Self {
        let pattern_static: &'static str = Box::leak(pattern.to_string().into_boxed_str());
        Self {
            pattern: pattern_static,
            finder: Finder::new(pattern_static),
        }
    }
}
```

**Verification:**
```bash
cargo test -p quench -- matcher
```

---

### Phase 2: Fix String Allocation Patterns

**Goal:** Eliminate inefficient string conversions in workspace detection.

**Problem:** `workspace.rs:150` uses double conversion:

```rust
// Current (two allocations):
let dir_name = entry.file_name().to_string_lossy().to_string();
```

**Fix:**
```rust
// Fixed (one allocation for lossy case, zero for valid UTF-8):
let dir_name = entry.file_name().to_string_lossy().into_owned();
```

**File:** `crates/cli/src/adapter/javascript/workspace.rs`

**Change 1:** Line 150
```rust
// Before:
let dir_name = entry.file_name().to_string_lossy().to_string();

// After:
let dir_name = entry.file_name().to_string_lossy().into_owned();
```

**Change 2:** Lines 151-153 - Document unavoidable clone:
```rust
// format!() creates a new String, clone is unavoidable
// (need owned value for both Vec and HashMap)
let rel_path = format!("{}/{}", base, dir_name);
paths.push(rel_path.clone());
names.insert(rel_path, dir_name);
```

**Verification:**
```bash
cargo test -p quench -- workspace
```

---

### Phase 3: Add Memory-Mapped File Reading

**Goal:** Implement mmap for files >64KB as specified in performance spec.

**Problem:** All file reading uses `std::fs::read_to_string()`:
- `checks/cloc.rs:102`
- `checks/escapes/mod.rs:225`
- `checks/placeholders/mod.rs:64`
- `checks/docs/mod.rs:100`

The spec (`docs/specs/20-performance.md:74`) says:
> Use memory-mapped I/O for files > 64KB

**Solution:** Create centralized `file_reader.rs` module.

**Step 3a:** Add memmap2 dependency

**File:** `crates/cli/Cargo.toml`
```toml
[dependencies]
memmap2 = "0.9"
```

**Step 3b:** Create file_reader module

**File:** `crates/cli/src/file_reader.rs`
```rust
//! Centralized file reading with size-based strategy.
//!
//! Per docs/specs/20-performance.md:
//! - < 64KB: Direct read into buffer
//! - >= 64KB: Memory-mapped I/O

use std::fs::{self, File};
use std::io;
use std::path::Path;

use memmap2::Mmap;

use crate::file_size::MMAP_THRESHOLD;

/// Content of a file, either owned or memory-mapped.
pub enum FileContent {
    /// Small file read into memory.
    Owned(String),
    /// Large file memory-mapped.
    Mapped(MappedContent),
}

/// Memory-mapped file content with UTF-8 validation.
pub struct MappedContent {
    mmap: Mmap,
}

impl MappedContent {
    /// Get content as string slice.
    /// Returns None if content is not valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.mmap).ok()
    }
}

impl FileContent {
    /// Read file using appropriate strategy based on size.
    pub fn read(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        let size = meta.len();

        if size < MMAP_THRESHOLD {
            // Small file: direct read
            let content = fs::read_to_string(path)?;
            Ok(FileContent::Owned(content))
        } else {
            // Large file: memory-map
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            Ok(FileContent::Mapped(MappedContent { mmap }))
        }
    }

    /// Get content as string slice.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FileContent::Owned(s) => Some(s),
            FileContent::Mapped(m) => m.as_str(),
        }
    }
}
```

**Step 3c:** Update checks to use FileContent

Example update for `checks/cloc.rs`:

```rust
// Before:
let content = match std::fs::read_to_string(&file.path) {
    Ok(c) => c,
    Err(_) => continue,
};

// After:
use crate::file_reader::FileContent;

let content = match FileContent::read(&file.path) {
    Ok(c) => c,
    Err(_) => continue,
};
let Some(content) = content.as_str() else {
    continue; // Skip non-UTF-8 files
};
```

**Verification:**
```bash
cargo test -p quench
# Verify large file handling:
cargo run -- check tests/fixtures/bench-medium
```

---

### Phase 4: Document Pre-allocation Rationale

**Goal:** Add comments explaining capacity estimates for future maintainers.

**File:** `crates/cli/src/runner.rs`

The pre-allocation at lines 84-88 is intentionally sized for warm cache (common case):

```rust
// Pre-size for expected distribution (optimized for warm cache case)
// Cold runs will reallocate, but that's acceptable as they're infrequent
// (~4 reallocations worst case, negligible vs check work)
let file_count = files.len();
let mut cached_violations: HashMap<PathBuf, CachedViolationsArc> =
    HashMap::with_capacity(file_count);
// Expect ~10% cache miss on warm runs. Cold runs will reallocate.
let mut uncached_files: Vec<&WalkedFile> = Vec::with_capacity(file_count / 10 + 1);
```

**Verification:**
```bash
cargo test -p quench -- runner
```

---

### Phase 5: Final Verification and Benchmarks

**Goal:** Verify all changes pass tests and don't regress performance.

**Steps:**
1. Run full test suite
2. Run `make check`
3. Compare benchmark results
4. Dogfood on quench

**Verification:**
```bash
# Full test suite
cargo test --all

# CI checks
make check

# Benchmarks (compare before/after)
cargo bench -p quench -- --save-baseline after-17h

# Dogfooding
cargo run --release -- check
cargo run --release -- check tests/fixtures/bench-medium
```

**Expected Results:**
- No performance regression (within measurement noise)
- Reduced memory allocations in startup path
- Large file handling uses mmap (verify via strace/dtruss if curious)

## Key Implementation Details

### Box::leak Pattern

The `Box::leak` pattern is intentional and acceptable:
- Patterns are compiled once at startup
- They live for the program duration
- The alternative (unsafe lifetime extension) is worse
- Memory "leaked" is reclaimed on process exit

The fix eliminates the redundant clone, not the leak itself.

### Memory-Mapped I/O Safety

The `unsafe` block in `Mmap::map()` is safe because:
1. File handle is valid (just opened)
2. We don't mutate the mapped memory
3. We handle the case where file is modified during mapping (returns stale data, which is acceptable for linting)

The `memmap2` crate is the standard choice for safe mmap in Rust.

### Pre-allocation Strategy

| Scenario | Cache Miss Rate | Vec Reallocations |
|----------|----------------|-------------------|
| Warm run | ~10% | 0-1 |
| Cold run | 100% | ~4 |
| Config change | 100% | ~4 |

The ~4 reallocations on cold runs add negligible overhead compared to actually checking all files.

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test -p quench -- matcher` | Tests pass |
| 2 | `cargo test -p quench -- workspace` | Tests pass |
| 3 | `cargo test -p quench` | All tests pass |
| 4 | `cargo test -p quench -- runner` | Tests pass |
| 5 | `make check` | All quality gates pass |

## Exit Criteria

- [ ] Box::leak double clone eliminated in matcher.rs
- [ ] String allocation pattern fixed in workspace.rs
- [ ] Memory-mapped I/O implemented for files >64KB
- [ ] Pre-allocation rationale documented in runner.rs
- [ ] `make check` passes
- [ ] No performance regressions (benchmark comparison)
- [ ] Dogfooding passes: `quench check` on quench
