# Phase 315: Rust Adapter - Escapes

**Root Feature:** `quench-a0ea`

## Overview

Implement default escape patterns for the Rust adapter. When a Rust project is detected (via `Cargo.toml`), the escapes check will automatically apply Rust-specific patterns:

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `unsafe { }` | comment | `// SAFETY:` |
| `.unwrap()` | forbid | - |
| `.expect(` | forbid | - |
| `mem::transmute` | comment | `// SAFETY:` |

These patterns follow the specification in `docs/specs/langs/rust.md` and `docs/specs/checks/escape-hatches.md`. The key behaviors:

- **unsafe** and **transmute**: Require a `// SAFETY:` comment explaining invariants
- **unwrap** and **expect**: Forbidden in source code, automatically allowed in test code
- All patterns respect `#[cfg(test)]` block detection from Phase 310

Reference docs:
- `docs/specs/langs/rust.md` (Default Escape Patterns section)
- `docs/specs/checks/escape-hatches.md` (Actions section)

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── mod.rs             # UPDATE: EscapePattern with advice field
│   │   ├── rust.rs            # UPDATE: implement default_escapes()
│   │   └── rust_tests.rs      # UPDATE: add escape pattern tests
│   └── checks/
│       ├── escapes.rs         # UPDATE: merge adapter defaults with config
│       └── escapes_tests.rs   # UPDATE: add Rust-specific tests
├── tests/
│   ├── specs/
│   │   └── adapters/rust.rs   # UPDATE: remove #[ignore] from escape specs
│   └── fixtures/
│       └── rust/
│           ├── unsafe-fail/   # NEW: fixture for unsafe without comment
│           ├── unsafe-ok/     # NEW: fixture for unsafe with SAFETY comment
│           ├── unwrap-source/ # NEW: fixture for unwrap in source code
│           └── unwrap-test/   # NEW: fixture for unwrap in test code only
└── plans/
    └── phase-315.md
```

## Dependencies

No new external dependencies. Uses existing:
- `crate::adapter` module for `EscapePattern`, `EscapeAction`
- `crate::checks::escapes` for pattern matching infrastructure
- Phase 310's `CfgTestInfo` for `#[cfg(test)]` block detection

## Implementation Phases

### Phase 1: Extend EscapePattern with Advice

Add an `advice` field to the adapter's `EscapePattern` type so each default pattern can provide actionable guidance.

**Update `crates/cli/src/adapter/mod.rs`:**

```rust
/// An escape pattern with its action.
#[derive(Debug, Clone)]
pub struct EscapePattern {
    /// Pattern name for reporting (e.g., "unsafe", "unwrap").
    pub name: &'static str,
    /// Regex pattern to match.
    pub pattern: &'static str,
    /// Required action for this escape.
    pub action: EscapeAction,
    /// Required comment pattern (for Comment action).
    pub comment: Option<&'static str>,
    /// Advice to show when pattern is violated.
    pub advice: &'static str,
}
```

**Milestone:** `EscapePattern` struct has an `advice` field.

**Verification:**
```bash
cargo build
```

---

### Phase 2: Implement default_escapes() for RustAdapter

Add the four default Rust escape patterns to `RustAdapter`.

**Update `crates/cli/src/adapter/rust.rs`:**

```rust
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

// ... existing code ...

/// Default escape patterns for Rust.
const RUST_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe",
        pattern: r"unsafe\s*\{",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining the invariants.",
    },
    EscapePattern {
        name: "unwrap",
        pattern: r"\.unwrap\(\)",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Use ? operator or handle the error explicitly.",
    },
    EscapePattern {
        name: "expect",
        pattern: r"\.expect\(",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Use ? operator or handle the error explicitly.",
    },
    EscapePattern {
        name: "transmute",
        pattern: r"mem::transmute",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining type compatibility.",
    },
];

impl Adapter for RustAdapter {
    // ... existing methods ...

    fn default_escapes(&self) -> &'static [EscapePattern] {
        RUST_ESCAPE_PATTERNS
    }
}
```

