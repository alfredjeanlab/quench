# Checkpoint 8H: Tests Correlation Tech Debt

**Plan:** `checkpoint-8h-techdebt`
**Root Feature:** `tests-correlation`
**Depends On:** `checkpoint-8g-bugfix` (Edge cases in tests correlation check)

## Overview

Consolidate duplicated logic in the tests correlation module to improve maintainability and reduce code surface area. The correlation logic currently has several patterns repeated across multiple functions—base name extraction, test suffix/prefix matching, and candidate path generation—that can be unified into shared abstractions.

**Key Tech Debt Items:**
1. Base name extraction logic appears in 3+ locations with slightly different implementations
2. Test suffix/prefix matching (`_test`, `_tests`, `test_`) is repeated in `has_test_for()`, `is_test_only()`, and `has_correlated_test()`
3. Candidate test path generation is duplicated between Rust and JavaScript with similar structure
4. Constants like suffix patterns are embedded inline rather than defined once

## Project Structure

```
quench/
└── crates/cli/src/checks/tests/
    ├── correlation.rs            # MODIFY: Extract shared patterns
    ├── correlation_tests.rs      # MODIFY: Update tests for refactored code
    ├── mod.rs                    # REVIEW: Minor cleanup if needed
    └── patterns.rs               # NEW: Shared test pattern constants and matchers
```

## Dependencies

No new dependencies required. Existing crates:
- `globset` - Pattern matching
- `rayon` - Parallel processing

## Implementation Phases

### Phase 1: Extract Test Pattern Constants

**Goal:** Define test suffix/prefix patterns as constants to eliminate magic strings.

**Tasks:**

1. Create `patterns.rs` module with shared constants:

```rust
// crates/cli/src/checks/tests/patterns.rs

//! Shared test file naming patterns and utilities.

/// Suffixes that identify test files (Rust/Go style).
pub const TEST_SUFFIXES: &[&str] = &["_tests", "_test", "_spec"];

/// Prefixes that identify test files.
pub const TEST_PREFIXES: &[&str] = &["test_"];

/// Suffixes for JS/TS style test files (part of stem, e.g., "parser.test.ts").
pub const JS_TEST_SUFFIXES: &[&str] = &[".test", ".spec"];

/// All suffix patterns combined for extraction.
pub const ALL_TEST_SUFFIXES: &[&str] = &["_tests", "_test", ".test", ".spec", "_spec"];
```

2. Add `pub mod patterns;` to `mod.rs`.

3. Replace inline `"_test"`, `"_tests"`, `"test_"` strings in `correlation.rs` with constants.

**Verification:**
```bash
cargo check -p quench
cargo test -p quench -- correlation
```

---

### Phase 2: Unify Base Name Extraction

**Goal:** Consolidate base name extraction into a single function with clear semantics.

**Current duplication:**
- `extract_base_name()` at line 579 - strips suffixes for test files
- `correlation_base_name()` at line 448 - returns raw file stem
- Inline stem extraction in `TestIndex::has_test_for()` at line 124

**Tasks:**

1. Rename `extract_base_name()` to `strip_test_affixes()` to clarify its purpose:

```rust
/// Strip test-related suffixes and prefixes from a file stem.
///
/// Examples:
/// - "parser_tests" -> "parser"
/// - "test_parser" -> "parser"
/// - "parser.test" -> "parser"
/// - "parser" -> "parser" (unchanged)
pub fn strip_test_affixes(stem: &str) -> &str {
    for suffix in patterns::ALL_TEST_SUFFIXES {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            return stripped;
        }
    }
    for prefix in patterns::TEST_PREFIXES {
        if let Some(stripped) = stem.strip_prefix(prefix) {
            return stripped;
        }
    }
    stem
}
```

2. Create `file_base_name()` for extracting base from path:

```rust
/// Extract the normalized base name from a file path.
///
/// Returns the file stem with test affixes stripped.
pub fn file_base_name(path: &Path) -> Option<&str> {
    let stem = path.file_stem()?.to_str()?;
    Some(strip_test_affixes(stem))
}
```

