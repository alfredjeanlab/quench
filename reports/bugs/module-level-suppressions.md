# Module-level suppressions not supported

## Description

Quench does not currently parse or handle module-level suppressions using `#![allow(...)]` syntax in Rust code.

### Current Behavior

Only item-level suppressions are recognized:
- `#[allow(...)]` - ✅ supported
- `#[expect(...)]` - ✅ supported
- `#![allow(...)]` - ❌ not supported (module/crate-level)

### Impact

Users cannot use module-level `#![allow(...)]` attributes to suppress lints for an entire module or crate, and must instead either:
- Apply suppressions individually to each item
- Use configuration-based allowlists in `quench.toml`

### Implementation

The suppression parser in `crates/cli/src/adapter/rust/suppress.rs` checks for lines starting with `#[allow(` or `#[expect(`, which doesn't match `#![...]` syntax. Support would require:

1. Updating docs/specs to document this behavior is supported
2. Adding "block-box" tests/specs for the correct behavior 
3. Updating `parse_suppress_line()` to recognize `#![...]` patterns
4. Determining the scope/applicability of module-level suppressions
5. Adding crate-level tests for module-level suppression handling

### Files Involved

- `crates/cli/src/adapter/rust/suppress.rs` - Suppression parsing
- `crates/cli/src/adapter/rust/suppress_tests.rs` - Tests
