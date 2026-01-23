# Checkpoint 4F: Quick Wins - Rust Adapter

**Root Feature:** `quench-4f7e`

## Overview

Code cleanup checkpoint for the Rust adapter following performance validation (checkpoint 4D) and stress testing (checkpoint 4E). The adapter is functionally complete and well-tested. This checkpoint focuses on reducing code duplication and removing minor redundancies.

**Cleanup targets identified:**

| Category | Location | Impact |
|----------|----------|--------|
| Duplicate code | `build_glob_set` function (3 copies) | ~30 lines consolidated |
| Redundant check | `workspace.rs` double `workspace.is_none()` | 3 lines removed |
| Unused import | `CargoWorkspace` re-export from `adapter::rust` | Minor cleanup |

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── mod.rs              # Add common glob module
│   │   ├── glob.rs             # NEW: shared build_glob_set
│   │   ├── generic.rs          # Remove local build_glob_set
│   │   └── rust/
│   │       ├── mod.rs          # Remove local build_glob_set
│   │       └── workspace.rs    # Remove redundant check
│   └── checks/
│       └── cloc.rs             # Use shared build_glob_set
└── plans/
    └── checkpoint-4f-quickwins.md
```

## Dependencies

No new dependencies. This is a pure cleanup phase.

## Implementation Phases

### Phase 1: Consolidate `build_glob_set` Function

The `build_glob_set` function is duplicated in three locations with nearly identical implementations:

| Location | Lines | Logging |
|----------|-------|---------|
| `adapter/generic.rs:80-90` | 11 | `tracing::warn` on invalid pattern |
| `adapter/rust/mod.rs:128-136` | 9 | Silent on invalid pattern |
| `checks/cloc.rs:311-321` | 11 | `tracing::warn` on invalid pattern |

**Approach:** Create a shared module `adapter/glob.rs` with the canonical implementation.

**New file `crates/cli/src/adapter/glob.rs`:**
```rust
//! Glob pattern utilities.

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Build a GlobSet from pattern strings.
///
/// Invalid patterns are logged and skipped.
pub fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(e) => {
                tracing::warn!("invalid glob pattern '{}': {}", pattern, e);
            }
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}
```

**Update `crates/cli/src/adapter/mod.rs`:**
```rust
// Add module declaration
pub mod glob;

// Re-export for convenience
pub use glob::build_glob_set;
```

**Update `crates/cli/src/adapter/generic.rs`:**
```rust
// Remove local build_glob_set function
// Replace with:
use super::glob::build_glob_set;
```

**Update `crates/cli/src/adapter/rust/mod.rs`:**
```rust
// Remove local build_glob_set function
// Replace with:
use super::super::glob::build_glob_set;
// Or use crate path:
use crate::adapter::glob::build_glob_set;
```

**Update `crates/cli/src/checks/cloc.rs`:**
```rust
// Remove local build_glob_set function
// Replace with:
use crate::adapter::glob::build_glob_set;
```

**Milestone:** Single source of truth for glob building.

**Verification:**
```bash
cargo build -p quench
cargo test -p quench
cargo clippy -p quench -- -D warnings
```

**Status:** [ ] Pending

---

### Phase 2: Remove Redundant Workspace Check

In `workspace.rs:45-60`, there's a redundant pattern:

```rust
// Line 45: First check
if workspace.is_none() {
    // Single package, not a workspace
    if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
        return Self { ... };
    }
    return Self::default();
}

// Line 58: Redundant check (workspace is guaranteed Some here)
let Some(workspace) = workspace else {
    return Self::default();
};
```

The second check is unnecessary because we already returned if `workspace.is_none()`.

**Fix:**
```rust
fn from_toml(value: &Value, root: &Path) -> Self {
    let Some(workspace) = value.get("workspace") else {
        // Single package, not a workspace
        if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
            return Self {
                is_workspace: false,
                packages: vec![pkg.as_str().unwrap_or("").to_string()],
                member_patterns: vec![],
            };
        }
        return Self::default();
    };

    // workspace is now bound and guaranteed to be Some
    let members = workspace
        .get("members")
        .and_then(|m| m.as_array())
        // ... rest unchanged
```

This uses `let-else` to combine the check and binding in one statement, eliminating the redundant check.

**Milestone:** Cleaner workspace parsing logic.

**Verification:**
```bash
cargo test -p quench -- workspace
```

**Status:** [ ] Pending

---

### Phase 3: Clean Up Module Re-exports

The `adapter::mod.rs` re-exports `CargoWorkspace` from the rust module, but it's only used in:
1. `main.rs:8` - imported via full path `quench::adapter::rust::CargoWorkspace`
2. `benches/stress.rs:15` - imported via full path
3. `benches/adapter.rs:15` - imported via full path

The re-export in `adapter/mod.rs` line 13 is not used (everything uses the full path):

```rust
// Current (line 13)
pub use rust::{CfgTestInfo, PolicyCheckResult, RustAdapter, parse_suppress_attrs};
// CargoWorkspace is not in this list, but is exported from rust/mod.rs
```

Actually, looking more carefully, `CargoWorkspace` is exported from `rust/mod.rs:22` and accessed via `adapter::rust::CargoWorkspace`. This is already clean.

**Review other re-exports:**

The `adapter/mod.rs:13` exports:
- `CfgTestInfo` - used in `checks/escapes.rs:13`
- `PolicyCheckResult` - not used directly (accessed via RustAdapter method)
- `RustAdapter` - used in `checks/escapes.rs:14`
- `parse_suppress_attrs` - used in `checks/escapes.rs:14`

`PolicyCheckResult` is only used internally via `RustAdapter::check_lint_policy()`. Consider removing from public re-export.

**Change in `crates/cli/src/adapter/mod.rs`:**
```rust
// Before
pub use rust::{CfgTestInfo, PolicyCheckResult, RustAdapter, parse_suppress_attrs};