3. Update callers:
   - `TestIndex::new()` - use `file_base_name()`
   - `analyze_single_source()` - use `file_base_name()`
   - `analyze_correlation()` - use `file_base_name()`

**Verification:**
```bash
cargo test -p quench -- correlation
cargo test -p quench tests::specs  # behavioral tests
```

---

### Phase 3: Consolidate Test Matching Logic

**Goal:** Unify the base name matching logic from `has_test_for()`, `is_test_only()`, and `has_correlated_test()`.

**Current duplication:**
- `TestIndex::has_test_for()` checks: direct match, `{name}_test`, `{name}_tests`, `test_{name}`
- `is_test_only()` checks: same patterns in reverse (test -> source matching)
- `has_correlated_test()` checks: same patterns again

**Tasks:**

1. Create a `matches_base_name()` helper:

```rust
/// Check if a test base name correlates with a source base name.
///
/// Matching rules:
/// 1. Direct: "parser" matches "parser"
/// 2. Source with suffix: "parser" matches "parser_test", "parser_tests"
/// 3. Source with prefix: "parser" matches "test_parser"
pub fn matches_base_name(test_base: &str, source_base: &str) -> bool {
    // Direct match
    if test_base == source_base {
        return true;
    }

    // Test has suffix matching source
    for suffix in patterns::TEST_SUFFIXES {
        if test_base == format!("{}{}", source_base, suffix) {
            return true;
        }
    }

    // Test has prefix matching source
    for prefix in patterns::TEST_PREFIXES {
        if test_base == format!("{}{}", prefix, source_base) {
            return true;
        }
    }

    false
}
```

2. Simplify `TestIndex::has_test_for()`:

```rust
pub fn has_test_for(&self, source_path: &Path) -> bool {
    let source_base = source_path.file_stem().and_then(|s| s.to_str())?;
    self.base_names.iter().any(|test_base|
        matches_base_name(test_base, source_base)
    )
}
```

3. Simplify `is_test_only()`:

```rust
fn is_test_only(test_base: &str, source_base_names: &HashSet<String>) -> bool {
    !source_base_names.iter().any(|source_base|
        matches_base_name(test_base, source_base)
    )
}
```

4. Simplify `has_correlated_test()`:

```rust
pub fn has_correlated_test(
    source_path: &Path,
    test_changes: &[PathBuf],
    test_base_names: &[String],
) -> bool {
    let source_base = source_path.file_stem().and_then(|s| s.to_str())?;

    // Strategy 1: Check expected test locations
    let expected = find_test_locations(source_path);
    if test_changes.iter().any(|t| expected.iter().any(|e| t.ends_with(e))) {
        return true;
    }

    // Strategy 2: Base name matching
    test_base_names.iter().any(|test_base|
        matches_base_name(test_base, source_base)
    )
}
```

**Verification:**
```bash
cargo test -p quench -- correlation
cargo test -p quench tests::specs::checks::tests
```

---

### Phase 4: Unify Candidate Path Generation

**Goal:** Merge `candidate_test_paths()` and `candidate_js_test_paths()` into a single language-aware function.

**Tasks:**

1. Create unified `candidate_test_paths_for()` function:

```rust
/// Generate candidate test file paths for a given source file.
pub fn candidate_test_paths_for(source_path: &Path) -> Vec<String> {
    let base = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return vec![],
    };

    let lang = detect_language(source_path);

    match lang {
        Language::Rust => candidate_rust_test_paths(base),
        Language::Go => candidate_go_test_paths(base),
        Language::JavaScript => candidate_js_test_paths(base),
        Language::Python => candidate_python_test_paths(base),
        Language::Unknown => vec![],
    }
}

fn candidate_rust_test_paths(base: &str) -> Vec<String> {
    vec![
        format!("tests/{}_tests.rs", base),
        format!("tests/{}_test.rs", base),
        format!("tests/{}.rs", base),
        format!("test/{}_tests.rs", base),
        format!("test/{}_test.rs", base),
        format!("test/{}.rs", base),
    ]
}

fn candidate_go_test_paths(base: &str) -> Vec<String> {
    vec![format!("{}_test.go", base)]
}

fn candidate_js_test_paths(base: &str) -> Vec<String> {
    let exts = ["ts", "js"];
    let mut paths = Vec::with_capacity(16);
    for ext in &exts {
        paths.push(format!("{}.test.{}", base, ext));
        paths.push(format!("{}.spec.{}", base, ext));
        paths.push(format!("__tests__/{}.test.{}", base, ext));
        paths.push(format!("tests/{}.test.{}", base, ext));
    }
    paths
}

fn candidate_python_test_paths(base: &str) -> Vec<String> {
    vec![
        format!("test_{}.py", base),
        format!("tests/test_{}.py", base),
        format!("{}_test.py", base),
    ]
}
```

