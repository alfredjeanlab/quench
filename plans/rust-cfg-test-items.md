# Plan: Distinguish #[cfg(test)] Item Types

## Overview

The `inline_cfg_test` check currently treats all `#[cfg(test)]` blocks identically, producing the message "Move tests to a sibling _tests.rs file" regardless of what item follows the attribute. This message is only appropriate for test **modules** (e.g., `mod tests { ... }`). For other item types like functions, structs, or impl blocks containing test helpers, the message is misleading.

This plan adds item-type detection to the cfg_test parser, enabling distinct violation messages for different item kinds.

## Project Structure

```
crates/cli/src/adapter/rust/
├── cfg_test.rs       # Main parsing logic (modify)
├── cfg_test_tests.rs # Unit tests (modify)
└── mod.rs            # Module re-exports (no change)

crates/cli/src/checks/
└── cloc.rs           # Violation creation (modify)

docs/specs/langs/
└── rust.md           # Specification (modify)

tests/fixtures/rust/
└── cfg-test-items/   # New fixture for item type tests
```

## Dependencies

No new dependencies required. Uses existing lexer-based parsing approach.

## Implementation Phases

### Phase 1: Extend CfgTestBlock with Item Kind

Add an enum to represent the item type following `#[cfg(test)]`:

```rust
// In cfg_test.rs

/// The kind of item following a #[cfg(test)] attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfgTestItemKind {
    /// `mod foo { ... }` - test module
    Mod,
    /// `fn foo() { ... }` - function (e.g., test helper)
    Fn,
    /// `impl Foo { ... }` - impl block with test methods
    Impl,
    /// `struct Foo { ... }` - test-only struct
    Struct,
    /// `enum Foo { ... }` - test-only enum
    Enum,
    /// `type Foo = ...;` - test-only type alias
    Type,
    /// `trait Foo { ... }` - test-only trait
    Trait,
    /// `const FOO: ...` - test-only constant
    Const,
    /// `static FOO: ...` - test-only static
    Static,
    /// Unknown or macro-generated item
    Unknown,
}

pub struct CfgTestBlock {
    pub attr_line: usize,
    pub range: Range<usize>,
    pub item_kind: CfgTestItemKind,  // NEW
}
```

**Milestone**: `CfgTestBlock` has an `item_kind` field set to `Unknown` for all blocks.

### Phase 2: Parse Item Kind from First Non-Attribute Line

Detect item kind by looking at the first token of the line following the `#[cfg(test)]` attribute (after skipping additional attributes like `#[path = "..."]`):

```rust
fn detect_item_kind(line: &str) -> CfgTestItemKind {
    let trimmed = line.trim();

    // Skip visibility modifiers: pub, pub(crate), pub(super), etc.
    let after_vis = skip_visibility(trimmed);

    // Skip additional modifiers: async, unsafe, const (for fn)
    let after_mods = skip_fn_modifiers(after_vis);

    // Match first keyword
    if after_mods.starts_with("mod ") {
        CfgTestItemKind::Mod
    } else if after_mods.starts_with("fn ") {
        CfgTestItemKind::Fn
    } else if after_mods.starts_with("impl ") || after_mods.starts_with("impl<") {
        CfgTestItemKind::Impl
    } else if after_mods.starts_with("struct ") {
        CfgTestItemKind::Struct
    } else if after_mods.starts_with("enum ") {
        CfgTestItemKind::Enum
    } else if after_mods.starts_with("type ") {
        CfgTestItemKind::Type
    } else if after_mods.starts_with("trait ") {
        CfgTestItemKind::Trait
    } else if after_mods.starts_with("const ") {
        CfgTestItemKind::Const
    } else if after_mods.starts_with("static ") {
        CfgTestItemKind::Static
    } else {
        CfgTestItemKind::Unknown
    }
}

fn skip_visibility(s: &str) -> &str {
    if s.starts_with("pub(") {
        // Handle pub(crate), pub(super), pub(in path)
        if let Some(end) = s.find(')') {
            return s[end + 1..].trim_start();
        }
    } else if s.starts_with("pub ") {
        return &s[4..];
    }
    s
}

fn skip_fn_modifiers(s: &str) -> &str {
    let mut result = s;
    for modifier in ["async ", "unsafe ", "const ", "extern "] {
        if result.starts_with(modifier) {
            result = &result[modifier.len()..];
        }
    }
    // Handle extern "C" fn
    if result.starts_with('"') {
        if let Some(end) = result[1..].find('"') {
            result = result[end + 2..].trim_start();
        }
    }
    result
}
```

