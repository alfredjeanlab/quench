# cfg-test-split: Configurable `#[cfg(test)]` Handling Modes

**Plan:** `cfg-test-split`
**Feature Branch:** `feature/cfg-test-split`
**Root Feature:** `quench-9937`

## Overview

Extend the `[rust].cfg_test_split` configuration from a boolean to a three-mode enum that controls how `#[cfg(test)]` blocks are handled during LOC counting. This enables projects to enforce the sibling `_tests.rs` file convention by failing on inline test modules.

| Mode | Behavior |
|------|----------|
| `"count"` | Split `#[cfg(test)]` blocks into test LOC (current `true` behavior) |
| `"require"` | Fail if source files contain inline `#[cfg(test)]` blocks |
| `"off"` | Count all lines as source LOC, don't parse for `#[cfg(test)]` (current `false` behavior) |

## Project Structure

Files to modify:

```
crates/cli/src/
├── config/
│   ├── mod.rs          # MODIFY: Add CfgTestSplitMode enum, update RustConfig
│   └── parse.rs        # MODIFY: Parse string/bool with backward compat
├── checks/
│   └── cloc.rs         # MODIFY: Handle all three modes, create violations for "require"
└── adapter/
    └── rust/
        ├── mod.rs      # MODIFY: Add method to detect inline cfg(test) locations
        └── cfg_test.rs # MODIFY: Return line locations for violation reporting
tests/
├── specs/
│   └── adapters/
│       └── rust.rs     # MODIFY: Add specs for cfg_test_split modes
└── fixtures/
    └── rust/
        ├── inline-cfg-test/  # NEW: Fixture with inline #[cfg(test)]
        └── sibling-tests/    # NEW: Fixture with _tests.rs pattern
```

## Dependencies

- No new external crates required
- Uses existing `serde` deserialization infrastructure
- Uses existing `CfgTestInfo` parsing logic

## Implementation Phases

### Phase 1: Add CfgTestSplitMode Enum

Add the new enum type and update `RustConfig` to use it, maintaining backward compatibility.

**File:** `crates/cli/src/config/mod.rs`

```rust
/// Mode for handling #[cfg(test)] blocks in Rust files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CfgTestSplitMode {
    /// Split #[cfg(test)] blocks into test LOC (default).
    #[default]
    Count,
    /// Fail if source files contain inline #[cfg(test)] blocks.
    Require,
    /// Count all lines as source LOC, don't parse for #[cfg(test)].
    Off,
}
```

Update `RustConfig`:

```rust
pub struct RustConfig {
    /// How to handle #[cfg(test)] blocks (default: "count").
    pub cfg_test_split: CfgTestSplitMode,
    // ... rest unchanged
}
```

**Verification:**
- [ ] `cargo build` compiles with new enum type
- [ ] Default value is `CfgTestSplitMode::Count`

### Phase 2: Implement Backward-Compatible Parsing

Update config parsing to accept both boolean (legacy) and string (new) values.

**File:** `crates/cli/src/config/parse.rs`

```rust
/// Parse cfg_test_split from TOML value.
/// Supports both legacy boolean and new string modes.
fn parse_cfg_test_split(value: Option<&toml::Value>) -> CfgTestSplitMode {
    match value {
        // Legacy boolean support
        Some(toml::Value::Boolean(true)) => CfgTestSplitMode::Count,
        Some(toml::Value::Boolean(false)) => CfgTestSplitMode::Off,
        // New string modes
        Some(toml::Value::String(s)) => match s.as_str() {
            "count" => CfgTestSplitMode::Count,
            "require" => CfgTestSplitMode::Require,
            "off" => CfgTestSplitMode::Off,
            _ => CfgTestSplitMode::Count, // Default on unknown
        },
        None => CfgTestSplitMode::Count,
        _ => CfgTestSplitMode::Count,
    }
}
```

Update `parse_rust_config()` to use the new parser:

```rust
pub(super) fn parse_rust_config(value: Option<&toml::Value>) -> RustConfig {
    let Some(toml::Value::Table(t)) = value else {
        return RustConfig::default();
    };

    let cfg_test_split = parse_cfg_test_split(t.get("cfg_test_split"));
    // ... rest unchanged
}
```

**Verification:**
- [ ] `cfg_test_split = true` parses as `Count`
- [ ] `cfg_test_split = false` parses as `Off`
- [ ] `cfg_test_split = "count"` parses as `Count`
- [ ] `cfg_test_split = "require"` parses as `Require`
- [ ] `cfg_test_split = "off"` parses as `Off`
- [ ] Missing value defaults to `Count`

### Phase 3: Extend CfgTestInfo for Violation Reporting

Add location information to `CfgTestInfo` so violations can report the line number where inline `#[cfg(test)]` appears.

**File:** `crates/cli/src/adapter/rust/cfg_test.rs`

