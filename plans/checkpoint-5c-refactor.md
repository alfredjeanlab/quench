# Checkpoint 5C: Shell Adapter - Refactor

**Root Feature:** `quench-a8e2`

## Overview

Refactoring checkpoint to reduce code duplication between Rust and Shell adapters. Validation in checkpoint 5B confirmed both adapters work correctly; this checkpoint extracts shared patterns to improve maintainability and simplify future adapter additions.

**Current State**: Shell adapter complete with 75 tests passing. Significant code duplication exists between `rust/` and `shell/` adapter modules.

**End State**: Common utilities extracted, duplication reduced by ~100 LOC, all 75 tests still pass, `make check` succeeds.

## Project Structure

Files to create/modify:

```
crates/cli/src/adapter/
├── mod.rs                     # Re-export common utilities
├── common/
│   ├── mod.rs                 # Common module
│   ├── suppress.rs            # Extracted: check_justification_comment()
│   └── policy.rs              # Extracted: PolicyCheckResult, check_lint_policy()
├── rust/
│   ├── suppress.rs            # Use common::suppress
│   └── policy.rs              # Use common::policy
└── shell/
    ├── suppress.rs            # Use common::suppress
    └── policy.rs              # Use common::policy
```

## Dependencies

No new dependencies.

## Implementation Phases

### Phase 5C.1: Extract Common Suppress Utilities

**Goal**: Extract `check_justification_comment()` into shared module.

**Current duplication** (`rust/suppress.rs:85-127` vs `shell/suppress.rs:76-118`):
```rust
// Both files have nearly identical implementations:
fn check_justification_comment(
    lines: &[&str],
    directive_line: usize,
    required_pattern: Option<&str>,
) -> (bool, Option<String>) {
    // Walk backward looking for comment...
}
```

**Differences between implementations**:
| Aspect | Rust | Shell |
|--------|------|-------|
| Comment prefix | `//` | `#` |
| Exclude pattern | `#[` (attributes) | `shellcheck` |
| Stop condition | Non-attribute, non-comment | Non-comment |

**Solution**: Parameterized common function:

**Create `adapter/common/mod.rs`:**
```rust
pub mod suppress;
pub mod policy;
```

**Create `adapter/common/suppress.rs`:**
```rust
//! Common suppress directive utilities.

/// Comment style configuration for different languages.
pub struct CommentStyle {
    /// Comment line prefix (e.g., "//" for Rust, "#" for Shell).
    pub prefix: &'static str,
    /// Patterns that indicate a directive line, not a justification comment.
    pub directive_patterns: &'static [&'static str],
}

impl CommentStyle {
    pub const RUST: Self = Self {
        prefix: "//",
        directive_patterns: &["#["],
    };

    pub const SHELL: Self = Self {
        prefix: "#",
        directive_patterns: &["shellcheck"],
    };
}

/// Check if there's a justification comment above a directive line.
pub fn check_justification_comment(
    lines: &[&str],
    directive_line: usize,
    required_pattern: Option<&str>,
    style: &CommentStyle,
) -> (bool, Option<String>) {
    let mut check_line = directive_line;

    while check_line > 0 {
        check_line -= 1;
        let line = lines[check_line].trim();

        // Stop at blank lines
        if line.is_empty() {
            break;
        }

        // Check for comment
        if line.starts_with(style.prefix) {
            // Skip directive lines (not justification comments)
            if style.directive_patterns.iter().any(|p| line.contains(p)) {
                continue;
            }

            let comment_text = line.trim_start_matches(style.prefix).trim();

            // If specific pattern required, check for it
            if let Some(pattern) = required_pattern {
                let pattern_prefix = pattern.trim_start_matches(style.prefix).trim();
                if comment_text.starts_with(pattern_prefix) || line.starts_with(pattern) {
                    return (true, Some(comment_text.to_string()));
                }
                continue;
            }

            // Any non-empty comment counts as justification
            if !comment_text.is_empty() {
                return (true, Some(comment_text.to_string()));
            }
        } else {
            // Stop at non-comment line
            break;
        }
    }

    (false, None)
}
```