// After - remove unused PolicyCheckResult re-export
pub use rust::{CfgTestInfo, RustAdapter, parse_suppress_attrs};
```

**Milestone:** Cleaner public API surface.

**Verification:**
```bash
cargo build -p quench
cargo test -p quench
```

**Status:** [ ] Pending

---

### Phase 4: Add Glob Module Tests

Create `crates/cli/src/adapter/glob_tests.rs` to test the consolidated function:

```rust
//! Tests for glob utilities.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn builds_empty_set_from_empty_patterns() {
    let set = build_glob_set(&[]);
    assert!(!set.is_match("anything.rs"));
}

#[test]
fn matches_simple_pattern() {
    let set = build_glob_set(&["*.rs".to_string()]);
    assert!(set.is_match("foo.rs"));
    assert!(!set.is_match("foo.txt"));
}

#[test]
fn matches_glob_star_pattern() {
    let set = build_glob_set(&["**/*.rs".to_string()]);
    assert!(set.is_match("src/foo.rs"));
    assert!(set.is_match("src/deep/nested/bar.rs"));
    assert!(!set.is_match("foo.txt"));
}

#[test]
fn skips_invalid_pattern() {
    // Invalid pattern (unclosed bracket) should be skipped
    let set = build_glob_set(&["[invalid".to_string(), "*.rs".to_string()]);
    // Valid pattern should still work
    assert!(set.is_match("foo.rs"));
}

#[test]
fn matches_multiple_patterns() {
    let set = build_glob_set(&[
        "*.rs".to_string(),
        "*.toml".to_string(),
    ]);
    assert!(set.is_match("lib.rs"));
    assert!(set.is_match("Cargo.toml"));
    assert!(!set.is_match("README.md"));
}
```

Add test module declaration in `glob.rs`:
```rust
#[cfg(test)]
#[path = "glob_tests.rs"]
mod tests;
```

**Milestone:** Shared glob function has dedicated tests.

**Verification:**
```bash
cargo test -p quench -- glob
```

**Status:** [ ] Pending

---

### Phase 5: Final Verification

Run the full verification suite to ensure all changes are correct.

**Verification checklist:**
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cargo build --all`
- [ ] `./scripts/bootstrap`
- [ ] `cargo audit`
- [ ] `cargo deny check`

**Run `make check` to verify all gates pass.**

**Milestone:** All quality gates pass, codebase is cleaner.

**Status:** [ ] Pending

## Key Implementation Details

### Why Consolidate build_glob_set?

The three implementations have minor differences:

1. **Error handling:** Two versions log warnings, one silently ignores errors
2. **Code duplication:** ~30 lines repeated across the codebase
3. **Maintenance risk:** Bug fixes need to be applied in multiple places

The consolidated version:
- Logs warnings (consistent with majority behavior)
- Includes the error message (helpful for debugging)
- Lives in a dedicated module (easy to find and modify)

### Why Remove PolicyCheckResult from Re-export?

The `PolicyCheckResult` type is only used as a return type from `RustAdapter::check_lint_policy()`. External code accesses it through the adapter:

```rust
let result = adapter.check_lint_policy(&files, &policy);
// result is PolicyCheckResult, but type is inferred
```

No external code imports `PolicyCheckResult` directly. Removing it from the top-level re-export:
- Reduces API surface
- Indicates it's an implementation detail
- Still accessible via `adapter::rust::PolicyCheckResult` if needed

### No Behavior Changes

This cleanup phase should produce **no observable behavior changes**:
- Same check results
- Same output format
- Same performance characteristics

All changes are internal refactoring for maintainability.

## Verification Plan

1. **Before any changes** - snapshot current behavior:
   ```bash
   cargo build --release
   ./target/release/quench check tests/fixtures/bench-rust --json > /tmp/before.json
   ```

2. **After all changes** - verify identical output:
   ```bash
   cargo build --release
   ./target/release/quench check tests/fixtures/bench-rust --json > /tmp/after.json
   diff /tmp/before.json /tmp/after.json  # Should be empty (except timing)
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Lines Affected | Status |
|-------|------|----------------|--------|
| 1 | Consolidate `build_glob_set` | +20 new, -30 removed | [ ] Pending |
| 2 | Remove redundant workspace check | -3 lines | [ ] Pending |
| 3 | Clean up module re-exports | -1 export | [ ] Pending |
| 4 | Add glob module tests | +40 lines | [ ] Pending |
| 5 | Final verification | 0 lines | [ ] Pending |

## Notes

- Total expected change: ~30 lines removed (net negative)
- No new dependencies
- No behavior changes
- All changes are internal refactoring for maintainability
- The Rust adapter remains functionally identical
