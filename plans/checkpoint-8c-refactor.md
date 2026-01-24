# Checkpoint 8C: Refactor - Tests Correlation

**Root Feature:** `quench-91c5`

## Overview

Refactor the tests correlation check implementation to eliminate code duplication, improve maintainability, and establish clear abstractions. The feature works correctly (validated in checkpoint 8B); this checkpoint focuses on code quality improvements without changing behavior.

Key refactoring targets:
1. Test path generation duplicated in 3 locations
2. Placeholder checking logic duplicated between branch and commit scopes
3. Inline test detection with two separate implementations
4. Violation building logic with similar patterns in two scopes
5. Path normalization patterns repeated throughout

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── mod.rs              # Orchestration (REFACTOR: extract helpers)
│   ├── mod_tests.rs        # Unit tests
│   ├── correlation.rs      # Core logic (REFACTOR: add helpers)
│   ├── correlation_tests.rs
│   ├── diff.rs             # Git integration (no changes)
│   └── diff_tests.rs
└── tests/specs/checks/tests/
    ├── mod.rs
    ├── correlation.rs      # Behavioral tests
    └── output.rs           # Output snapshot tests
```

## Dependencies

No new dependencies required. This is a pure refactoring checkpoint that reorganizes existing code.

## Implementation Phases

### Phase 1: Extract Test Path Helper

**Goal:** Consolidate the hardcoded test path patterns into a single helper function.

**Problem:** Test paths are defined identically in 3 locations:
- `mod.rs:119-125` (branch scope placeholder check)
- `mod.rs:232-238` (commit scope placeholder check)
- `correlation.rs:199-219` (`find_test_locations()`)

**Solution:** Add a new helper function in `correlation.rs`:

```rust
/// Get candidate test file paths for a source file.
///
/// Returns patterns like: tests/{base}_tests.rs, tests/{base}_test.rs, etc.
pub fn candidate_test_paths(base_name: &str) -> Vec<String> {
    vec![
        format!("tests/{}_tests.rs", base_name),
        format!("tests/{}_test.rs", base_name),
        format!("tests/{}.rs", base_name),
        format!("test/{}_tests.rs", base_name),
        format!("test/{}_test.rs", base_name),
        format!("test/{}.rs", base_name),
    ]
}
```

**Files to modify:**
- `correlation.rs` - Add `candidate_test_paths()` function
- `mod.rs` - Replace hardcoded arrays with calls to `candidate_test_paths()`

**Verification:**
```bash
cargo test --all
cargo test --test specs checks_tests
```

### Phase 2: Extract Placeholder Check Helper

**Goal:** Eliminate duplication in placeholder test checking between branch and commit scopes.

**Problem:** Nearly identical placeholder checking logic in two locations:
- `mod.rs:116-135` (branch scope)
- `mod.rs:229-249` (commit scope)

**Solution:** Add a helper function:

```rust
/// Check if any placeholder test satisfies the test requirement for a source file.
fn has_placeholder_for_source(
    source_path: &Path,
    root: &Path,
) -> bool {
    let base_name = match source_path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    candidate_test_paths(base_name).iter().any(|test_path| {
        let test_file = Path::new(test_path);
        root.join(test_file).exists()
            && has_placeholder_test(test_file, base_name, root).unwrap_or(false)
    })
}
```

**Files to modify:**
- `mod.rs` - Add helper function and use it in both `run_branch_scope()` and `run_commit_scope()`

**Verification:**
```bash
cargo test --all
cargo test --test specs checks_tests
```

### Phase 3: Unify Inline Test Detection

**Goal:** Consolidate the two inline test detection implementations into a single abstraction.

**Problem:** Two similar but separate functions:
- `has_inline_test_changes()` in `correlation.rs:361` - uses `base..HEAD` or `--cached`
- `has_inline_test_changes_in_commit()` in `mod.rs:314` - uses `hash^..hash`

**Solution:** Create a unified function with an enum for the diff range:

```rust
/// Specifies the git diff range for inline test detection.
pub enum DiffRange<'a> {
    /// Staged changes (--cached)
    Staged,
    /// Branch changes (base..HEAD)
    Branch(&'a str),
    /// Single commit (hash^..hash)
    Commit(&'a str),
}

/// Check if a Rust file has inline test changes in the specified range.
pub fn has_inline_test_changes(
    file_path: &Path,
    root: &Path,
    range: DiffRange<'_>,
) -> bool {
    let diff_content = match get_file_diff_for_range(file_path, root, range) {
        Ok(content) => content,
        Err(_) => return false,
    };
    changes_in_cfg_test(&diff_content)
}
```

**Files to modify:**
- `correlation.rs` - Add `DiffRange` enum, update `has_inline_test_changes()`, add `get_file_diff_for_range()`
- `mod.rs` - Remove `has_inline_test_changes_in_commit()`, use unified function

**Verification:**
```bash
cargo test --all
cargo test --test specs checks_tests::correlation
```

### Phase 4: Extract Path Normalization

**Goal:** Reduce repeated path normalization patterns.

**Problem:** Same pattern appears 3+ times:
```rust
c.path.strip_prefix(ctx.root).unwrap_or(&c.path)
```

**Solution:** Add a helper in `mod.rs`:

```rust
/// Normalize a path relative to root, returning the original if not under root.
fn relative_to_root<'a>(path: &'a Path, root: &Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}
```

And use it consistently:
```rust
// Before
let change = changes.iter().find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path).eq(path));

