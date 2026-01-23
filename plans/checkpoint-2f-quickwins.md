# Checkpoint 2F: Quick Wins Cleanup - CLOC Works

**Root Feature:** `quench-fda7`

## Overview

Code cleanup checkpoint to remove dead code, temporary scaffolding, and consolidate duplicate logic in the CLOC implementation. The CLOC check is now fully functional (checkpoint-2e completed), making this the ideal time to clean up the codebase before proceeding to additional checks.

**Cleanup targets identified:**

| Category | Location | Impact |
|----------|----------|--------|
| Dead code | `reader.rs` module | ~140 lines removed |
| Duplicate logic | `cloc.rs` violation creation | ~30 lines consolidated |
| Inconsistent patterns | `strip_prefix` handling | Standardized approach |

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── lib.rs               # Module exports (remove reader)
│   ├── reader.rs            # DELETE - unused file reader module
│   ├── reader_tests.rs      # DELETE - tests for dead code
│   └── checks/
│       ├── cloc.rs          # Consolidate duplicate logic
│       └── cloc_tests.rs    # Update tests if needed
└── plans/
    └── checkpoint-2f-quickwins.md
```

## Dependencies

No new dependencies. This is a pure cleanup phase.

## Implementation Phases

### Phase 1: Remove Dead reader.rs Module

The `reader.rs` module exports `FileReader`, `FileContent`, and `ReadStrategy` but they are never used anywhere in the codebase. The CLOC check uses `std::fs::read()` directly in `count_file_metrics()`.

**Evidence:** Grep for `FileReader|FileContent|ReadStrategy` shows only references in:
- `reader.rs` (definition)
- `reader_tests.rs` (tests)
- `lib.rs` (export)

No actual usage in any check, runner, or CLI code.

**Changes:**

1. Delete `crates/cli/src/reader.rs`
2. Delete `crates/cli/src/reader_tests.rs`
3. Remove from `crates/cli/src/lib.rs`:
   ```rust
   // Remove these lines:
   pub mod reader;
   pub use reader::{FileContent, FileReader, ReadStrategy};
   ```

**Milestone:** ~240 lines of dead code removed (reader.rs + reader_tests.rs).

**Verification:**
```bash
cargo build -p quench
cargo test -p quench
```

---

### Phase 2: Consolidate Violation Creation Logic

The `run()` method in `cloc.rs` has duplicate code for creating line count and token count violations (lines 98-142). Both branches:
- Check violation limits identically
- Compute `display_path` with `strip_prefix` identically
- Select advice based on `is_test` identically
- Create violations with the same structure

**Current code (simplified):**
```rust
// Line count violation (lines 98-117)
if line_count > max_lines {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit && current >= limit {
        break;
    }
    let display_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
    let advice = if is_test { ... } else { ... };
    violations.push(Violation::file_only(...));
}

// Token count violation (lines 120-142) - NEARLY IDENTICAL
if let Some(max_tokens) = cloc_config.max_tokens && token_count > max_tokens {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit && current >= limit {
        break;
    }
    let display_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
    let advice = if is_test { ... } else { ... };
    violations.push(Violation::file_only(...));
}
```

**Refactored approach:**

Extract a helper that encapsulates violation limit checking and creation:

```rust
/// Check violation limit and maybe create a violation.
/// Returns Some(violation) if under limit, None if limit exceeded.
fn check_and_create_violation(
    ctx: &CheckContext,
    file_path: &Path,
    is_test: bool,
    advice: &Option<String>,
    advice_test: &Option<String>,
    value: usize,
    threshold: usize,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit && current >= limit {
        return None;
    }

    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    let advice = if is_test {
        advice_test.clone()
    } else {
        advice.clone()
    };

    Some(
        Violation::file_only(display_path, "file_too_large", advice)
            .with_threshold(value as i64, threshold as i64),
    )
}
```

**Simplified call sites:**
```rust
if line_count > max_lines {
    match check_and_create_violation(ctx, &file.path, is_test,
        &cloc_config.advice, &cloc_config.advice_test, line_count, max_lines)
    {
        Some(v) => violations.push(v),
        None => break,
    }
}