```rust
/// Information about a single #[cfg(test)] block.
#[derive(Debug)]
pub struct CfgTestBlock {
    /// Line number where the attribute starts (0-indexed).
    pub attr_line: usize,
    /// Line range of the entire block (attribute through closing brace).
    pub range: Range<usize>,
}

#[derive(Debug, Default)]
pub struct CfgTestInfo {
    /// Detailed block information (for violation reporting).
    pub blocks: Vec<CfgTestBlock>,
    /// Line ranges (0-indexed) that are inside #[cfg(test)] blocks.
    /// Derived from blocks for backward compatibility.
    pub test_ranges: Vec<Range<usize>>,
}

impl CfgTestInfo {
    /// Check if file has any inline #[cfg(test)] blocks.
    pub fn has_inline_tests(&self) -> bool {
        !self.blocks.is_empty()
    }

    /// Get the first inline test location (for violation reporting).
    pub fn first_inline_test_line(&self) -> Option<usize> {
        self.blocks.first().map(|b| b.attr_line)
    }
}
```

Update `parse()` to populate `blocks`:

```rust
impl CfgTestInfo {
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        // ... existing logic, but also track attr_line

        // When detecting cfg(test):
        info.blocks.push(CfgTestBlock {
            attr_line: block_start,  // Line where #[cfg(test)] appears
            range: block_start..line_idx + 1,
        });

        // Rebuild test_ranges from blocks for compatibility
        info.test_ranges = info.blocks.iter().map(|b| b.range.clone()).collect();

        info
    }
}
```

**Verification:**
- [ ] `CfgTestInfo::parse()` populates both `blocks` and `test_ranges`
- [ ] `has_inline_tests()` returns true when `#[cfg(test)]` present
- [ ] `first_inline_test_line()` returns correct line number

### Phase 4: Update Cloc Check for All Modes

Modify the cloc check to handle all three modes and generate violations for `require` mode.

**File:** `crates/cli/src/checks/cloc.rs`

```rust
use crate::config::CfgTestSplitMode;

impl Check for ClocCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // ... existing setup ...

        let rust_config = &ctx.config.rust;

        // Only create adapter for modes that need parsing
        let rust_adapter = match rust_config.cfg_test_split {
            CfgTestSplitMode::Count | CfgTestSplitMode::Require => Some(RustAdapter::new()),
            CfgTestSplitMode::Off => None,
        };

        // ... in file processing loop ...

        for file in ctx.files {
            // ... existing file processing ...

            // Handle Rust-specific logic based on mode
            if is_rust_source && let Some(adapter) = rust_adapter.as_ref() {
                let content = match std::fs::read_to_string(&file.path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                match rust_config.cfg_test_split {
                    CfgTestSplitMode::Require => {
                        // Check for inline tests and generate violation
                        let cfg_info = CfgTestInfo::parse(&content);
                        if cfg_info.has_inline_tests() {
                            if let Some(line) = cfg_info.first_inline_test_line() {
                                violations.push(create_inline_cfg_test_violation(
                                    ctx, &file.path, line as u32 + 1
                                ));
                            }
                        }
                        // Still count as source (no splitting)
                        (nonblank_lines, 0, false)
                    }
                    CfgTestSplitMode::Count => {
                        // Existing behavior: split source/test
                        let classification = adapter.classify_lines(relative_path, &content);
                        let is_test = classification.test_lines > classification.source_lines;
                        (classification.source_lines, classification.test_lines, is_test)
                    }
                    CfgTestSplitMode::Off => unreachable!(), // Adapter is None
                }
            }
            // ... rest of processing ...
        }
    }
}

/// Create a violation for inline #[cfg(test)] block.
fn create_inline_cfg_test_violation(
    ctx: &CheckContext,
    file_path: &Path,
    line: u32,
) -> Violation {
    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    Violation::file(
        display_path,
        line,
        "inline_cfg_test",
        "Move tests to a sibling _tests.rs file.",
    )
}
```

**Verification:**
- [ ] `cfg_test_split = "count"` splits LOC (existing behavior)
- [ ] `cfg_test_split = "off"` counts all as source
- [ ] `cfg_test_split = "require"` generates `inline_cfg_test` violation
- [ ] Violation includes correct line number

### Phase 5: Add Test Fixtures

Create fixtures to test all three modes.

**Fixture:** `tests/fixtures/rust/inline-cfg-test/`

```
tests/fixtures/rust/inline-cfg-test/
├── quench.toml
└── src/
    └── lib.rs
```

`quench.toml`:
```toml
version = 1

[rust]
cfg_test_split = "require"
```

`src/lib.rs`:
```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
```

**Fixture:** `tests/fixtures/rust/sibling-tests/`

```
tests/fixtures/rust/sibling-tests/
├── quench.toml
└── src/
    ├── lib.rs
    └── lib_tests.rs
```

`quench.toml`:
```toml
version = 1

[rust]
cfg_test_split = "require"
```

`src/lib.rs`:
```rust
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

`src/lib_tests.rs`:
```rust
use super::*;

