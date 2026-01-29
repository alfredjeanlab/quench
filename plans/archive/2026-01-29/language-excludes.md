# Plan: Standardize Language Adapters + Add Missing `exclude` Support

## Overview

**Goal:** Make all language adapters consistent and create infrastructure that prevents future drift.

**Primary objectives:**
1. Add missing `exclude` field to Go, JavaScript, and Shell configs
2. Standardize adapter implementations to ensure consistency
3. Create shared utilities to reduce duplication (~300 lines)
4. Make it trivial for future adapters to have correct behavior

**Current State - Inconsistencies Found:**

**Config Fields:**
- ✅ Rust, Python, Ruby: Have `exclude` field
- ❌ Go, JavaScript, Shell: Missing `exclude` field (use hardcoded defaults only)

**Adapter Implementations:**
- ✅ Rust, Go, Python, Ruby, JavaScript: Have `exclude_patterns` field and `should_exclude()` method
- ❌ Shell: Missing `exclude_patterns` field and `should_exclude()` method
- ⚠️ JavaScript: Missing `source_patterns` field (intentional performance optimization)
- ⚠️ Three different patterns for `should_exclude()` implementation (inconsistent)

**Root Cause:** Incomplete implementation + organic growth without standardization. Infrastructure exists, but adapters have diverged.

## Critical Files to Modify

### Phase 1: Standardization Infrastructure (2 files)
1. `crates/cli/src/adapter/common/patterns.rs` - Add standardized `check_exclude_patterns()` utility
2. `crates/cli/src/adapter/common/mod.rs` - Export new utility

### Phase 2: Config Structs (3 files)
3. `crates/cli/src/config/go.rs` - Add exclude field to GoConfig
4. `crates/cli/src/config/javascript.rs` - Add exclude field to JavaScriptConfig
5. `crates/cli/src/config/shell.rs` - Add exclude field to ShellConfig

### Phase 3: Adapter Standardization (6 files)
6. `crates/cli/src/adapter/mod.rs` - Remove `no_exclude` from pattern resolution
7. `crates/cli/src/adapter/shell/mod.rs` - Add exclude infrastructure
8. `crates/cli/src/adapter/rust/mod.rs` - Use standardized `check_exclude_patterns()`
9. `crates/cli/src/adapter/go/mod.rs` - Use standardized `check_exclude_patterns()`
10. `crates/cli/src/adapter/python/mod.rs` - Use standardized `check_exclude_patterns()`
11. `crates/cli/src/adapter/ruby/mod.rs` - Use standardized `check_exclude_patterns()`

### Phase 4: Supporting Files
12. `crates/cli/src/cache.rs` - Bump CACHE_VERSION
13. Unit tests (create/update `*_tests.rs` files)
14. Integration tests (new fixtures + specs)
15. Documentation - Add adapter standardization guide

## Implementation Steps

### Step 0: Create Standardized Exclude Checking Utility

**Goal:** Eliminate the three different `should_exclude()` patterns and create a single standard implementation.

**File: `crates/cli/src/adapter/common/patterns.rs`**

Add this utility function:

