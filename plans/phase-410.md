# Phase 410: Shell Adapter - Escapes

## Overview

Add default escape pattern detection for Shell projects. When quench detects a Shell project, it should automatically flag `set +e` and `eval ` usages that lack a `# OK:` justification comment. This parallels the Rust adapter's `unsafe { }` pattern which requires `// SAFETY:` comments.

## Project Structure

Key files to modify/create:

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── shell/
│   │   │   └── mod.rs           # Add default_escapes() implementation
│   │   └── shell_tests.rs       # Add escape pattern tests
│   └── checks/escapes/
│       └── patterns.rs          # Wire up Shell patterns in get_adapter_escape_patterns()
├── tests/
│   ├── specs/checks/
│   │   └── escapes_shell.rs     # Behavioral tests for Shell escape patterns
│   └── fixtures/shell-escapes/  # Test fixtures
│       ├── basic/               # Valid shell with comments
│       ├── missing-comment/     # set +e / eval without comments
│       └── test-files/          # Verify test code is allowed
└── docs/specs/langs/shell.md    # Already specifies behavior (reference)
```

## Dependencies

No new dependencies required. Uses existing:
- `regex` - Pattern matching (via `crate::pattern::CompiledPattern`)
- `globset` - File classification (already used by ShellAdapter)

## Implementation Phases

### Phase 1: Define Shell Escape Patterns

Add the default escape patterns constant to the Shell adapter.

**File:** `crates/cli/src/adapter/shell/mod.rs`

```rust
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Default escape patterns for Shell.
const SHELL_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "set_plus_e",
        pattern: r"set \+e",
        action: EscapeAction::Comment,
        comment: Some("# OK:"),
        advice: "Add a # OK: comment explaining why error checking is disabled.",
    },
    EscapePattern {
        name: "eval",
        pattern: r"\beval\s",
        action: EscapeAction::Comment,
        comment: Some("# OK:"),
        advice: "Add a # OK: comment explaining why eval is safe here.",
    },
];
```

**Milestone:** Constants defined, adapter compiles.

### Phase 2: Implement default_escapes() for ShellAdapter

Override the `default_escapes()` method in the `Adapter` trait implementation.

**File:** `crates/cli/src/adapter/shell/mod.rs`

```rust
impl Adapter for ShellAdapter {
    // ... existing methods ...

    fn default_escapes(&self) -> &'static [EscapePattern] {
        SHELL_ESCAPE_PATTERNS
    }
}
```

**Milestone:** ShellAdapter returns escape patterns via trait method.

### Phase 3: Wire Shell Patterns into Escapes Check

Update `get_adapter_escape_patterns()` to include Shell patterns when a Shell project is detected.

**File:** `crates/cli/src/checks/escapes/patterns.rs`

```rust
pub(super) fn get_adapter_escape_patterns(root: &Path) -> Vec<ConfigEscapePattern> {
    use crate::adapter::Adapter;

    let mut patterns = Vec::new();

    match detect_language(root) {
        ProjectLanguage::Rust => {
            let rust_adapter = RustAdapter::new();
            patterns.extend(convert_adapter_patterns(rust_adapter.default_escapes()));
        }
        ProjectLanguage::Shell => {
            let shell_adapter = ShellAdapter::new();
            patterns.extend(convert_adapter_patterns(shell_adapter.default_escapes()));
        }
        ProjectLanguage::Generic => {
            // No default patterns for generic projects
        }
    }

    patterns
}
```

**Milestone:** Shell projects automatically apply escape patterns.

### Phase 4: Add Unit Tests

Add unit tests for the Shell adapter escape patterns.

**File:** `crates/cli/src/adapter/shell_tests.rs`

```rust
// =============================================================================
// DEFAULT ESCAPE PATTERNS
// =============================================================================

#[test]
fn default_escapes_returns_patterns() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    assert!(!escapes.is_empty(), "should return escape patterns");
}

#[test]
fn default_escapes_includes_set_plus_e() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    let found = escapes.iter().any(|p| p.name == "set_plus_e");
    assert!(found, "should include set_plus_e pattern");
}

