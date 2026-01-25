# Checkpoint 10H: Tech Debt - Dogfooding Milestone 2

**Root Feature:** `quench-10h`
**Follows:** checkpoint-10g-bugfix (Bug Fixes)

## Overview

This checkpoint addresses technical debt accumulated during Dogfooding Milestone 2. With 531 passing tests and no clippy warnings, the codebase is stable but has accumulated patterns that should be cleaned up before Milestone 3 (CI integration). Focus areas:

1. **Pattern Resolution Consolidation** - DRY up nearly-identical `resolve_*_patterns` functions
2. **Large File Refactoring** - Split oversized modules for better maintainability
3. **Config Defaults Trait** - Centralize scattered `default_*` functions
4. **Test Helper Consolidation** - Reduce duplication between unit and spec test utilities

These refactors maintain identical behavior while improving code organization and maintainability.

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── mod.rs                      # MODIFY: Extract pattern resolution trait
│   └── patterns.rs                 # CREATE: Generic pattern resolution
├── config/
│   ├── mod.rs                      # MODIFY: Centralize defaults
│   ├── defaults.rs                 # CREATE: DefaultPatterns trait
│   ├── checks.rs                   # MODIFY: Extract cloc into submodule
│   └── cloc.rs                     # CREATE: ClocConfig separated
├── checks/
│   └── escapes/
│       ├── mod.rs                  # MODIFY: Split into submodules
│       ├── patterns.rs             # CREATE: Pattern matching logic
│       └── violations.rs           # CREATE: Violation handling
└── test_utils.rs                   # MODIFY: Add shared helpers

tests/specs/
└── helpers/                        # CREATE: Shared spec test utilities
    └── mod.rs
```

## Dependencies

No new external dependencies. Uses existing crates only.

## Implementation Phases

### Phase 1: Pattern Resolution Consolidation

**Goal**: Extract duplicated `resolve_*_patterns` functions into a generic implementation.

**Problem**: Four nearly-identical functions exist in `adapter/mod.rs`:
- `resolve_rust_patterns` (lines 299-333)
- `resolve_go_patterns` (lines 336-364)
- `resolve_javascript_patterns` (lines 367-401)
- `resolve_shell_patterns` (lines 404-431)

**Solution**: Create a trait for language config with default patterns.

**File:** `crates/cli/src/adapter/patterns.rs`

```rust
/// Trait for language configurations that provide default patterns.
pub trait LanguageDefaults {
    /// Default source file patterns for this language.
    fn default_source() -> Vec<String>;

    /// Default test file patterns for this language.
    fn default_tests() -> Vec<String>;

    /// Default ignore patterns for this language.
    fn default_ignore() -> Vec<String> {
        vec![]
    }
}

/// Resolved patterns for an adapter.
pub struct ResolvedPatterns {
    pub source: Vec<String>,
    pub test: Vec<String>,
    pub ignore: Vec<String>,
}

/// Generic pattern resolution.
///
/// Resolution hierarchy:
/// 1. Language-specific config (most specific)
/// 2. Project-wide fallback
/// 3. Language defaults (zero-config)
pub fn resolve_patterns<C: LanguageDefaults>(
    lang_source: &[String],
    lang_tests: &[String],
    lang_ignore: &[String],
    fallback_test: &[String],
) -> ResolvedPatterns {
    let test = if !lang_tests.is_empty() {
        lang_tests.to_vec()
    } else if !fallback_test.is_empty() {
        fallback_test.to_vec()
    } else {
        C::default_tests()
    };

    let source = if !lang_source.is_empty() {
        lang_source.to_vec()
    } else {
        C::default_source()
    };

    let ignore = if !lang_ignore.is_empty() {
        lang_ignore.to_vec()
    } else {
        C::default_ignore()
    };

    ResolvedPatterns { source, test, ignore }
}
```

**Modify:** `crates/cli/src/config/mod.rs` - Implement trait for each language config.

**Tests:**
```rust
#[test]
fn resolve_patterns_uses_lang_config_first() {
    let patterns = resolve_patterns::<RustConfig>(
        &["custom/**".to_string()],  // lang source
        &["test/**".to_string()],     // lang tests
        &[],                           // lang ignore
        &[],                           // fallback
    );
    assert_eq!(patterns.source, vec!["custom/**"]);
    assert_eq!(patterns.test, vec!["test/**"]);
}

#[test]
fn resolve_patterns_falls_back_to_project_then_defaults() {
    let patterns = resolve_patterns::<RustConfig>(
        &[],                           // no lang source
        &[],                           // no lang tests
        &[],                           // no lang ignore
        &["fallback/**".to_string()], // fallback
    );
    // Source uses default, test uses fallback
    assert_eq!(patterns.source, RustConfig::default_source());
    assert_eq!(patterns.test, vec!["fallback/**"]);
}
```

**Verification:**
```bash
cargo test adapter::patterns
cargo test config::mod_tests
```

---

### Phase 2: Escapes Module Split

**Goal**: Split the 742-line `checks/escapes/mod.rs` into focused submodules.

**Problem**: `escapes/mod.rs` handles too many concerns:
- Pattern matching and compilation
- Comment detection
- Violation generation
- Multi-language dispatch

**Solution**: Extract into focused submodules:

**Create:** `crates/cli/src/checks/escapes/detect.rs`
```rust
//! Pattern detection for escape hatches.
//!
//! Finds pattern matches in source code.