2. Move `Language` enum and `detect_language()` to `patterns.rs` for sharing.

3. Update `has_placeholder_for_source()` in `mod.rs` to use the unified function.

4. Keep the original public functions as thin wrappers for backward compatibility:

```rust
/// Get candidate test paths for Rust files.
///
/// Deprecated: Use `candidate_test_paths_for()` instead.
pub fn candidate_test_paths(base_name: &str) -> Vec<String> {
    candidate_rust_test_paths(base_name)
}
```

**Verification:**
```bash
cargo test -p quench -- correlation
cargo test -p quench -- placeholder
```

---

### Phase 5: Final Cleanup and Documentation

**Goal:** Remove any remaining duplication, add documentation, run full test suite.

**Tasks:**

1. Review `correlation.rs` for any remaining inline patterns.

2. Add module documentation to `patterns.rs`:

```rust
//! Test file naming patterns and matching utilities.
//!
//! This module provides shared constants and functions for identifying
//! test files and correlating them with source files across languages.
//!
//! # Supported Languages
//!
//! - Rust: `*_test.rs`, `*_tests.rs`, `test_*.rs`
//! - Go: `*_test.go`
//! - JavaScript/TypeScript: `*.test.ts`, `*.spec.ts`, `__tests__/*.test.ts`
//! - Python: `test_*.py`, `*_test.py`
```

3. Ensure `#[inline]` hints on hot path functions if appropriate.

4. Run full test suite and verify no regressions.

**Verification:**
```bash
make check  # Full CI validation
```

## Key Implementation Details

### Backward Compatibility

The refactoring preserves all existing public function signatures. The original `candidate_test_paths()` and `candidate_js_test_paths()` remain available. Internal helpers are consolidated without changing external behavior.

### Performance Considerations

- `matches_base_name()` uses early return for direct match (most common case)
- Pattern constants use `&'static str` to avoid allocations
- `strip_test_affixes()` returns `&str` slice, no allocation
- No changes to the `TestIndex` O(1) lookup strategy

### Code Reduction Estimate

| Area | Before | After | Reduction |
|------|--------|-------|-----------|
| Suffix/prefix strings | 15+ inline | 4 constants | -11 |
| Base name extraction | 3 functions | 2 functions | -1 |
| Matching logic | 3 implementations | 1 shared | -2 |
| Path generation | 2 functions | 1 unified + helpers | cleaner API |

## Verification Plan

### Per-Phase Verification

Each phase includes specific verification commands that must pass before proceeding.

### Full Verification Checklist

After all phases complete:

```bash
# 1. Code compiles without warnings
cargo check -p quench

# 2. Clippy passes
cargo clippy -p quench -- -D warnings

# 3. Unit tests pass
cargo test -p quench -- correlation
cargo test -p quench -- placeholder

# 4. Behavioral specs pass
cargo test -p quench tests::specs::checks::tests

# 5. Full CI check
make check
```

### Success Criteria

- [ ] `patterns.rs` module created with shared constants
- [ ] `strip_test_affixes()` replaces duplicated suffix stripping
- [ ] `matches_base_name()` unifies matching logic
- [ ] `candidate_test_paths_for()` provides language-aware path generation
- [ ] All existing tests pass (no behavioral changes)
- [ ] `make check` passes

## Deliverables

1. **New Module:** `crates/cli/src/checks/tests/patterns.rs` with shared patterns
2. **Refactored:** `correlation.rs` with reduced duplication
3. **Updated Tests:** `correlation_tests.rs` adjusted for new function names if needed
4. **No API Changes:** All public functions maintain backward compatibility