#[test]
fn default_escapes_includes_eval() {
    let adapter = ShellAdapter::new();
    let escapes = adapter.default_escapes();
    let found = escapes.iter().any(|p| p.name == "eval");
    assert!(found, "should include eval pattern");
}

#[test]
fn escape_patterns_use_ok_comment() {
    let adapter = ShellAdapter::new();
    for pattern in adapter.default_escapes() {
        assert_eq!(
            pattern.comment,
            Some("# OK:"),
            "pattern {} should require # OK: comment",
            pattern.name
        );
    }
}
```

**Milestone:** Unit tests pass, patterns correctly defined.

### Phase 5: Add Behavioral Tests and Fixtures

Create test fixtures and behavioral specs.

**Fixtures:**

```
tests/fixtures/shell-escapes/
├── basic/
│   ├── quench.toml              # Empty or minimal
│   └── scripts/
│       └── build.sh             # Valid: set +e with # OK: comment
├── missing-comment/
│   ├── quench.toml
│   └── scripts/
│       └── deploy.sh            # Invalid: set +e and eval without comments
└── test-files/
    ├── quench.toml
    └── tests/
        └── integration.bats     # Should be allowed (test code)
```

**Behavioral Tests:** `tests/specs/checks/escapes_shell.rs`

```rust
#[test]
fn shell_set_plus_e_with_comment_passes() {
    // Script with: # OK: need to continue on error\n set +e
    let fixture = setup_fixture("shell-escapes/basic");
    let result = run_quench(&fixture, &["check", "--escapes", "-o", "json"]);
    assert!(result.passed);
}

#[test]
fn shell_set_plus_e_without_comment_fails() {
    // Script with bare: set +e
    let fixture = setup_fixture("shell-escapes/missing-comment");
    let result = run_quench(&fixture, &["check", "--escapes", "-o", "json"]);
    assert!(!result.passed);
    assert!(result.has_violation("set_plus_e", "missing_comment"));
}

#[test]
fn shell_eval_without_comment_fails() {
    let fixture = setup_fixture("shell-escapes/missing-comment");
    let result = run_quench(&fixture, &["check", "--escapes", "-o", "json"]);
    assert!(result.has_violation("eval", "missing_comment"));
}

#[test]
fn shell_escapes_allowed_in_test_files() {
    // .bats file with bare set +e should pass (test code)
    let fixture = setup_fixture("shell-escapes/test-files");
    let result = run_quench(&fixture, &["check", "--escapes", "-o", "json"]);
    assert!(result.passed);
}
```

**Milestone:** All behavioral tests pass, fixtures correctly exercise patterns.

### Phase 6: Verify and Clean Up

Run full check suite and ensure all tests pass.

```bash
make check
```

**Milestone:** `make check` passes with no warnings.

## Key Implementation Details

### Pattern Regex Design

| Pattern | Regex | Rationale |
|---------|-------|-----------|
| `set +e` | `set \+e` | Literal match; `+` needs escaping |
| `eval ` | `\beval\s` | Word boundary prevents matching `evaluate`; trailing whitespace ensures it's the command |

### Comment Detection

The existing `has_justification_comment()` function in `crates/cli/src/checks/escapes/comment.rs` already supports `#` style comments (shell/Python). The function checks:
1. Same line (inline comment after code)
2. Preceding lines (searching upward through comments/blanks)

Example valid patterns:
```bash
# OK: need to continue on failed tests
set +e

set +e  # OK: error handling done manually below

# OK: command string is validated above
eval "$safe_cmd"
```

### Test Code Treatment

Per the escape-hatches spec, test code is:
- **Counted** in metrics (tracked separately as `test` counts)
- **Never fails** checks (escape hatches allowed in tests)

Shell test detection uses `ShellAdapter::classify()`:
- `*.bats` files (BATS test framework)
- `*_test.sh` files
- Files in `tests/` or `test/` directories

## Verification Plan

1. **Unit tests:** `cargo test shell` - adapter tests pass
2. **Behavioral tests:** `cargo test escapes_shell` - specs pass
3. **Integration:** Run against a real shell project:
   ```bash
   cd tests/fixtures/shell-escapes/missing-comment
   cargo run -- check --escapes -o json
   # Should show violations for set +e and eval
   ```
4. **Full suite:** `make check` - all gates pass