use crate::pattern::CompiledPattern;

pub struct Detection {
    pub line: u32,
    pub column: u32,
    pub matched_text: String,
}

/// Detect all matches of a pattern in content.
pub fn detect_pattern(pattern: &CompiledPattern, content: &str) -> Vec<Detection> {
    // ... extracted from mod.rs
}
```

**Create:** `crates/cli/src/checks/escapes/violations.rs`
```rust
//! Violation generation for escape hatches.
//!
//! Converts detections into violations with proper advice.

use crate::check::Violation;

/// Generate a violation for an escape hatch detection.
pub fn make_violation(
    file: &Path,
    line: u32,
    pattern_name: &str,
    action: EscapeAction,
    advice: &str,
) -> Violation {
    // ... extracted from mod.rs
}
```

**Files to update:**
- `checks/escapes/mod.rs` - Reduce to ~300 lines of orchestration
- `checks/escapes/detect.rs` - Pattern detection (~150 lines)
- `checks/escapes/violations.rs` - Violation creation (~150 lines)
- `checks/escapes/comment.rs` - Already exists, keep as-is

**Verification:**
```bash
cargo test checks::escapes
cargo test --test specs -- escapes
```

---

### Phase 3: Config Defaults Centralization

**Goal**: Consolidate scattered `default_*` functions into a centralized module.

**Problem**: 30+ `default_*` functions spread across config modules make it hard to:
- Find all defaults
- Ensure consistency
- Document defaults in one place

**Solution**: Create a `defaults.rs` module with a trait-based approach.

**Create:** `crates/cli/src/config/defaults.rs`

```rust
//! Centralized default values for configuration.
//!
//! All default values are documented here for easy reference.

/// Default file size limits.
pub mod size {
    /// Default max lines for source files (750).
    pub const MAX_LINES: usize = 750;
    /// Default max lines for test files (1500).
    pub const MAX_LINES_TEST: usize = 1500;
    /// Default max tokens (20000, ~5k words).
    pub const MAX_TOKENS: usize = 20000;
}

/// Default glob patterns.
pub mod patterns {
    /// Rust source patterns.
    pub fn rust_source() -> Vec<String> {
        vec![
            "src/**/*.rs".to_string(),
            "crates/**/*.rs".to_string(),
        ]
    }

    /// Rust test patterns.
    pub fn rust_tests() -> Vec<String> {
        vec![
            "tests/**/*.rs".to_string(),
            "**/*_test.rs".to_string(),
            "**/*_tests.rs".to_string(),
        ]
    }

    // ... similar for Go, JavaScript, Shell
}

/// Default advice messages.
pub mod advice {
    pub const CLOC_SOURCE: &str = "Split into smaller modules for LLM context.";
    pub const CLOC_TEST: &str = "Use table-driven tests or split test file.";
}
```

**Modify configs to use centralized defaults:**

```rust
// Before:
impl LangClocConfig {
    pub(super) fn default_max_lines() -> usize { 750 }
}

// After:
impl LangClocConfig {
    pub(super) fn default_max_lines() -> usize {
        defaults::size::MAX_LINES
    }
}
```

**Verification:**
```bash
cargo test config::defaults
cargo test config::mod_tests
```

---

### Phase 4: Large Test File Splitting

**Goal**: Split oversized test files for better organization.

**Files to split:**
| File | Lines | Action |
|------|-------|--------|
| `config/mod_tests.rs` | 1032 | Split by config section |
| `checks/docs/toc_tests.rs` | 916 | Split by TOC feature |
| `checks/tests/correlation_tests.rs` | 816 | Split by language |

**Split `config/mod_tests.rs`:**

```
config/
├── mod_tests.rs           # ~200 lines (parse, version, basic)
├── checks_tests.rs        # ~300 lines (cloc, escapes, agents config)
├── lang_tests.rs          # ~300 lines (rust, go, javascript, shell config)
└── suppress_tests.rs      # ~200 lines (suppress config tests)
```

**Pattern for referencing:**
```rust
// config/mod.rs
#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "checks_tests.rs"]
mod checks_tests;

#[cfg(test)]
#[path = "lang_tests.rs"]
mod lang_tests;

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod suppress_tests;
```

**Verification:**
```bash
cargo test config::
cargo test checks::docs::toc_tests
cargo test checks::tests::correlation
```

---

### Phase 5: Test Utilities Consolidation

**Goal**: Create shared test utilities to reduce duplication between unit and spec tests.

**Problem**: `test_utils.rs` and `tests/specs/prelude.rs` have overlapping functionality.

**Solution**: Create a shared testing crate or consolidate utilities.

**Enhance:** `crates/cli/src/test_utils.rs`

```rust
/// Git repository test helpers.
pub mod git {
    use std::path::Path;
    use std::process::Command;