**Milestone:** `RustAdapter::default_escapes()` returns the four Rust patterns.

**Verification:**
```bash
cargo test adapter::rust
```

---

### Phase 3: Unit Tests for Default Escapes

Add tests verifying the adapter returns correct patterns.

**Add to `crates/cli/src/adapter/rust_tests.rs`:**

```rust
mod default_escapes {
    use super::*;
    use crate::adapter::{Adapter, EscapeAction};

    #[test]
    fn returns_four_default_patterns() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        assert_eq!(patterns.len(), 4);
    }

    #[test]
    fn unsafe_pattern_requires_safety_comment() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let unsafe_pattern = patterns.iter().find(|p| p.name == "unsafe").unwrap();

        assert_eq!(unsafe_pattern.action, EscapeAction::Comment);
        assert_eq!(unsafe_pattern.comment, Some("// SAFETY:"));
    }

    #[test]
    fn unwrap_pattern_is_forbidden() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let unwrap_pattern = patterns.iter().find(|p| p.name == "unwrap").unwrap();

        assert_eq!(unwrap_pattern.action, EscapeAction::Forbid);
    }

    #[test]
    fn expect_pattern_is_forbidden() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let expect_pattern = patterns.iter().find(|p| p.name == "expect").unwrap();

        assert_eq!(expect_pattern.action, EscapeAction::Forbid);
    }

    #[test]
    fn transmute_pattern_requires_safety_comment() {
        let adapter = RustAdapter::new();
        let patterns = adapter.default_escapes();
        let transmute_pattern = patterns.iter().find(|p| p.name == "transmute").unwrap();

        assert_eq!(transmute_pattern.action, EscapeAction::Comment);
        assert_eq!(transmute_pattern.comment, Some("// SAFETY:"));
    }
}
```

**Milestone:** All unit tests for default escape patterns pass.

**Verification:**
```bash
cargo test adapter::rust::tests::default_escapes
```

---

### Phase 4: Merge Adapter Defaults in Escapes Check

Update the escapes check to merge adapter default patterns with user-configured patterns. Adapter defaults apply when:
1. A language adapter is detected for the project
2. No explicit patterns are configured in `quench.toml`

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use crate::adapter::{AdapterRegistry, EscapePattern as AdapterEscapePattern};

// In EscapesCheck::run():

fn run(&self, ctx: &CheckContext) -> CheckResult {
    let config = &ctx.config.check.escapes;

    if config.check == CheckLevel::Off {
        return CheckResult::passed(self.name());
    }

    // Get adapter defaults for the project
    let registry = AdapterRegistry::for_project(ctx.root);
    let adapter_patterns = get_adapter_escape_patterns(&registry, ctx.root);

    // Merge patterns: config patterns override adapter defaults by name
    let merged_patterns = merge_patterns(&config.patterns, &adapter_patterns);

    // No patterns configured = nothing to check
    if merged_patterns.is_empty() {
        return CheckResult::passed(self.name());
    }

    // Compile patterns once
    let patterns = match compile_merged_patterns(&merged_patterns) {
        Ok(p) => p,
        Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
    };

    // ... rest of existing logic ...
}

/// Get escape patterns from the adapter for the detected language.
fn get_adapter_escape_patterns(registry: &AdapterRegistry, root: &Path) -> Vec<EscapePatternConfig> {
    // For each detected language, collect its default escapes
    // Currently just Rust; extensible to other languages
    let mut patterns = Vec::new();

    // Check if Rust is detected
    if root.join("Cargo.toml").exists() {
        let rust_adapter = RustAdapter::new();
        for p in rust_adapter.default_escapes() {
            patterns.push(EscapePatternConfig {
                name: p.name.to_string(),
                pattern: p.pattern.to_string(),
                action: p.action.into(),
                comment: p.comment.map(String::from),
                advice: Some(p.advice.to_string()),
                threshold: 0,
            });
        }
    }

    patterns
}