if let Some(max_tokens) = cloc_config.max_tokens && token_count > max_tokens {
    match check_and_create_violation(ctx, &file.path, is_test,
        &cloc_config.advice, &cloc_config.advice_test, token_count, max_tokens)
    {
        Some(v) => violations.push(v),
        None => break,
    }
}
```

**Milestone:** Duplicate violation logic consolidated into single helper.

**Verification:**
```bash
cargo test -p quench -- cloc
cargo clippy -p quench -- -D warnings
```

---

### Phase 3: Standardize strip_prefix Pattern

The code uses `path.strip_prefix(root)` in multiple places with inconsistent error handling:

| Location | Pattern | Behavior |
|----------|---------|----------|
| `file_package()` line 228 | `.ok()?` | Returns `None` on failure |
| Violation creation (2x) | `.unwrap_or(path)` | Falls back to full path |
| `PatternMatcher` (2x) | `.unwrap_or(path)` | Falls back to full path |

**Standardize to:** Use `.unwrap_or(path)` everywhere for consistency. The fallback to full path is safer and matches the majority of existing code.

**Add helper method to PatternMatcher:**
```rust
impl PatternMatcher {
    /// Convert absolute path to relative for pattern matching.
    fn relative_path<'a>(path: &'a Path, root: &Path) -> &'a Path {
        path.strip_prefix(root).unwrap_or(path)
    }

    fn is_test_file(&self, path: &Path, root: &Path) -> bool {
        self.test_patterns.is_match(Self::relative_path(path, root))
    }

    fn is_excluded(&self, path: &Path, root: &Path) -> bool {
        self.exclude_patterns.is_match(Self::relative_path(path, root))
    }
}
```

**Update file_package:**
```rust
fn file_package(path: &Path, root: &Path, packages: &[String]) -> Option<String> {
    // Use unwrap_or for consistency; if path isn't under root, no package matches
    let relative = path.strip_prefix(root).ok()?;  // Keep .ok()? here - semantically correct
    // ... rest unchanged
}
```

**Decision:** Actually, `.ok()?` is correct for `file_package()` because if the path isn't under root, it can't belong to any package. Keep as-is but add a comment explaining why.

**Milestone:** Consistent path handling throughout CLOC check.

**Verification:**
```bash
cargo test -p quench -- cloc
cargo test --test '*' -- cloc  # spec tests
```

---

### Phase 4: Final Cleanup and Verification

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

## Key Implementation Details

### Why Remove reader.rs?

The `reader.rs` module was likely scaffolding for a planned feature:
- Size-gated file reading (skip files > 10MB)
- Memory-mapped I/O support (commented as disabled due to unsafe lint)

However, the CLOC check uses `std::fs::read()` directly, which is simpler and sufficient:
```rust
fn count_file_metrics(path: &Path) -> std::io::Result<FileMetrics> {
    let content = std::fs::read(path)?;  // Direct read, no size gating
    // ...
}
```

The 10MB limit check is not needed because:
1. Source files are typically < 1MB
2. The file walker already filters by extension (no binary files)
3. If a truly massive source file exists, it's a valid violation

### Violation Helper Design

The helper uses `Option<Violation>` return type to signal whether the limit was exceeded:
- `Some(violation)` → Under limit, add to list
- `None` → Limit exceeded, break the loop

This avoids the previous pattern of checking limit and immediately breaking, which was duplicated twice.

### No Behavior Changes

This cleanup phase should produce **no observable behavior changes**:
- Same violations reported
- Same metrics calculated
- Same output format
- Same performance characteristics

All changes are internal refactoring.

## Verification Plan

1. **Before any changes** - snapshot current behavior:
   ```bash
   cargo build --release
   ./target/release/quench check tests/fixtures/bench-medium --json > /tmp/before.json
   ```

2. **After all changes** - verify identical output:
   ```bash
   cargo build --release
   ./target/release/quench check tests/fixtures/bench-medium --json > /tmp/after.json
   diff /tmp/before.json /tmp/after.json  # Should be empty
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Lines Affected | Status |
|-------|------|----------------|--------|
| 1 | Remove dead reader.rs module | -240 lines | [ ] Pending |
| 2 | Consolidate violation creation | -20 lines | [ ] Pending |
| 3 | Standardize strip_prefix pattern | ~10 lines | [ ] Pending |
| 4 | Final verification | 0 lines | [ ] Pending |

## Notes

- Total expected reduction: ~250 lines of code
- No new dependencies
- No behavior changes
- All changes are internal refactoring for maintainability