    /// Initialize a git repo with test configuration.
    pub fn init(path: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("git init should succeed");

        // Set test identity
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .expect("git config email should succeed");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .expect("git config name should succeed");
    }

    /// Create an initial commit.
    pub fn commit(path: &Path, message: &str) {
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .expect("git add should succeed");

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .expect("git commit should succeed");
    }
}
```

**Update `tests/specs/prelude.rs`:**
```rust
// Re-export from library for consistency
pub use quench::test_utils::git::{init as git_init, commit as git_commit};

// Spec-specific git helpers that wrap the library functions
pub fn git_initial_commit(project: &Project) {
    quench::test_utils::git::commit(project.path(), "feat: initial commit");
}
```

**Verification:**
```bash
cargo test --lib test_utils
cargo test --test specs
```

---

### Phase 6: Documentation and Cleanup

**Goal**: Document changes and ensure all tests pass.

**Tasks:**

1. **Update module docs** - Document the new module structure
2. **Clean up `#[allow]` pragmas** - Review and justify or remove
3. **Run full verification** - `make check`
4. **Update CLAUDE.md** - Note any new patterns

**`#[allow]` review:**
- `#[allow(clippy::expect_used)]` in `checks/git/parse.rs:24` - Justified (regex compilation)
- `#[allow(dead_code)]` in `checks/git/docs.rs:35` - Remove if unused
- `#[allow(clippy::enum_variant_names)]` in `adapter/javascript/suppress.rs:32` - Review

**Verification:**
```bash
# Full test suite
make check

# Specific modules
cargo test adapter::
cargo test config::
cargo test checks::escapes

# Dogfooding
cargo run --release -- check --timing
```

---

## Key Implementation Details

### Pattern Resolution Trait Design

The trait-based approach provides:
- **Single source of truth** for pattern resolution logic
- **Type safety** - Language configs implement `LanguageDefaults`
- **Testability** - Easy to test with mock configs

### Module Split Strategy

When splitting large modules:
1. Keep public API in `mod.rs`
2. Extract implementation details to submodules
3. Re-export types needed by other modules
4. Minimize import changes in callers

### Test File Organization

Test file naming convention:
- `foo_tests.rs` - Unit tests for `foo.rs`
- `foo/bar_tests.rs` - Unit tests for `foo/bar.rs`
- Tests organized by feature, not by test type

---

## Verification Plan

### Per-Phase Verification

| Phase | Command | Expected |
|-------|---------|----------|
| 1 | `cargo test adapter::patterns` | Pattern resolution tests pass |
| 2 | `cargo test checks::escapes` | Escapes behavior unchanged |
| 3 | `cargo test config::defaults` | Default values documented |
| 4 | `cargo test config::` | All config tests pass |
| 5 | `cargo test --test specs` | Spec tests pass |
| 6 | `make check` | Full suite passes |

### Final Verification

```bash
# Full test suite
make check

# Dogfooding
cargo run --release -- check --timing

# Verify no clippy warnings
cargo clippy --all-targets --all-features -- -D warnings

# Line count check (ensure large files are smaller)
wc -l crates/cli/src/checks/escapes/mod.rs    # Should be < 400
wc -l crates/cli/src/config/mod_tests.rs      # Should be < 300
```

### Success Criteria

1. **All tests pass**: `cargo test --all` exits 0
2. **No new warnings**: `cargo clippy` clean
3. **Files reduced**: Target files under 500 lines
4. **Dogfooding passes**: `quench check` on quench reports 0 violations
5. **Code coverage maintained**: No reduction in test coverage

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1. Pattern Resolution | Low | Extensive test coverage, no behavior change |
| 2. Escapes Split | Low | Re-export maintains API, test behavior |
| 3. Config Defaults | Very Low | Constants only, no logic change |
| 4. Test Splitting | Very Low | Tests remain, just reorganized |
| 5. Test Utils | Low | Re-export pattern maintains compatibility |
| 6. Cleanup | Very Low | Documentation only |

---

## Summary

| Phase | Deliverable | Purpose |
|-------|-------------|---------|
| 1 | `adapter/patterns.rs` | DRY pattern resolution |
| 2 | Split escapes module | Better organization |
| 3 | `config/defaults.rs` | Centralized defaults |
| 4 | Split test files | Improved maintainability |
| 5 | Enhanced test utils | Reduced duplication |
| 6 | Documentation | Clean handoff |

---

## Completion Criteria

- [ ] Phase 1: Pattern resolution trait implemented
- [ ] Phase 2: Escapes module split complete
- [ ] Phase 3: Config defaults centralized
- [ ] Phase 4: Large test files split
- [ ] Phase 5: Test utilities consolidated
- [ ] Phase 6: `make check` passes
- [ ] `./done` executed successfully
