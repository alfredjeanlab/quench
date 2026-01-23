# Checkpoint 4C: Post-Checkpoint Refactor - Rust Adapter

**Root Feature:** `quench-4d66`

## Overview

This refactor checkpoint improves the code organization and maintainability of the Rust language adapter. The checkpoint 4B validation passed with all criteria met - no bugs were identified. This refactor focuses on structural improvements to prepare the adapter for future language additions and reduce cognitive load.

**Refactoring Goals:**
1. Extract distinct concerns into submodules
2. Improve string literal handling in `#[cfg(test)]` parser
3. Consolidate related types and reduce file size

The rust.rs file (566 lines) contains five distinct sections that should be separate modules:
- Core adapter (patterns, classification)
- `#[cfg(test)]` block parsing
- Cargo workspace parsing
- Suppress attribute parsing
- Lint policy checking

## Project Structure

Current structure to refactor:

```
quench/
├── crates/cli/src/adapter/
│   ├── mod.rs              # Adapter trait, registry, detection
│   ├── generic.rs          # Generic fallback adapter
│   ├── generic_tests.rs    # Generic adapter tests
│   ├── rust.rs             # Rust adapter (566 lines) <- REFACTOR
│   └── rust_tests.rs       # Rust adapter tests (710 lines)
```

Target structure after refactor:

```
quench/
├── crates/cli/src/adapter/
│   ├── mod.rs              # Adapter trait, registry, detection (unchanged)
│   ├── generic.rs          # Generic fallback adapter (unchanged)
│   ├── generic_tests.rs    # (unchanged)
│   ├── rust/
│   │   ├── mod.rs          # RustAdapter core, default patterns (~150 lines)
│   │   ├── cfg_test.rs     # CfgTestInfo parser (~100 lines)
│   │   ├── workspace.rs    # CargoWorkspace parser (~100 lines)
│   │   ├── suppress.rs     # SuppressAttr parser (~130 lines)
│   │   └── policy.rs       # PolicyCheckResult, lint policy (~60 lines)
│   ├── rust_tests.rs       # Tests for rust/mod.rs
│   └── rust/
│       ├── cfg_test_tests.rs
│       ├── workspace_tests.rs
│       ├── suppress_tests.rs
│       └── policy_tests.rs
```

## Dependencies

No new dependencies required. Uses existing:
- `globset` for pattern matching
- `toml` for Cargo.toml parsing

## Implementation Phases

### Phase 1: Extract cfg_test Module

**Goal:** Move `CfgTestInfo` and related functions to a dedicated module.

**Extract from rust.rs (lines 131-206):**
```rust
// crates/cli/src/adapter/rust/cfg_test.rs

use std::ops::Range;

/// Result of parsing a Rust file for #[cfg(test)] blocks.
#[derive(Debug, Default)]
pub struct CfgTestInfo {
    pub test_ranges: Vec<Range<usize>>,
}

impl CfgTestInfo {
    pub fn parse(content: &str) -> Self { /* ... */ }
    pub fn is_test_line(&self, line_idx: usize) -> bool { /* ... */ }
}

fn is_cfg_test_attr(line: &str) -> bool { /* ... */ }
```

**Also improve string literal handling:**

The current parser counts braces in string literals, which can cause incorrect block boundaries. Add a simple string-skip heuristic:

```rust
impl CfgTestInfo {
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        let mut in_cfg_test = false;
        let mut brace_depth: i32 = 0;
        let mut block_start = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            if !in_cfg_test && is_cfg_test_attr(trimmed) {
                in_cfg_test = true;
                block_start = line_idx;
                brace_depth = 0;
                continue;
            }

            if in_cfg_test {
                // Count braces outside of string literals
                let mut in_string = false;
                let mut prev_char = '\0';

                for ch in trimmed.chars() {
                    if ch == '"' && prev_char != '\\' {
                        in_string = !in_string;
                    } else if !in_string {
                        match ch {
                            '{' => brace_depth += 1,
                            '}' => {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    info.test_ranges.push(block_start..line_idx + 1);
                                    in_cfg_test = false;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    prev_char = ch;
                }
            }
        }

        info
    }
}
```

**Move tests:** Extract cfg_test-related tests from rust_tests.rs to `rust/cfg_test_tests.rs`.

**Milestone:** CfgTestInfo module compiles, existing tests pass, string literal edge case handled.

**Status:** [ ] Pending