/// Merge user config patterns with adapter defaults.
/// User patterns override defaults by name.
fn merge_patterns(
    config_patterns: &[EscapePatternConfig],
    adapter_patterns: &[EscapePatternConfig],
) -> Vec<EscapePatternConfig> {
    let mut merged = Vec::new();
    let config_names: HashSet<_> = config_patterns.iter().map(|p| &p.name).collect();

    // Add adapter defaults not overridden by config
    for pattern in adapter_patterns {
        if !config_names.contains(&pattern.name) {
            merged.push(pattern.clone());
        }
    }

    // Add all config patterns (they take precedence)
    merged.extend(config_patterns.iter().cloned());

    merged
}
```

**Key changes:**
1. Detect project language and get adapter default patterns
2. Merge with user config (config patterns override by name)
3. Use merged patterns for scanning

**Milestone:** Rust projects automatically apply default escape patterns.

**Verification:**
```bash
cargo test checks::escapes
```

---

### Phase 5: Test Fixtures for Escape Patterns

Create fixtures for behavioral specs.

**Create `tests/fixtures/rust/unsafe-fail/`:**
```
Cargo.toml:
[package]
name = "unsafe-fail"
version = "0.1.0"

src/lib.rs:
pub fn dangerous() {
    unsafe { std::ptr::null::<i32>().read() };
}
```

**Create `tests/fixtures/rust/unsafe-ok/`:**
```
Cargo.toml:
[package]
name = "unsafe-ok"
version = "0.1.0"

src/lib.rs:
pub fn safe_dangerous() {
    // SAFETY: Reading from null pointer is actually UB, this is just for testing
    unsafe { std::ptr::null::<i32>().read() };
}
```

**Create `tests/fixtures/rust/unwrap-source/`:**
```
Cargo.toml:
[package]
name = "unwrap-source"
version = "0.1.0"

src/lib.rs:
pub fn risky() -> i32 {
    Some(42).unwrap()
}
```

**Create `tests/fixtures/rust/unwrap-test/`:**
```
Cargo.toml:
[package]
name = "unwrap-test"
version = "0.1.0"

src/lib.rs:
pub fn safe() -> i32 { 42 }

#[cfg(test)]
mod tests {
    #[test]
    fn test_safe() {
        assert_eq!(Some(42).unwrap(), 42);
    }
}
```

**Milestone:** All fixtures created and accessible.

**Verification:**
```bash
ls tests/fixtures/rust/
```

---

### Phase 6: Enable Behavioral Specs

Remove `#[ignore]` from specs that test escape patterns.

**Update `tests/specs/adapters/rust.rs`:**

Change from:
```rust
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_unsafe_without_safety_comment_fails() {
```

To:
```rust
#[test]
fn rust_adapter_unsafe_without_safety_comment_fails() {
```

**Specs to enable:**
- `rust_adapter_unsafe_without_safety_comment_fails`
- `rust_adapter_unsafe_with_safety_comment_passes`
- `rust_adapter_unwrap_in_source_code_fails`
- `rust_adapter_unwrap_in_test_code_allowed`
- `rust_adapter_expect_in_source_code_fails`
- `rust_adapter_transmute_without_safety_comment_fails`

**Note:** Suppress-related specs (allow/forbid lists) are out of scope for this phase.

**Milestone:** Six escape pattern specs pass without `#[ignore]`.

**Verification:**
```bash
cargo test --test specs rust_adapter_unsafe
cargo test --test specs rust_adapter_unwrap
cargo test --test specs rust_adapter_expect
cargo test --test specs rust_adapter_transmute
```

---

## Key Implementation Details

### Pattern Matching Strategy

The patterns use simple regex that balance accuracy with performance:

