# Tech Debt: Adapter Consistency Improvements

## Problem

Language adapters have minor inconsistencies that add friction:
1. Inconsistent method naming (`is_ignored` vs `should_ignore`)
2. Policy wrapper files that add no value (just delegate to common)
3. Slight structural variations between adapters

## Files to Touch

### Method naming standardization

| File | Current | Proposed |
|------|---------|----------|
| `adapter/ruby/mod.rs` | `is_ignored()` | `should_ignore()` |
| `adapter/go/mod.rs` | `should_ignore()` | (keep) |
| `adapter/rust/mod.rs` | `should_ignore()` | (keep) |
| `adapter/javascript/mod.rs` | `should_ignore()` | (keep) |
| `adapter/shell/mod.rs` | `is_ignored()` | `should_ignore()` |

### Policy wrapper consolidation (optional)

These files are ~25 lines each and just call common:

| File | Lines | Content |
|------|-------|---------|
| `adapter/ruby/policy.rs` | 26 | 1-line delegation |
| `adapter/go/policy.rs` | 29 | 1-line delegation |
| `adapter/rust/policy.rs` | 29 | 1-line delegation |
| `adapter/javascript/policy.rs` | 29 | 1-line delegation |
| `adapter/shell/policy.rs` | 29 | 1-line delegation |

**Option A:** Keep as-is (allows future per-language customization)
**Option B:** Re-export common directly from adapter mod.rs

### Tests to update

| File | Changes |
|------|---------|
| `adapter/ruby/mod_tests.rs` | Rename `is_ignored` calls |
| `adapter/shell_tests.rs` | Rename `is_ignored` calls |

## Implementation

### Phase 1: Standardize naming

```rust
// In ruby/mod.rs, change:
pub fn is_ignored(&self, path: &Path) -> bool {
// To:
pub fn should_ignore(&self, path: &Path) -> bool {

// Same for shell/mod.rs
```

### Phase 2: Policy re-export (if doing Option B)

```rust
// In adapter/ruby/mod.rs, replace:
mod policy;
pub use policy::{PolicyCheckResult, check_lint_policy};

// With:
pub use crate::adapter::common::policy::{PolicyCheckResult, check_lint_policy};

// Then delete adapter/ruby/policy.rs and policy_tests.rs
```

### Phase 3: Document adapter pattern

Add to `adapter/mod.rs` or create `adapter/ARCHITECTURE.md`:

```rust
//! # Language Adapter Pattern
//!
//! Each language adapter follows this structure:
//!
//! ```text
//! adapter/{lang}/
//! ├── mod.rs           # {Lang}Adapter struct + Adapter trait impl
//! ├── mod_tests.rs     # Classification and pattern tests
//! ├── suppress.rs      # Lint directive parsing (language-specific)
//! ├── suppress_tests.rs
//! └── policy.rs        # (optional) Re-export or customize common policy
//! ```
//!
//! ## Required methods
//! - `new()` - Default patterns
//! - `with_patterns(ResolvedPatterns)` - Config-resolved patterns
//! - `should_ignore(&Path) -> bool` - Check ignore patterns
//!
//! ## Adapter trait
//! - `name() -> &'static str`
//! - `extensions() -> &'static [&'static str]`
//! - `classify(&Path) -> FileKind`
//! - `default_escapes() -> &'static [EscapePattern]`
```

## Decision Points

### Keep policy wrappers?

**Pro keep:**
- Future customization without changing common
- Clear per-language entry points
- Tests already exist

**Pro remove:**
- Less code to maintain
- No actual per-language logic exists
- Reduces file count by 10 (5 policy.rs + 5 policy_tests.rs)

**Recommendation:** Keep for now. The indirection cost is minimal and preserves extensibility.

### Document or abstract adapter pattern?

**Option A:** Document the pattern (this plan)
**Option B:** Create macro/trait defaults to reduce boilerplate

**Recommendation:** Document first. Macro abstraction adds complexity and the current pattern is clear enough. Revisit if adding more languages.

## Verification

```bash
# Check naming consistency
rg "fn (is_ignored|should_ignore)" crates/cli/src/adapter/

# Run all adapter tests
cargo test --all -- adapter

# Verify no broken references
cargo check --all
```

## Impact

- **Lines changed:** ~10-20 (naming only)
- **Lines removed:** 0 (if keeping policy wrappers) or ~150 (if removing)
- **Clarity:** Consistent API across all adapters
- **Documentation:** Clear pattern for future adapters