---

### Phase 2: Extract workspace Module

**Goal:** Move `CargoWorkspace` and related functions to a dedicated module.

**Extract from rust.rs (lines 319-437):**
```rust
// crates/cli/src/adapter/rust/workspace.rs

use std::fs;
use std::path::Path;
use toml::Value;

/// Cargo workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct CargoWorkspace {
    pub is_workspace: bool,
    pub packages: Vec<String>,
    pub member_patterns: Vec<String>,
}

impl CargoWorkspace {
    pub fn from_root(root: &Path) -> Self { /* ... */ }
    fn from_toml(value: &Value, root: &Path) -> Self { /* ... */ }
}

fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> { /* ... */ }
fn read_package_name(dir: &Path) -> Option<String> { /* ... */ }
```

**Move tests:** Extract workspace-related tests from rust_tests.rs to `rust/workspace_tests.rs`.

**Milestone:** CargoWorkspace module compiles, existing tests pass.

**Status:** [ ] Pending

---

### Phase 3: Extract suppress Module

**Goal:** Move `SuppressAttr` and parsing functions to a dedicated module.

**Extract from rust.rs (lines 439-562):**
```rust
// crates/cli/src/adapter/rust/suppress.rs

/// Suppress attribute found in source code.
#[derive(Debug, Clone)]
pub struct SuppressAttr {
    pub line: usize,
    pub kind: &'static str,
    pub codes: Vec<String>,
    pub has_comment: bool,
    pub comment_text: Option<String>,
}

pub fn parse_suppress_attrs(content: &str, comment_pattern: Option<&str>) -> Vec<SuppressAttr> {
    /* ... */
}

struct ParsedAttr { /* ... */ }
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> { /* ... */ }
fn check_justification_comment(...) -> (bool, Option<String>) { /* ... */ }
```

**Move tests:** Extract suppress-related tests from rust_tests.rs to `rust/suppress_tests.rs`.

**Milestone:** SuppressAttr module compiles, existing tests pass.

**Status:** [ ] Pending

---

### Phase 4: Extract policy Module

**Goal:** Move `PolicyCheckResult` and policy checking to a dedicated module.

**Extract from rust.rs (lines 260-317):**
```rust
// crates/cli/src/adapter/rust/policy.rs

use std::path::Path;
use crate::config::{LintChangesPolicy, RustPolicyConfig};
use super::FileKind;

/// Result of checking lint policy.
#[derive(Debug, Default)]
pub struct PolicyCheckResult {
    pub changed_lint_config: Vec<String>,
    pub changed_source: Vec<String>,
    pub standalone_violated: bool,
}

pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &RustPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    /* ... */
}
```

**Note:** The policy check currently lives as a method on `RustAdapter`. Extract it as a free function that takes a classifier closure, making it more testable and reusable.

**Move tests:** Extract policy-related tests from rust_tests.rs to `rust/policy_tests.rs`.

**Milestone:** PolicyCheckResult module compiles, existing tests pass.

**Status:** [ ] Pending

---

### Phase 5: Consolidate rust Module

**Goal:** Create the `rust/mod.rs` that ties submodules together and contains only the core adapter.

**New rust/mod.rs (~150 lines):**
```rust
//! Rust language adapter.

use std::path::Path;
use globset::{Glob, GlobSet, GlobSetBuilder};

mod cfg_test;
mod policy;
mod suppress;
mod workspace;

pub use cfg_test::CfgTestInfo;
pub use policy::{check_lint_policy, PolicyCheckResult};
pub use suppress::{parse_suppress_attrs, SuppressAttr};
pub use workspace::CargoWorkspace;

use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Default escape patterns for Rust.
const RUST_ESCAPE_PATTERNS: &[EscapePattern] = &[
    /* unchanged */
];

/// Rust language adapter.
pub struct RustAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RustAdapter {
    pub fn new() -> Self { /* unchanged */ }
    pub fn should_ignore(&self, path: &Path) -> bool { /* unchanged */ }

    /// Parse a file and return line-level classification.
    pub fn classify_lines(&self, path: &Path, content: &str) -> LineClassification {
        /* uses CfgTestInfo internally */
    }

    /// Check lint policy against changed files.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &crate::config::RustPolicyConfig,
    ) -> PolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}

impl Adapter for RustAdapter { /* unchanged */ }

/// Result of classifying lines within a single file.
#[derive(Debug, Default)]
pub struct LineClassification {
    pub source_lines: usize,
    pub test_lines: usize,
}

fn build_glob_set(patterns: &[String]) -> GlobSet { /* unchanged */ }
```