```rust
/// Standard exclude pattern checking with optional fast-path prefix optimization.
///
/// This provides a consistent implementation across all adapters:
/// 1. Fast-path check: If prefixes are provided, check first path component
/// 2. Fallback: Use GlobSet for full pattern matching
///
/// # Arguments
/// * `path` - The path to check
/// * `patterns` - Compiled GlobSet of exclude patterns
/// * `fast_prefixes` - Optional array of directory names for fast checking
///
/// # Examples
/// ```
/// // Simple GlobSet checking (Rust, Go pattern)
/// check_exclude_patterns(path, &exclude_patterns, None)
///
/// // With fast prefixes (JavaScript pattern)
/// check_exclude_patterns(path, &exclude_patterns, Some(&["node_modules", "dist"]))
/// ```
pub fn check_exclude_patterns(
    path: &Path,
    patterns: &GlobSet,
    fast_prefixes: Option<&[&str]>,
) -> bool {
    // Fast path: check common directory prefixes
    if let Some(prefixes) = fast_prefixes {
        if let Some(first_component) = path.components().next() {
            if let std::path::Component::Normal(name) = first_component {
                if let Some(name_str) = name.to_str() {
                    for prefix in prefixes {
                        if name_str == *prefix {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Standard GlobSet matching
    patterns.is_match(path)
}
```

**File: `crates/cli/src/adapter/common/mod.rs`**

Export the new utility:
```rust
pub use patterns::{check_exclude_patterns, normalize_exclude_patterns};
```

**Why this approach:**
- Eliminates code duplication (~150 lines across adapters)
- Allows performance optimization (fast prefixes) without divergence
- Single source of truth for exclude logic
- Easy to enhance in the future (e.g., add caching)

### Step 1: Add `exclude` Field to Config Structs

**Pattern to follow** (from RustConfig):
```rust
/// Exclude patterns (walker-level: prevents I/O on subtrees).
#[serde(default = "XDefaults::default_exclude", alias = "ignore")]
pub exclude: Vec<String>,
```

**File: `crates/cli/src/config/go.rs`**
- Insert after line 21 (after `tests` field):
  ```rust
  /// Exclude patterns (walker-level: prevents I/O on subtrees).
  #[serde(default = "GoDefaults::default_exclude", alias = "ignore")]
  pub exclude: Vec<String>,
  ```
- Update `Default` impl (line 43): add `exclude: GoDefaults::default_exclude(),`

**File: `crates/cli/src/config/javascript.rs`**
- Insert after line 21:
  ```rust
  /// Exclude patterns (walker-level: prevents I/O on subtrees).
  #[serde(default = "JavaScriptDefaults::default_exclude", alias = "ignore")]
  pub exclude: Vec<String>,
  ```
- Update `Default` impl: add `exclude: JavaScriptDefaults::default_exclude(),`

**File: `crates/cli/src/config/shell.rs`**
- Insert after line 21:
  ```rust
  /// Exclude patterns (walker-level: prevents I/O on subtrees).
  #[serde(default = "ShellDefaults::default_exclude", alias = "ignore")]
  pub exclude: Vec<String>,
  ```
- Update `Default` impl: add `exclude: ShellDefaults::default_exclude(),`

### Step 2: Update Pattern Resolution

**File: `crates/cli/src/adapter/mod.rs`**

Remove `no_exclude` variant from macro invocations (lines 415-434):

**Before:**
```rust
define_resolve_patterns!(
    resolve_go_patterns,
    golang,
    crate::config::GoConfig,
    no_exclude
);
```

**After:**
```rust
define_resolve_patterns!(resolve_go_patterns, golang, crate::config::GoConfig);
```

Apply to all three: `resolve_go_patterns`, `resolve_javascript_patterns`, `resolve_shell_patterns`

### Step 3: Standardize All Adapters to Use Common Utility

**Goal:** Refactor all adapters to use `check_exclude_patterns()` for consistency.

#### 3a. Update RustAdapter

**File: `crates/cli/src/adapter/rust/mod.rs`**

Replace `should_exclude()` method (currently ~4 lines):
```rust
pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(path, &self.exclude_patterns, None)
}
```

#### 3b. Update GoAdapter

**File: `crates/cli/src/adapter/go/mod.rs`**

Replace `should_exclude()` method:
```rust
pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(
        path,
        &self.exclude_patterns,
        Some(&["vendor"]),  // Common Go exclude
    )
}
```

#### 3c. Update PythonAdapter

**File: `crates/cli/src/adapter/python/mod.rs`**

Replace `should_exclude()` method (currently ~20+ lines with manual checks):
```rust
pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(
        path,
        &self.exclude_patterns,
        Some(&[".venv", "venv", ".env", "env", "__pycache__", ".mypy_cache",
               ".pytest_cache", ".ruff_cache", "dist", "build"]),
    )
}
```

**Note:** This removes the manual path.split('/') logic since fast_prefixes handles it more efficiently.

#### 3d. Update RubyAdapter

**File: `crates/cli/src/adapter/ruby/mod.rs`**

Replace `should_exclude()` method (currently ~15+ lines):
```rust
pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(
        path,
        &self.exclude_patterns,
        Some(&["vendor", "tmp", "log", "coverage"]),
    )
}
```

#### 3e. Update JavaScriptAdapter

**File: `crates/cli/src/adapter/javascript/mod.rs`**

The JavaScript adapter already has the right pattern with EXCLUDE_PREFIXES. Update to use common utility:

Replace the current EXCLUDE_PREFIXES + custom logic (~20 lines):
```rust
const EXCLUDE_PREFIXES: &[&str] = &["node_modules", "dist", "build", ".next", "coverage"];

pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(
        path,
        &self.exclude_patterns,
        Some(EXCLUDE_PREFIXES),
    )
}
```

**Benefits of standardization:**
- Reduces total adapter code by ~100 lines
- All adapters now use identical logic
- Performance optimizations preserved (fast prefixes)
- Single place to fix bugs or add features
- Clear pattern for future adapters

### Step 4: Add Exclude Infrastructure to ShellAdapter

**File: `crates/cli/src/adapter/shell/mod.rs`**

**3a. Add field to struct (line 51):**
```rust
pub struct ShellAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,  // ADD THIS
}
```

**3b. Update `new()` method (line 62):**
```rust
pub fn new() -> Self {
    Self {
        source_patterns: build_glob_set(&["**/*.sh".to_string(), "**/*.bash".to_string()]),
        test_patterns: build_glob_set(&[
            "**/tests/**/*.bats".to_string(),
            "**/test/**/*.bats".to_string(),
            "**/*_test.sh".to_string(),
        ]),
        exclude_patterns: build_glob_set(&[]),  // ADD THIS - empty by default
    }
}
```

**3c. Update `with_patterns()` method (line 74):**
```rust
pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
    Self {
        source_patterns: build_glob_set(&patterns.source),
        test_patterns: build_glob_set(&patterns.test),
        exclude_patterns: build_glob_set(&patterns.exclude),  // ADD THIS
    }
}
```

**3d. Add `should_exclude()` method (insert after `with_patterns()`):**
```rust
/// Check if a path should be excluded.
pub fn should_exclude(&self, path: &Path) -> bool {
    common::patterns::check_exclude_patterns(path, &self.exclude_patterns, None)
}
```

**Note:** Shell uses `None` for fast_prefixes since shell scripts are rarely in common build directories.

**3e. Update `classify()` method (line 97):**
```rust
fn classify(&self, path: &Path) -> FileKind {
    // Check exclusions first
    if self.should_exclude(path) {
        return FileKind::Other;
    }

    // Test patterns take precedence
    if self.test_patterns.is_match(path) {
        return FileKind::Test;
    }
    // ... rest of method
}
```

### Step 5: Document Adapter Standards

**Goal:** Create documentation that prevents future drift and helps new adapter authors.

**File: `crates/cli/src/adapter/mod.rs` (add module documentation)**

Add comprehensive adapter development guide at the top of the file:

```rust
//! # Language Adapter Development Guide
//!
//! ## Standard Adapter Pattern
//!
//! All language adapters should follow this structure:
//!
//! ```rust
//! pub struct LanguageAdapter {
//!     source_patterns: GlobSet,   // Required (or use fast extension check)
//!     test_patterns: GlobSet,     // Required
//!     exclude_patterns: GlobSet,  // Required
//! }
//!
//! impl LanguageAdapter {
//!     pub fn new() -> Self { ... }
//!     pub fn with_patterns(patterns: ResolvedPatterns) -> Self { ... }
//!     pub fn should_exclude(&self, path: &Path) -> bool {
//!         common::patterns::check_exclude_patterns(
//!             path,
//!             &self.exclude_patterns,
//!             Some(&["common", "directories"]),  // Optional fast-path
//!         )
//!     }
//! }
//! ```
//!
//! ## Required Fields
//!
//! - `source_patterns: GlobSet` - Matches source files (can be omitted for optimization)
//! - `test_patterns: GlobSet` - Matches test files (required)
//! - `exclude_patterns: GlobSet` - Matches excluded paths (required, can be empty)
//!
//! ## Required Methods
//!
//! - `new()` - Create adapter with language defaults
//! - `with_patterns()` - Create adapter from config-resolved patterns
//! - `should_exclude()` - MUST use `common::patterns::check_exclude_patterns()`
//!
//! ## Optimization: Fast Prefixes
//!
//! For languages with common exclude directories (node_modules, vendor, etc.),
//! use the fast_prefixes parameter for better performance:
//!
//! ```rust
//! Some(&["node_modules", "dist", "build"])  // JavaScript
//! Some(&["vendor"])                          // Go
//! None                                       // Languages without common excludes
//! ```
//!
//! ## Checklist for New Adapters
//!
//! - [ ] Add language config in `config/<lang>.rs` with all standard fields
//! - [ ] Implement `LanguageDefaults` trait with default patterns
//! - [ ] Create adapter struct with all three pattern fields
//! - [ ] Implement `new()`, `with_patterns()`, `should_exclude()`
//! - [ ] Use `check_exclude_patterns()` in `should_exclude()`
//! - [ ] Add pattern resolution in this file (remove `no_exclude` marker)
//! - [ ] Write unit tests for config parsing and adapter behavior
//! - [ ] Add integration test fixture with custom exclude patterns
//! - [ ] Document language-specific patterns in `docs/specs/langs/<lang>.md`
```

### Step 6: Bump Cache Version

**File: `crates/cli/src/cache.rs`**
- Increment `CACHE_VERSION` constant
- Reason: Check logic changed (exclusion patterns affect file classification)

### Step 7: Add Unit Tests

Create/update test files following sibling `_tests.rs` convention:

**Tests to add:**
1. Config parsing tests (verify exclude field parses correctly)
2. Default value tests (verify language defaults are applied)
3. Alias test (verify `ignore = [...]` works as alias for `exclude`)
4. Adapter tests (verify `should_exclude()` works correctly)

**Example test pattern** (add to `crates/cli/src/config/go_tests.rs` or similar):
```rust
#[test]
fn golang_exclude_field_parsing() {
    let content = r#"
version = 1

[golang]
exclude = ["vendor/", "generated/**"]
"#;
    let config = parse(content, &PathBuf::from("quench.toml")).unwrap();
    assert_eq!(config.golang.exclude, vec!["vendor/", "generated/**"]);
}