| Pattern | Regex | Notes |
|---------|-------|-------|
| unsafe | `unsafe\s*\{` | Matches `unsafe {` with optional whitespace |
| unwrap | `\.unwrap\(\)` | Literal method call |
| expect | `\.expect\(` | Opening paren only (args vary) |
| transmute | `mem::transmute` | Qualified path |

### Comment Detection

For `Comment` action patterns, the existing `has_justification_comment()` function searches upward:
1. Same line as the pattern
2. Preceding lines, stopping at non-blank/non-comment lines

This is already implemented in `escapes.rs` and works with `// SAFETY:`.

### Test Code Detection

Test code detection uses two mechanisms:
1. **File-level**: Files matching test patterns (`tests/**`, `*_test.rs`, etc.)
2. **Inline**: Lines inside `#[cfg(test)]` blocks (from Phase 310)

The escapes check already handles this via `classify_file()`. For forbid patterns like `.unwrap()`, violations are only reported for source code, not test code.

### Pattern Override Semantics

When a user configures a pattern with the same name as an adapter default:

```toml
[[check.escapes.patterns]]
name = "unwrap"
action = "count"
threshold = 5
```

This **replaces** the adapter default entirely. The user's config takes full precedence.

### Integration with Existing Escapes Check

The current escapes check flow:

1. Load patterns from config
2. Compile patterns
3. Scan files, track metrics
4. Check thresholds, generate violations

The modification adds step 0.5:
0.5. Get adapter defaults and merge with config

The rest of the flow remains unchanged.

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test adapter::rust
cargo test checks::escapes

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run escape pattern specs
cargo test --test specs rust_adapter_unsafe
cargo test --test specs rust_adapter_unwrap
cargo test --test specs rust_adapter_expect
cargo test --test specs rust_adapter_transmute

# Full quality gates
make check
```

### Test Matrix

| Test Case | Input | Expected |
|-----------|-------|----------|
| unsafe no comment | `unsafe { }` | FAIL, "// SAFETY:" advice |
| unsafe with comment | `// SAFETY: ...\nunsafe { }` | PASS |
| unwrap in source | `src/lib.rs: .unwrap()` | FAIL, "forbidden" |
| unwrap in test file | `tests/foo.rs: .unwrap()` | PASS |
| unwrap in cfg(test) | `src/lib.rs: #[cfg(test)] .unwrap()` | PASS |
| expect in source | `src/lib.rs: .expect("x")` | FAIL, "forbidden" |
| transmute no comment | `mem::transmute(x)` | FAIL, "// SAFETY:" advice |
| transmute with comment | `// SAFETY: ...\nmem::transmute(x)` | PASS |

### Manual Verification

```bash
# Create a test project
mkdir /tmp/rust-test && cd /tmp/rust-test
echo '[package]\nname = "test"\nversion = "0.1.0"' > Cargo.toml
mkdir src
echo 'pub fn f() { Some(1).unwrap(); }' > src/lib.rs

# Run quench
cargo run -- check --escapes

# Expected output:
# escapes: FAIL
#   src/lib.rs:1: .unwrap() in production code
#     Use ? operator or handle the error explicitly.
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Extend EscapePattern | `adapter/mod.rs` | [ ] Pending |
| 2 | Implement default_escapes | `adapter/rust.rs` | [ ] Pending |
| 3 | Unit tests for defaults | `adapter/rust_tests.rs` | [ ] Pending |
| 4 | Merge patterns in check | `checks/escapes.rs` | [ ] Pending |
| 5 | Create fixtures | `tests/fixtures/rust/*` | [ ] Pending |
| 6 | Enable specs | `tests/specs/adapters/rust.rs` | [ ] Pending |

## Future Phases

- **Phase 320**: Suppress attribute checking (`#[allow(...)]` patterns)
- **Phase 325**: Lint config policy enforcement
- **Phase 330**: Shell adapter escape patterns