**Update adapter/mod.rs re-exports:**
```rust
pub mod rust;
pub use rust::{CfgTestInfo, PolicyCheckResult, RustAdapter, parse_suppress_attrs};
```

**Milestone:** All modules compile, all exports work, all tests pass.

**Status:** [ ] Pending

---

### Phase 6: Run Full Test Suite

Execute `make check` to ensure all quality gates pass.

```bash
make check
```

**Checklist:**
- [ ] `cargo fmt --all -- --check` - no formatting issues
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - no warnings
- [ ] `cargo test --all` - all tests pass
- [ ] `cargo test rust_adapter` - all 19 behavioral specs pass
- [ ] `cargo build --all` - builds successfully
- [ ] `./scripts/bootstrap` - conventions pass
- [ ] `cargo audit` - no vulnerabilities
- [ ] `cargo deny check` - licenses/bans OK

**Milestone:** All quality gates pass with new module structure.

**Status:** [ ] Pending

## Key Implementation Details

### Module Visibility

All submodules use `pub(crate)` for internal types and `pub` for types that need to be exported from the adapter module:

```rust
// rust/cfg_test.rs
pub struct CfgTestInfo { ... }  // Public - used by cloc check
pub(crate) fn is_cfg_test_attr(line: &str) -> bool { ... }  // Internal
```

### Test File Organization

Tests follow the sibling `_tests.rs` convention. Each submodule gets its own test file:

```
rust/
├── mod.rs
├── cfg_test.rs
├── cfg_test_tests.rs    # Tests for cfg_test.rs
├── workspace.rs
├── workspace_tests.rs   # Tests for workspace.rs
└── ...
```

Each test file uses the pattern:
```rust
// cfg_test_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn basic_cfg_test_block() { /* ... */ }
```

### String Literal Handling Improvement

The improved `CfgTestInfo::parse()` handles string literals by tracking quote state:

| Input | Before | After |
|-------|--------|-------|
| `let s = "{ not a brace }";` | Counts as `{` and `}` | Skipped |
| `let s = "\"escaped\"";` | Incorrect tracking | Handles escapes |
| `r#"raw { string"#` | May miscount | Still limited (acceptable for v1) |

**Limitation:** Raw strings (`r#"..."#`) are not fully handled. This is acceptable as they're rare in test modules.

### Re-export Strategy

The `adapter/mod.rs` re-exports commonly used types for ergonomic imports:

```rust
// Users can import either way:
use quench::adapter::rust::CfgTestInfo;  // Explicit
use quench::adapter::CfgTestInfo;        // Re-exported convenience
```

## Verification Plan

### Unit Test Migration

For each extracted module, verify tests still pass:

```bash
# After Phase 1
cargo test --package quench -- cfg_test

# After Phase 2
cargo test --package quench -- workspace

# After Phase 3
cargo test --package quench -- suppress

# After Phase 4
cargo test --package quench -- policy
```

### Behavioral Spec Verification

All 19 Rust adapter specs must continue passing:

```bash
cargo test --test specs rust_adapter
```

### Integration Verification

Verify the refactor doesn't change observable behavior:

```bash
# Before refactor - capture outputs
./target/release/quench check tests/fixtures/rust-simple -o json > /tmp/before-simple.json
./target/release/quench check tests/fixtures/rust-workspace -o json > /tmp/before-workspace.json
./target/release/quench check tests/fixtures/rust/cfg-test -o json > /tmp/before-cfgtest.json

# After refactor - compare
./target/release/quench check tests/fixtures/rust-simple -o json | diff - /tmp/before-simple.json
./target/release/quench check tests/fixtures/rust-workspace -o json | diff - /tmp/before-workspace.json
./target/release/quench check tests/fixtures/rust/cfg-test -o json | diff - /tmp/before-cfgtest.json
```

### Full Quality Gates

```bash
make check
```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Extract cfg_test module | [ ] Pending |
| 2 | Extract workspace module | [ ] Pending |
| 3 | Extract suppress module | [ ] Pending |
| 4 | Extract policy module | [ ] Pending |
| 5 | Consolidate rust module | [ ] Pending |
| 6 | Run full test suite | [ ] Pending |