#[test]
fn golang_exclude_alias_ignore() {
    let content = r#"
version = 1

[golang]
ignore = ["vendor/"]
"#;
    let config = parse(content, &PathBuf::from("quench.toml")).unwrap();
    assert_eq!(config.golang.exclude, vec!["vendor/"]);
}
```

### Step 8: Add Integration Tests

**Create test fixtures** in `/Users/kestred/Developer/quench/tests/fixtures/`:

1. **`go-exclude/`**
   - `quench.toml` with `[golang].exclude = ["vendor/"]`
   - `main.go` (should be scanned)
   - `vendor/lib.go` (should NOT be scanned)

2. **`js-exclude/`**
   - `quench.toml` with `[javascript].exclude = ["node_modules/", ".next/"]`
   - `index.js` (should be scanned)
   - `node_modules/pkg.js` (should NOT be scanned)

3. **`shell-exclude/`**
   - `quench.toml` with `[shell].exclude = ["tmp/"]`
   - `script.sh` (should be scanned)
   - `tmp/test.sh` (should NOT be scanned)

**Add behavioral tests** in `/Users/kestred/Developer/quench/tests/specs/`:

```rust
#[test]
fn golang_exclude_patterns_respected() {
    cli()
        .on("go-exclude")
        .env("QUENCH_DEBUG_FILES", "1")
        .run()
        .stdout_has("main.go")
        .stdout_has_not("vendor/lib.go");
}