**Update `rust/suppress.rs`:**
```rust
use crate::adapter::common::suppress::{check_justification_comment, CommentStyle};

// In parse_suppress_attrs():
let (has_comment, comment_text) =
    check_justification_comment(&lines, line_idx, comment_pattern, &CommentStyle::RUST);
```

**Update `shell/suppress.rs`:**
```rust
use crate::adapter::common::suppress::{check_justification_comment, CommentStyle};

// In parse_shellcheck_suppresses():
let (has_comment, comment_text) =
    check_justification_comment(&lines, line_idx, comment_pattern, &CommentStyle::SHELL);
```

**Milestone**: Shared suppress utility works, both adapter test suites pass.

---

### Phase 5C.2: Extract Common Policy Utilities

**Goal**: Extract `check_lint_policy()` into shared module.

**Current duplication** (`rust/policy.rs` and `shell/policy.rs` are nearly identical):

| Component | Rust | Shell |
|-----------|------|-------|
| Result struct | `PolicyCheckResult` | `ShellPolicyCheckResult` |
| Config type | `RustPolicyConfig` | `ShellPolicyConfig` |
| Function body | Identical | Identical |

**Solution**: Generic policy checker with trait:

**Create `adapter/common/policy.rs`:**
```rust
//! Common lint policy checking utilities.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::LintChangesPolicy;

/// Result of checking lint policy.
#[derive(Debug, Default)]
pub struct PolicyCheckResult {
    /// Lint config files that were changed.
    pub changed_lint_config: Vec<String>,
    /// Source/test files that were changed.
    pub changed_source: Vec<String>,
    /// Whether the standalone policy is violated.
    pub standalone_violated: bool,
}

/// Policy configuration trait for language-specific configs.
pub trait PolicyConfig {
    /// Get the lint changes policy.
    fn lint_changes(&self) -> LintChangesPolicy;
    /// Get the list of lint config file patterns.
    fn lint_config(&self) -> &[String];
}

/// Check lint policy against changed files.
pub fn check_lint_policy<P: PolicyConfig>(
    changed_files: &[&Path],
    policy: &P,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    if policy.lint_changes() == LintChangesPolicy::None {
        return PolicyCheckResult::default();
    }

    let mut result = PolicyCheckResult::default();

    for file in changed_files {
        let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if it's a lint config file
        if policy
            .lint_config()
            .iter()
            .any(|cfg| filename == cfg || file.to_string_lossy().ends_with(cfg))
        {
            result.changed_lint_config.push(file.display().to_string());
            continue;
        }

        // Check if it's a source or test file
        let kind = classify(file);
        if kind == FileKind::Source || kind == FileKind::Test {
            result.changed_source.push(file.display().to_string());
        }
    }

    // Standalone policy violated if both lint config AND source changed
    result.standalone_violated = policy.lint_changes() == LintChangesPolicy::Standalone
        && !result.changed_lint_config.is_empty()
        && !result.changed_source.is_empty();

    result
}
```

**Implement trait for config types in `config/rust.rs`:**
```rust
impl crate::adapter::common::policy::PolicyConfig for RustPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }
    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
```

**Implement trait for config types in `config/shell.rs`:**
```rust
impl crate::adapter::common::policy::PolicyConfig for ShellPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }
    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
```

**Update `rust/policy.rs`:**
```rust
pub use crate::adapter::common::policy::{check_lint_policy, PolicyCheckResult};
```

**Update `shell/policy.rs`:**
```rust
pub use crate::adapter::common::policy::{check_lint_policy, PolicyCheckResult};
// Remove ShellPolicyCheckResult, use PolicyCheckResult
```

**Milestone**: Single policy implementation, both test suites pass.

---

### Phase 5C.3: Update Module Exports