// After
let change = changes.iter().find(|c| relative_to_root(&c.path, ctx.root).eq(path));
```

**Files to modify:**
- `mod.rs` - Add helper, use in `run_branch_scope()` and `run_commit_scope()`

**Verification:**
```bash
cargo test --all
```

### Phase 5: Clean Up Magic Strings and Constants

**Goal:** Replace magic strings with named constants.

**Changes:**

1. **Rust extension check** - Add constant:
```rust
const RUST_EXT: &str = "rs";

// Before
if path.extension().is_some_and(|e| e == "rs")

// After
if path.extension().is_some_and(|e| e == RUST_EXT)
```

2. **Hash truncation length** - Add constant and helper:
```rust
const SHORT_HASH_LEN: usize = 7;

fn short_hash(hash: &str) -> &str {
    if hash.len() >= SHORT_HASH_LEN {
        &hash[..SHORT_HASH_LEN]
    } else {
        hash
    }
}
```

**Files to modify:**
- `mod.rs` - Add constants and helpers

**Verification:**
```bash
cargo test --all
```

### Phase 6: Final Verification

**Goal:** Ensure all refactoring maintains backward compatibility.

**Steps:**
1. Run full test suite
2. Run `make check` for CI validation
3. Verify no behavior changes in output

**Verification:**
```bash
# All unit tests
cargo test --all

# All behavioral tests
cargo test --test specs

# Full CI validation
make check

# Spot check output format unchanged
cd /tmp && rm -rf test-repo && mkdir test-repo && cd test-repo
git init && git config user.email "t@t" && git config user.name "T"
echo '[check.tests.commit]' > quench.toml
echo 'check = "error"' >> quench.toml
git add . && git commit -m "init"
mkdir src && echo "pub fn f() {}" > src/lib.rs
git add src/lib.rs
quench check --staged  # Should fail with same output format as before
```

## Key Implementation Details

### Helper Function Placement

- **`correlation.rs`**: Functions related to correlation detection logic
  - `candidate_test_paths()` - test path patterns
  - `DiffRange` + updated `has_inline_test_changes()` - inline test detection

- **`mod.rs`**: Functions related to check orchestration
  - `has_placeholder_for_source()` - placeholder checking
  - `relative_to_root()` - path normalization
  - `short_hash()` - hash formatting

### Preserving Test Coverage

All existing tests must continue to pass without modification. The refactoring should be purely internal reorganization:

| Test File | Tests | Coverage |
|-----------|-------|----------|
| `mod_tests.rs` | 6 | Module configuration |
| `correlation_tests.rs` | 19 | Correlation logic |
| `diff_tests.rs` | 7 | Git parsing |
| `tests/specs/checks/tests/correlation.rs` | 26 | Behavioral |
| `tests/specs/checks/tests/output.rs` | 4 | Output format |

### Import Updates

After adding public helpers to `correlation.rs`, update the imports in `mod.rs`:

```rust
use self::correlation::{
    CorrelationConfig, DiffRange, analyze_commit, analyze_correlation,
    candidate_test_paths, has_inline_test_changes, has_placeholder_test,
};
```

### Avoiding Over-Engineering

Keep refactoring minimal and focused:
- No new files (keep existing module structure)
- No major abstractions (just helper functions)
- No changes to public API
- No changes to output format

## Verification Plan

### Unit Tests
```bash
cargo test --lib
# Expected: All existing tests pass
```

### Behavioral Tests
```bash
cargo test --test specs checks_tests
# Expected: 30+ tests pass (correlation + output specs)
```

### CI Validation
```bash
make check
# Runs: fmt, clippy, test, build, audit, deny
# Expected: All pass with no new warnings
```

### Manual Spot Check
```bash
# Create test scenario and verify output unchanged
cd $(mktemp -d)
git init && git config user.email "t@t" && git config user.name "T"
echo '[check.tests.commit]
check = "error"' > quench.toml
git add . && git commit -m "init"

mkdir src && echo "pub fn f() {}" > src/feature.rs
git add src/feature.rs
quench check --staged 2>&1 | head -5
# Expected output:
# tests: FAIL
#   src/feature.rs: missing_tests (added, +1 lines)
#     Add tests in tests/feature_tests.rs or update inline #[cfg(test)] block
```

## Exit Criteria

- [ ] `candidate_test_paths()` helper extracted and used in 3 locations
- [ ] `has_placeholder_for_source()` helper extracted, eliminating duplication
- [ ] Inline test detection unified with `DiffRange` enum
- [ ] Path normalization helper used consistently
- [ ] Magic strings replaced with constants
- [ ] All tests pass: `make check`
- [ ] No changes to output format (behavioral compatibility)
