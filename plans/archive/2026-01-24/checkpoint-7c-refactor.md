# Checkpoint 7C: Refactor - Docs Check

**Root Feature:** `quench-c2a6`

## Overview

This checkpoint refactors the docs check implementation to improve maintainability, eliminate code duplication, and align with established patterns used elsewhere in the codebase. The docs check feature is complete and well-tested (54 passing behavioral specs); this refactoring focuses on code quality without changing functionality.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   └── glob.rs               # Existing shared glob utilities
│   └── checks/docs/
│       ├── mod.rs                # Coordinator (extract helpers)
│       ├── toc.rs                # Split into parse + validate modules
│       ├── toc/
│       │   ├── mod.rs            # NEW: Re-exports, validate_toc()
│       │   ├── parse.rs          # NEW: FencedBlock, TreeEntry parsing
│       │   └── detect.rs         # NEW: Tree detection heuristics
│       ├── links.rs              # Use shared glob, extract helpers
│       ├── specs.rs              # Remove KEEP UNTIL, use shared helpers
│       ├── commit.rs             # No changes needed
│       └── content.rs            # No changes needed
└── tests/specs/checks/docs/      # Verify all 54 specs still pass
```

## Dependencies

No new external dependencies. This checkpoint reuses existing infrastructure:

- `crate::adapter::glob::build_glob_set` - Already exists, will be reused
- Existing test infrastructure in `tests/specs/`

## Implementation Phases

### Phase 1: Remove Dead Code Markers

**Goal:** Clean up `KEEP UNTIL` comments that are no longer needed.

**File:** `crates/cli/src/checks/docs/specs.rs`

The `count_spec_files` function at line 64-68 is marked:
```rust
// KEEP UNTIL: Phase 616+ uses for metrics reporting
#[allow(dead_code)]
fn count_spec_files(root: &Path, specs_path: &str, extension: &str) -> usize {
```

However, this function IS being used by `collect_metrics()` on line 449. The marker is outdated.

**Action:** Remove the `// KEEP UNTIL` comment and the `#[allow(dead_code)]` attribute.

**Verification:**
```bash
cargo build --all
cargo clippy --all-targets --all-features -- -D warnings
grep -r "KEEP UNTIL" crates/cli/src/checks/docs/
```

---

### Phase 2: Use Shared Glob Utilities

**Goal:** Eliminate duplicate `build_glob_set` implementations.

**Problem:** Both `toc.rs` (line 394) and `links.rs` (line 114) define their own `build_glob_set` functions, but there's already a canonical version in `crate::adapter::glob::build_glob_set`.

**Files to modify:**
- `crates/cli/src/checks/docs/toc.rs`
- `crates/cli/src/checks/docs/links.rs`

**Changes:**

1. In `toc.rs`:
   - Remove the local `build_glob_set` function (lines 394-402)
   - Add import: `use crate::adapter::build_glob_set;`

2. In `links.rs`:
   - Remove the local `build_glob_set` function (lines 114-122)
   - Add import: `use crate::adapter::build_glob_set;`

**Verification:**
```bash
cargo build --all
cargo test --test specs checks_docs
```

---

### Phase 3: Extract Check Level Helper

**Goal:** Consolidate repeated check level logic into a shared helper.

**Problem:** The pattern for checking if a subcheck is disabled appears 4 times:
- `mod.rs:35-37` - Top-level docs check
- `toc.rs:610-617` - TOC check
- `links.rs:129-136` - Links check
- `specs.rs:291-298` - Specs check

Each follows the same pattern:
```rust
let check_level = config.check
    .as_deref()
    .or(ctx.config.check.docs.check.as_deref())
    .unwrap_or("error");
if check_level == "off" {
    return;
}
```

**Solution:** Add a helper function in `mod.rs`:

```rust
/// Check if a docs subcheck is disabled.
///
/// Returns true if the check should run, false if disabled.
fn is_check_enabled(subcheck: Option<&str>, parent: Option<&str>) -> bool {
    let level = subcheck.or(parent).unwrap_or("error");
    level != "off"
}
```

**Files to modify:**
- `crates/cli/src/checks/docs/mod.rs` - Add helper, update usage
- `crates/cli/src/checks/docs/toc.rs` - Use `super::is_check_enabled`
- `crates/cli/src/checks/docs/links.rs` - Use `super::is_check_enabled`
- `crates/cli/src/checks/docs/specs.rs` - Use `super::is_check_enabled`

**Verification:**
```bash
cargo test --test specs checks_docs
```

---

### Phase 4: Extract File Iteration Pattern

**Goal:** Consolidate duplicate file iteration logic.

**Problem:** Both `validate_toc` in `toc.rs` (lines 606-648) and `validate_links` in `links.rs` (lines 125-166) share nearly identical patterns:

1. Check if disabled
2. Build include/exclude glob sets
3. Iterate `ctx.files`
4. Filter by include patterns
5. Filter by exclude patterns
6. Read file content
7. Call per-file validator

**Solution:** Add a helper that handles the common iteration pattern:

```rust
/// Configuration for file-based validation.
pub struct FileValidatorConfig<'a> {
    pub include: &'a [String],
    pub exclude: &'a [String],
}

/// Process markdown files matching include/exclude patterns.
pub fn process_markdown_files<F>(
    ctx: &CheckContext,
    config: &FileValidatorConfig,
    mut validator: F,
) where
    F: FnMut(&Path, &str, &mut Vec<Violation>),
{
    let include_set = build_glob_set(config.include);
    let exclude_set = build_glob_set(config.exclude);

    for walked in ctx.files {
        let relative_path = walked.path.strip_prefix(ctx.root).unwrap_or(&walked.path);
        let path_str = relative_path.to_string_lossy();

        if !include_set.is_match(&*path_str) {
            continue;
        }
        if exclude_set.is_match(&*path_str) {
            continue;
        }

        let content = match std::fs::read_to_string(&walked.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Caller's validation function receives violations by reference
        validator(relative_path, &content, violations);
    }
}
```

**Files to modify:**
- `crates/cli/src/checks/docs/mod.rs` - Add helper
- `crates/cli/src/checks/docs/toc.rs` - Use helper
- `crates/cli/src/checks/docs/links.rs` - Use helper

**Verification:**
```bash
cargo test --test specs checks_docs
```

---

### Phase 5: Split toc.rs into Submodules

**Goal:** Improve maintainability by splitting the 744-line `toc.rs` into focused submodules.

**Current structure of toc.rs:**
- Lines 1-82: `FencedBlock`, `TreeEntry`, `extract_fenced_blocks`
- Lines 84-145: `parse_tree_block`, `parse_tree_line`
- Lines 147-282: `extract_indent_and_name`, `strip_comment`, `normalize_dot_prefix`
- Lines 284-371: Resolution strategies and `try_resolve`
- Lines 373-402: `try_resolve_block`, `build_glob_set` (remove in Phase 2)
- Lines 404-603: Tree detection heuristics (`NON_TREE_LANGUAGES`, `looks_like_tree`, `is_tree_line`, `looks_like_error_output`)
- Lines 605-738: `validate_toc`, `validate_file_toc`
- Lines 740-743: Test module reference

**New structure:**

```
crates/cli/src/checks/docs/toc/
├── mod.rs          # Re-exports, validate_toc, validate_file_toc (~150 lines)
├── parse.rs        # FencedBlock, TreeEntry, parsing functions (~200 lines)
├── resolve.rs      # ResolutionStrategy, try_resolve functions (~150 lines)
└── detect.rs       # looks_like_tree, NON_TREE_LANGUAGES, heuristics (~200 lines)
```

**Visibility:**
- `mod.rs`: pub(super) for `extract_fenced_blocks`, `looks_like_tree`, `parse_tree_block`
- `parse.rs`: pub(super) for types and parsing
- `resolve.rs`: pub(crate) for resolution (used by specs.rs too)
- `detect.rs`: pub(super) for tree detection

**Files to modify:**
- `crates/cli/src/checks/docs/toc.rs` → Move to `toc/mod.rs` and split
- `crates/cli/src/checks/docs/mod.rs` - Update `mod toc;`
- `crates/cli/src/checks/docs/specs.rs` - May need path updates for toc imports

**Note:** Preserve `#[cfg(test)] #[path = "toc_tests.rs"] mod tests;` in `toc/mod.rs`.

**Verification:**
```bash
cargo build --all
cargo test --test specs checks_docs
cargo test toc
```

---

### Phase 6: Final Verification

**Goal:** Ensure all changes maintain backward compatibility and pass CI.

**Steps:**
1. Run `make check` to verify all CI checks pass
2. Run quench on quench to verify dogfooding still works
3. Verify all 54 docs behavioral specs pass
4. Verify no regressions in other checks

**Verification:**
```bash
make check
cargo run -- check --docs
cargo test --test specs checks_docs -- --nocapture 2>&1 | grep -c PASS
# Should show 54+ specs
```

## Key Implementation Details

### Shared Glob Pattern

The existing `build_glob_set` in `crate::adapter::glob` is already well-tested:

```rust
// crates/cli/src/adapter/glob.rs
pub fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}
```

### Check Level Helper Pattern

The helper centralizes the check level resolution:

```rust
/// Resolve effective check level with fallback to parent.
pub(super) fn is_check_enabled(subcheck: Option<&str>, parent: Option<&str>) -> bool {
    matches!(subcheck.or(parent).unwrap_or("error"), "error" | "warn")
}
```

### Module Split Strategy

When splitting `toc.rs`:

1. **Keep public API stable** - `toc::validate_toc`, `toc::extract_fenced_blocks`, `toc::looks_like_tree`, `toc::parse_tree_block` remain accessible via `toc/mod.rs`

2. **Preserve test module** - Keep `#[path = "toc_tests.rs"]` pointing to existing test file

3. **Internal visibility** - Use `pub(super)` for types shared between toc submodules, `pub(crate)` only where needed externally (like `specs.rs` using toc parsing)

### Lines of Code Impact

| Module | Before | After | Change |
|--------|--------|-------|--------|
| toc.rs | 744 | ~150 (mod.rs) | Split into 4 files |
| toc/parse.rs | - | ~200 | New |
| toc/resolve.rs | - | ~150 | New |
| toc/detect.rs | - | ~200 | New |
| links.rs | 203 | ~180 | Remove duplication |
| specs.rs | 456 | ~450 | Remove dead code marker |
| mod.rs | 80 | ~120 | Add helpers |

## Verification Plan

### Phase 1 Verification
```bash
cargo build --all
cargo clippy --all-targets --all-features -- -D warnings
grep -r "KEEP UNTIL" crates/cli/src/checks/docs/
# Should return no matches
```

### Phase 2 Verification
```bash
cargo build --all
cargo test --test specs checks_docs
# All 54 specs should pass
```

### Phase 3 Verification
```bash
cargo test --test specs checks_docs
# Verify toc, links, specs all respect disabled state
```

### Phase 4 Verification
```bash
cargo test --test specs checks_docs
# Verify file iteration still works correctly
```

### Phase 5 Verification
```bash
cargo build --all
cargo test toc
cargo test --test specs checks_docs
# All toc unit tests and specs pass
```

### Phase 6 (Final) Verification
```bash
make check
# All CI checks pass

cargo run -- check --docs
# Quench validates its own docs

cargo test --test specs checks_docs 2>&1 | grep -E "^test.*ok$" | wc -l
# Should show 54+ passing tests
```

## Exit Criteria

- [ ] `KEEP UNTIL` markers removed from docs check modules
- [ ] Duplicate `build_glob_set` eliminated (using shared version)
- [ ] Check level logic consolidated into helper
- [ ] File iteration pattern extracted (optional, based on complexity)
- [ ] `toc.rs` split into focused submodules
- [ ] All 54 docs behavioral specs pass
- [ ] `make check` passes
- [ ] `quench check --docs` passes on quench repo