**Goal**: Update `adapter/mod.rs` to export common utilities.

**Update `adapter/mod.rs`:**
```rust
pub mod common;
pub mod generic;
pub mod glob;
pub mod rust;
pub mod shell;

pub use common::policy::PolicyCheckResult;
pub use common::suppress::CommentStyle;
// ... existing exports
```

**Milestone**: Public API unchanged, imports simplified.

---

### Phase 5C.4: Run Tests and Quality Gates

**Goal**: Verify refactoring preserves all behavior.

```bash
# Run all adapter tests
cargo test adapter

# Run shell-specific tests
cargo test shell

# Run rust-specific tests
cargo test rust

# Full quality gates
make check
```

**Expected**: All 75 tests pass, no behavior changes.

**Milestone**: `make check` passes.

---

### Phase 5C.5: Measure and Document Impact

**Goal**: Document LOC reduction and maintainability improvements.

**Before (estimated)**:
- `rust/suppress.rs`: 132 LOC
- `shell/suppress.rs`: 122 LOC
- `rust/policy.rs`: 68 LOC
- `shell/policy.rs`: 68 LOC
- **Total**: ~390 LOC

**After (estimated)**:
- `common/suppress.rs`: ~60 LOC
- `common/policy.rs`: ~50 LOC
- `rust/suppress.rs`: ~90 LOC (removed ~40 LOC)
- `shell/suppress.rs`: ~80 LOC (removed ~40 LOC)
- `rust/policy.rs`: ~10 LOC (re-export + trait impl)
- `shell/policy.rs`: ~10 LOC (re-export + trait impl)
- **Total**: ~300 LOC

**Net reduction**: ~90 LOC (~23% reduction in affected files)

**Maintainability improvements**:
1. Single source of truth for justification comment logic
2. Single source of truth for lint policy checking
3. Adding new adapters only requires trait implementation
4. Bug fixes in common logic apply to all adapters

**Milestone**: Documentation complete.

---

## Key Implementation Details

### Backward Compatibility

Public API remains unchanged:
- `parse_suppress_attrs()` signature unchanged
- `parse_shellcheck_suppresses()` signature unchanged
- `check_lint_policy()` functions work via trait dispatch

### Comment Style Extensibility

The `CommentStyle` struct supports future adapters:

```rust
// Future Python adapter
impl CommentStyle {
    pub const PYTHON: Self = Self {
        prefix: "#",
        directive_patterns: &["noqa", "type: ignore"],
    };
}
```

### Trait-Based Policy

The `PolicyConfig` trait allows type-safe policy checking without code duplication:

```rust
// Works with any config type implementing PolicyConfig
check_lint_policy(&files, &rust_policy, |p| adapter.classify(p));
check_lint_policy(&files, &shell_policy, |p| adapter.classify(p));
```

### Test Strategy

Existing tests remain in their adapter-specific locations:
- `rust/suppress_tests.rs` - Tests Rust-specific parsing
- `shell/suppress_tests.rs` - Tests Shell-specific parsing
- `rust/policy_tests.rs` - Tests with RustPolicyConfig
- `shell/policy_tests.rs` - Tests with ShellPolicyConfig

New tests for common utilities (optional):
- `common/suppress_tests.rs` - Tests parameterized function directly

## Verification Plan

### Automated Verification

```bash
# Verify no behavior changes
cargo test --all

# Verify code quality
make check
```

### Success Criteria

- [ ] `common/suppress.rs` created with `check_justification_comment()`
- [ ] `common/policy.rs` created with `PolicyConfig` trait and `check_lint_policy()`
- [ ] `rust/suppress.rs` uses common suppress utility
- [ ] `shell/suppress.rs` uses common suppress utility
- [ ] `rust/policy.rs` uses common policy via trait
- [ ] `shell/policy.rs` uses common policy via trait
- [ ] All 75 tests pass
- [ ] `make check` passes
- [ ] Net LOC reduction achieved