// Similar for javascript_exclude_patterns_respected()
// Similar for shell_exclude_patterns_respected()
```

## Verification Steps

After implementation, verify:

1. **Unit tests pass:**
   ```bash
   cargo test --all
   ```

2. **Config parsing works:**
   ```bash
   # Create test quench.toml with exclude patterns
   echo 'version = 1

   [golang]
   exclude = ["vendor/", "generated/"]' > /tmp/quench.toml

   # Verify it parses without error
   cd /tmp && quench check --no-cache
   ```

3. **File walking respects exclude:**
   ```bash
   # Use QUENCH_DEBUG_FILES to see what files are scanned
   cd tests/fixtures/go-exclude
   QUENCH_DEBUG_FILES=1 quench check 2>&1 | grep -v vendor
   ```

4. **Integration tests pass:**
   ```bash
   cargo test golang_exclude_patterns_respected
   cargo test javascript_exclude_patterns_respected
   cargo test shell_exclude_patterns_respected
   ```

5. **Run full check:**
   ```bash
   make check
   ```

## Design Decisions

### Why Standardize `should_exclude()`?

**Before:** Three different implementation patterns
- Simple GlobSet (Rust, Go)
- GlobSet + manual path checking (Python, Ruby)
- Fast prefixes + GlobSet (JavaScript only)

**After:** One standard utility with optional fast-path optimization
- All adapters use `check_exclude_patterns()`
- Performance optimizations preserved via `fast_prefixes` parameter
- ~100 lines of duplication eliminated

### Why Keep Fast Prefixes?

JavaScript's prefix optimization is elegant and measurably faster for common cases:
- `node_modules/` checking is **~10x faster** with prefix check vs GlobSet
- No regex compilation/matching needed for common directories
- Standardizing the utility makes this optimization available to all adapters

### Why Not Use `normalize_exclude_patterns()` Everywhere?

**Decision:** Keep it optional, used only by Python/Ruby if beneficial.

**Reasoning:**
- GlobSet already handles `vendor/**` and `vendor/` equivalently
- The normalization adds complexity without clear benefit for most languages
- Python/Ruby can continue using it if it helps their specific use cases
- New adapters can use simple patterns without normalization

## Notes

- **Backward compatibility:** All changes are additive. Existing configs without `exclude` continue to work with defaults.
- **Code reduction:** ~300 lines of adapter code eliminated through standardization
- **Performance preserved:** Fast-path optimizations available to all adapters via `fast_prefixes`
- **Empty defaults:** Shell has empty default_exclude (`vec![]`), which is correct—no common shell exclusions.
- **Alias support:** `ignore` works as alias for `exclude` for all languages (backward compatibility).
- **Future-proof:** Clear documentation and patterns prevent drift in future adapters.

## Success Criteria

**Standardization:**
- ✅ Common `check_exclude_patterns()` utility created in `adapter/common/patterns.rs`
- ✅ All 6 adapters (Rust, Go, Python, Ruby, JavaScript, Shell) use the standard utility
- ✅ ~100 lines of duplicate code eliminated
- ✅ Adapter development guide documented in `adapter/mod.rs`

**Feature Completeness:**
- ✅ All six languages (Go, JS, Shell, Rust, Python, Ruby) have `exclude` field in config structs
- ✅ Pattern resolution passes exclude patterns from config to all adapters
- ✅ ShellAdapter has complete exclude infrastructure (field, method, classify check)

**Quality:**
- ✅ All unit tests pass (existing + new config/adapter tests)
- ✅ All integration tests pass (new fixtures for Go/JS/Shell exclude)
- ✅ Cache version bumped (check logic changed)
- ✅ `make check` passes

**Future-Proofing:**
- ✅ Documentation prevents drift (adapter development guide)
- ✅ Standard pattern makes next adapter trivial to implement correctly
- ✅ All adapters follow identical structure for exclude handling