**Milestone**: All existing tests pass, `item_kind` correctly identified for each block.

### Phase 3: Differentiate Violation Messages

Update `cloc.rs` to produce different messages based on item kind:

```rust
fn create_inline_cfg_test_violation(
    ctx: &CheckContext,
    file_path: &Path,
    block: &CfgTestBlock,
) -> Violation {
    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    let line = block.attr_line as u32 + 1;

    let (code, advice) = match block.item_kind {
        CfgTestItemKind::Mod => (
            "inline_cfg_test",
            "Move tests to a sibling _tests.rs file.",
        ),
        CfgTestItemKind::Fn | CfgTestItemKind::Impl => (
            "cfg_test_helper",
            "Move test helper to the _tests.rs file, or use #[doc(hidden)] if needed in both.",
        ),
        CfgTestItemKind::Struct | CfgTestItemKind::Enum | CfgTestItemKind::Type
        | CfgTestItemKind::Trait | CfgTestItemKind::Const | CfgTestItemKind::Static => (
            "cfg_test_item",
            "Move test-only type to the _tests.rs file.",
        ),
        CfgTestItemKind::Unknown => (
            "inline_cfg_test",
            "Move tests to a sibling _tests.rs file.",
        ),
    };

    Violation::file(display_path, line, code, advice)
}
```

Violation codes:
- `inline_cfg_test` - test module (existing)
- `cfg_test_helper` - function/impl likely used as test helper
- `cfg_test_item` - test-only type definitions

**Milestone**: Different messages appear for different item types.

### Phase 4: Update Specification and Tests

1. Update `docs/specs/langs/rust.md` to document the three violation types
2. Add fixture `tests/fixtures/rust/cfg-test-items/` with examples:
   - `src/lib.rs` - module, function, impl, struct with `#[cfg(test)]`
3. Add behavioral tests in `tests/specs/` for each violation type

**Milestone**: Spec tests pass, documentation is complete.

### Phase 5: Consider Allowing Test Helpers in Impl Blocks

A common pattern is having a test-only constructor in an impl block:

```rust
impl TuiAppState {
    #[cfg(test)]
    pub fn for_test(...) -> Self { ... }
}
```

Options:
1. **Current approach**: Flag with `cfg_test_helper` advice to use `#[doc(hidden)]`
2. **Alternative**: Allow `#[cfg(test)]` on individual methods within non-test impl blocks

For now, use option 1 (flag with guidance). If user feedback suggests allowing option 2, add a configuration option like:

```toml
[rust]
cfg_test_split = "require"
allow_cfg_test_methods = true  # Future: allow #[cfg(test)] on impl methods
```

**Milestone**: Clear guidance for test helper pattern.

## Key Implementation Details

### Parsing Strategy

The current parser already processes lines sequentially looking for `#[cfg(test)]`. The modification captures the first non-attribute line after detecting the attribute:

```rust
// In the main parse loop, after detecting #[cfg(test)]:
if in_cfg_test && waiting_for_block_start {
    if !trimmed.starts_with("#[") {
        // This is the item line - capture its kind
        pending_item_kind = Some(detect_item_kind(trimmed));
    }
}
```

### Edge Cases

1. **Multi-line item definitions**: `pub async unsafe fn` split across lines
   - Handle by accumulating modifiers until a keyword is found
2. **Macro invocations**: `#[cfg(test)] my_macro!(...)`
   - Classify as `Unknown`
3. **Attributes between cfg and item**: Already skipped in current parser

### Backward Compatibility

The violation code `inline_cfg_test` remains for modules, so existing suppressions continue to work. New codes (`cfg_test_helper`, `cfg_test_item`) are additive.

## Verification Plan

1. **Unit tests** (`cfg_test_tests.rs`):
   - `detect_item_kind` correctly identifies each item type
   - Visibility modifiers are skipped properly
   - Function modifiers (async, unsafe, const, extern) are handled
   - `CfgTestBlock.item_kind` is populated correctly

2. **Integration tests** (`tests/specs/`):
   - Fixture with all item types produces correct violation codes
   - `inline_cfg_test` for `mod`
   - `cfg_test_helper` for `fn` and `impl`
   - `cfg_test_item` for `struct`, `enum`, `type`, `trait`, `const`, `static`

3. **Manual verification**:
   - Run quench on the `crates/cli/src/tui/app/state.rs` example from the issue
   - Confirm it now produces `cfg_test_helper` with appropriate advice

4. **Existing test suite**:
   - All existing cfg_test tests pass unchanged
   - No regressions in LOC counting behavior
