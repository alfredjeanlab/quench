# Phase 495: JavaScript Adapter - Escape Patterns

**Root Feature:** `quench-8006`

## Overview

Implement JavaScript/TypeScript escape pattern detection for the quench escapes check. This adds two default escape patterns:

1. **`as unknown`** - Type assertion bypass requiring `// CAST:` comment justification
2. **`@ts-ignore`** - TypeScript error suppression (forbidden in source, allowed in tests)

The infrastructure for escape detection already exists; this phase adds JavaScript-specific patterns following the same architecture as Rust, Go, and Shell adapters.

## Project Structure

```
crates/cli/src/adapter/
├── javascript/
│   └── mod.rs              # Add JS_ESCAPE_PATTERNS constant + default_escapes()
├── javascript_tests.rs     # Update escape pattern tests
└── mod.rs                  # (no changes needed)

crates/cli/src/checks/escapes/
└── patterns.rs             # Add JavaScript case to get_adapter_escape_patterns()

tests/fixtures/
├── js-simple/              # Existing: verify passes with no escapes
├── js-monorepo/            # Existing: verify passes with no escapes
└── violations/js/
    ├── as-unknown.ts       # Existing: tests as unknown detection
    └── ts-ignore.ts        # Existing: tests @ts-ignore detection
```

## Dependencies

No new dependencies required. Uses existing:
- `globset` for file pattern matching
- `crate::adapter::{EscapePattern, EscapeAction}` for pattern definition
- `crate::checks::escapes` for pattern compilation and matching

## Implementation Phases

### Phase 1: Define JavaScript Escape Patterns

**File:** `crates/cli/src/adapter/javascript/mod.rs`

Add the escape patterns constant following the Rust adapter pattern:

```rust
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Default escape patterns for JavaScript/TypeScript.
const JS_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "as_unknown",
        pattern: r"as\s+unknown",
        action: EscapeAction::Comment,
        comment: Some("// CAST:"),
        advice: "Add a // CAST: comment explaining why the type assertion is necessary.",
    },
    EscapePattern {
        name: "ts_ignore",
        pattern: r"@ts-ignore",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Use @ts-expect-error instead, which fails if the error is resolved.",
    },
];
```

Implement `default_escapes()` on the adapter:

```rust
impl Adapter for JavaScriptAdapter {
    // ... existing methods ...

    fn default_escapes(&self) -> &'static [EscapePattern] {
        JS_ESCAPE_PATTERNS
    }
}
```

**Verification:** Unit tests pass, adapter returns 2 patterns.

### Phase 2: Integrate with Escape Pattern Detection

**File:** `crates/cli/src/checks/escapes/patterns.rs`

Update `get_adapter_escape_patterns()` to handle JavaScript:

```rust
ProjectLanguage::JavaScript => {
    let js_adapter = JavaScriptAdapter::new();
    patterns.extend(convert_adapter_patterns(js_adapter.default_escapes()));
}
```

Add import at top of file:

```rust
use crate::adapter::{
    EscapePattern as AdapterEscapePattern, GoAdapter, JavaScriptAdapter, ProjectLanguage,
    RustAdapter, ShellAdapter, detect_language,
};
```

**Verification:** `get_adapter_escape_patterns()` returns JS patterns for JS projects.

### Phase 3: Update Unit Tests

**File:** `crates/cli/src/adapter/javascript_tests.rs`

Update the escape patterns test to verify the new patterns:

```rust
#[test]
fn default_escapes_has_js_patterns() {
    let adapter = JavaScriptAdapter::new();
    let escapes = adapter.default_escapes();

    assert_eq!(escapes.len(), 2);

    // Verify as_unknown pattern
    let as_unknown = escapes.iter().find(|p| p.name == "as_unknown").unwrap();
    assert_eq!(as_unknown.action, EscapeAction::Comment);
    assert_eq!(as_unknown.comment, Some("// CAST:"));

    // Verify ts_ignore pattern
    let ts_ignore = escapes.iter().find(|p| p.name == "ts_ignore").unwrap();
    assert_eq!(ts_ignore.action, EscapeAction::Forbid);
    assert!(ts_ignore.comment.is_none());
}
```

Add pattern matching tests:

```rust
#[test]
fn as_unknown_pattern_matches() {
    use crate::pattern::CompiledPattern;

    let adapter = JavaScriptAdapter::new();
    let pattern = adapter.default_escapes()
        .iter()
        .find(|p| p.name == "as_unknown")
        .unwrap();

    let compiled = CompiledPattern::compile(pattern.pattern).unwrap();

    // Should match
    assert!(compiled.is_match("data as unknown as UserData"));
    assert!(compiled.is_match("value as  unknown"));  // extra space

    // Should not match
    assert!(!compiled.is_match("as UnknownType"));  // not the keyword
    assert!(!compiled.is_match("// as unknown"));   // in comment (handled separately)
}

#[test]
fn ts_ignore_pattern_matches() {
    use crate::pattern::CompiledPattern;

    let adapter = JavaScriptAdapter::new();
    let pattern = adapter.default_escapes()
        .iter()
        .find(|p| p.name == "ts_ignore")
        .unwrap();

    let compiled = CompiledPattern::compile(pattern.pattern).unwrap();

    // Should match
    assert!(compiled.is_match("// @ts-ignore"));
    assert!(compiled.is_match("// @ts-ignore next line is wrong"));

    // Should not match
    assert!(!compiled.is_match("// @ts-expect-error"));  // allowed alternative
    assert!(!compiled.is_match("// ts-ignore"));         // missing @
}
```

**Verification:** All unit tests pass with `cargo test -p quench`.

### Phase 4: Verify Test Fixtures

Verify the existing fixtures work correctly with the new escape patterns.

**File:** `tests/fixtures/js-simple/` - Should pass (no escapes used)
**File:** `tests/fixtures/js-monorepo/` - Should pass (no escapes used)
**File:** `tests/fixtures/violations/js/as-unknown.ts` - Should fail (missing CAST comment)
**File:** `tests/fixtures/violations/js/ts-ignore.ts` - Should fail (forbidden pattern)

Run integration tests:

```bash
cargo test --test escapes -- js
```

**Verification:** Integration tests detect violations in fixture files.

### Phase 5: Test Code Exemption

Verify that escape patterns are allowed in test files. The escape check already handles test exemption via `FileKind::Test` classification. JavaScript test file detection uses:

- `**/*.test.{js,ts,jsx,tsx}`
- `**/*.spec.{js,ts,jsx,tsx}`
- `**/__tests__/**`
- `test/**`, `tests/**`

Add a test file with escapes to verify exemption:

**File:** `tests/fixtures/js-simple/tests/escapes-ok.test.ts` (if needed)

```typescript
// Test code: @ts-ignore is allowed here
// @ts-ignore
const value: number = "string" as unknown as number;
```

**Verification:** Escapes in test files don't trigger violations.

## Key Implementation Details

### Pattern Regex Design

**`as unknown` pattern:** `r"as\s+unknown"`
- Matches `as unknown` with any whitespace between
- Does not match `as UnknownType` (capital U)
- The `\s+` handles edge cases like `as  unknown` (multiple spaces)

**`@ts-ignore` pattern:** `r"@ts-ignore"`
- Simple literal match
- Located in TypeScript directive comments
- Distinct from `@ts-expect-error` (allowed alternative)

### Action Semantics

| Pattern | Action | Meaning |
|---------|--------|---------|
| `as unknown` | Comment | Allowed with `// CAST:` justification |
| `@ts-ignore` | Forbid | Never allowed in source code |

The `Forbid` action for `@ts-ignore` automatically allows it in test code per the escape check specification (see `docs/specs/checks/escape-hatches.md`).

### Comment Detection

The `// CAST:` comment is detected using the existing `has_justification_comment()` function in `crates/cli/src/checks/escapes/comment.rs`. It searches upward from the pattern match for the required comment prefix.

### TypeScript-Specific Considerations

Both patterns apply to `.ts` and `.tsx` files. The `as unknown` pattern also applies to `.js` files that might use JSDoc type annotations, though it's less common.

## Verification Plan

### Unit Tests

```bash
# Run JavaScript adapter tests
cargo test -p quench javascript

# Run escape pattern tests
cargo test -p quench escapes::patterns
```

### Integration Tests

```bash
# Run all escape-related tests
cargo test --test escapes

# Specific fixture verification
cargo run -- check --escapes tests/fixtures/js-simple
cargo run -- check --escapes tests/fixtures/js-monorepo
cargo run -- check --escapes tests/fixtures/violations/js
```

### Manual Verification

```bash
# Should pass (no escapes)
cargo run -- check tests/fixtures/js-simple

# Should fail with specific violations
cargo run -- check tests/fixtures/violations
# Expected output includes:
#   violations/js/as-unknown.ts:2: as unknown without // CAST: comment
#   violations/js/ts-ignore.ts:2: @ts-ignore in production code
```

### Full Test Suite

```bash
make check
```

## Acceptance Criteria

1. `JavaScriptAdapter::default_escapes()` returns 2 patterns
2. `as unknown` violations require `// CAST:` comment
3. `@ts-ignore` violations fail in source, pass in test files
4. Existing `js-simple` and `js-monorepo` fixtures pass
5. Violation fixtures correctly detect violations
6. All unit tests pass
7. `make check` succeeds