#[test]
fn test_add() {
    assert_eq!(add(1, 2), 3);
}
```

**Verification:**
- [ ] `inline-cfg-test` fixture triggers `inline_cfg_test` violation
- [ ] `sibling-tests` fixture passes with `require` mode

### Phase 6: Add Behavioral Specs

Add specs to `tests/specs/adapters/rust.rs` for the new modes.

```rust
// =============================================================================
// CFG_TEST_SPLIT MODE SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "count" (default): Split #[cfg(test)] blocks into test LOC
#[test]
fn rust_cfg_test_split_count_separates_source_and_test() {
    let cloc = check("cloc").on("rust/inline-cfg-test-count").json().passes();
    let metrics = cloc.require("metrics");

    // Source and test lines should be separated
    assert!(metrics.get("source_lines").and_then(|v| v.as_u64()).unwrap() > 0);
    assert!(metrics.get("test_lines").and_then(|v| v.as_u64()).unwrap() > 0);
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "require": Fail if source files contain inline #[cfg(test)]
#[test]
fn rust_cfg_test_split_require_fails_on_inline_tests() {
    let cloc = check("cloc").on("rust/inline-cfg-test").json().fails();

    assert!(cloc.has_violation("inline_cfg_test"));
    let v = cloc.require_violation("inline_cfg_test");
    assert!(v.get("line").is_some(), "violation should have line number");
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "require": Sibling _tests.rs pattern passes
#[test]
fn rust_cfg_test_split_require_passes_with_sibling_tests() {
    check("cloc").on("rust/sibling-tests").passes();
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = "off": Count all lines as source LOC
#[test]
fn rust_cfg_test_split_off_counts_all_as_source() {
    let cloc = check("cloc").on("rust/inline-cfg-test-off").json().passes();
    let metrics = cloc.require("metrics");

    // All lines counted as source, none as test
    assert!(metrics.get("source_lines").and_then(|v| v.as_u64()).unwrap() > 0);
    assert_eq!(metrics.get("test_lines").and_then(|v| v.as_u64()), Some(0));
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = true (legacy): Same as "count"
#[test]
fn rust_cfg_test_split_true_is_count() {
    let cloc = check("cloc").on("rust/cfg-test-split-true").json().passes();
    let metrics = cloc.require("metrics");

    // Should split like "count" mode
    assert!(metrics.get("test_lines").and_then(|v| v.as_u64()).unwrap() > 0);
}

/// Spec: docs/specs/langs/rust.md#cfg-test-split-modes
///
/// > cfg_test_split = false (legacy): Same as "off"
#[test]
fn rust_cfg_test_split_false_is_off() {
    let cloc = check("cloc").on("rust/cfg-test-split-false").json().passes();
    let metrics = cloc.require("metrics");

    // Should count all as source like "off" mode
    assert_eq!(metrics.get("test_lines").and_then(|v| v.as_u64()), Some(0));
}
```

**Verification:**
- [ ] All new specs pass
- [ ] Existing specs continue to pass

## Key Implementation Details

### Backward Compatibility

The legacy boolean values map directly to enum variants:
- `true` → `CfgTestSplitMode::Count` (split test LOC)
- `false` → `CfgTestSplitMode::Off` (no parsing)

This ensures existing configs continue to work without changes.

### Violation Output Format

The `inline_cfg_test` violation follows the established pattern:

**Text output:**
```
cloc: FAIL
  src/lib.rs:5: inline_cfg_test
    Move tests to a sibling _tests.rs file.
```

**JSON output:**
```json
{
  "file": "src/lib.rs",
  "line": 5,
  "type": "inline_cfg_test",
  "advice": "Move tests to a sibling _tests.rs file."
}
```

### `#[path]` Attribute Handling

The `require` mode specifically targets inline `#[cfg(test)] mod tests { ... }` blocks with actual test code. The sibling file pattern uses `#[cfg(test)]` with `#[path]` to reference an external file:

```rust
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

This pattern does **not** trigger a violation because:
1. The `#[cfg(test)]` block contains only a module declaration, not test code
2. The brace counting finds no `{` after the attribute (just a `;`)
3. The actual tests live in the separate `_tests.rs` file

### Performance Considerations

- `"off"` mode is fastest (no file content reading for Rust files)
- `"count"` mode reads files once for line classification
- `"require"` mode reads files once, but returns early after detecting first `#[cfg(test)]`

## Verification Plan

### Unit Tests

```bash
# Config parsing tests
cargo test config::tests::parse_cfg_test_split

# CfgTestInfo parsing tests
cargo test adapter::rust::cfg_test::tests
```

### Behavioral Specs

```bash
# Run all Rust adapter specs
cargo test --test specs adapters::rust

# Run specific new specs
cargo test --test specs rust_cfg_test_split
```

### Manual Testing

```bash
# Test "require" mode on inline tests
cargo run -- tests/fixtures/rust/inline-cfg-test
# Expected: FAIL with inline_cfg_test violation

# Test "require" mode on sibling tests
cargo run -- tests/fixtures/rust/sibling-tests
# Expected: PASS

# Test legacy boolean compatibility
cargo run -- tests/fixtures/rust/cfg-test-split-true
cargo run -- tests/fixtures/rust/cfg-test-split-false
```

### Full Check Suite

```bash
make check
```

All existing tests should continue to pass.
