# Plan: Config Module Refactor

## Overview

Extract test configuration types from `crates/cli/src/config/mod.rs` (776 lines) into a dedicated `tests.rs` submodule to bring the file under the 750-line limit. This follows the established pattern used for language-specific configs (`go.rs`, `shell.rs`, etc.).

## Project Structure

```
crates/cli/src/config/
├── mod.rs           # Main config (776 → ~603 lines after extraction)
├── tests.rs         # NEW: Test configuration types (~173 lines)
├── checks.rs        # Existing: check-level configs
├── go.rs            # Existing: Go language config
├── javascript.rs    # Existing: JavaScript config
├── ratchet.rs       # Existing: Ratchet config
├── shell.rs         # Existing: Shell config
└── suppress.rs      # Existing: Suppress config
```

## Dependencies

No new dependencies required. The extraction uses only:
- `serde::Deserialize` (already available)
- `super::duration` (sibling module)

## Implementation Phases

### Phase 1: Create `tests.rs` Module

Extract the following types from `mod.rs` lines 448-620:
- `TestsConfig`
- `TestSuiteConfig`
- `TestsTimeConfig`
- `TestsCommitConfig`

**File: `crates/cli/src/config/tests.rs`**

```rust
//! Test suite configuration.

use serde::Deserialize;

use super::duration;

/// Tests check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,
    // ... remaining fields
}

// ... TestSuiteConfig, TestsTimeConfig, TestsCommitConfig
```

### Phase 2: Update `mod.rs`

1. Add module declaration:
   ```rust
   mod tests;
   ```

2. Add re-export:
   ```rust
   pub use tests::{TestsConfig, TestSuiteConfig, TestsTimeConfig, TestsCommitConfig};
   ```

3. Remove the extracted types (lines 448-620)

### Phase 3: Verify

Run `make check` to ensure:
- All tests pass
- No clippy warnings
- File is under 750 lines

## Key Implementation Details

### Module Pattern

Follow the established pattern from `shell.rs` and `go.rs`:
- Module-level doc comment
- Import `serde::Deserialize`
- Import shared types from `super::`
- Self-contained structs with `Default` impls
- Keep `default_*` helper methods as `pub(crate)`

### Import Requirements

The `tests.rs` module needs:
```rust
use serde::Deserialize;
use super::duration;  // For deserialize_option on Duration fields
```

### Line Count Analysis

| Section | Lines | Action |
|---------|-------|--------|
| Current mod.rs | 776 | Over limit |
| Tests types (448-620) | ~173 | Extract |
| Expected mod.rs | ~603 | Under limit |

## Verification Plan

1. **Compilation**: `cargo build --all`
2. **Tests**: `cargo test --all`
3. **Linting**: `cargo clippy --all-targets --all-features -- -D warnings`
4. **Line count**: `quench check` should pass cloc check
5. **Full check**: `make check`
