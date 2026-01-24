# Escape Hatches Specification

The `escapes` check ensures escape hatches are documented.

## Purpose

Ensure "escape hatches" have justification comments:
- Bypassing compile-time safety (`unsafe`, type casts)
- Silencing warnings (`#[allow(...)]`)

Quench does not forbid usage directly, and assumes you are already running language-specific linters (Clippy, ShellCheck). Instead it ensures escapes and suppressions are commented.

## Actions

Each pattern can be configured with one of three actions:

### Count

Just count occurrences. Fail if count exceeds per-pattern threshold (default: 0).

Use for: Patterns you want to monitor with a specific tolerance.

### Comment

Pattern is allowed if accompanied by a justification comment.

Use for: Patterns that are sometimes necessary but should be documented.

```rust
// SAFETY: The pointer is valid because we just allocated it above
unsafe { *ptr = value; }
```

### Forbid

Pattern is never allowed in source code. Always allowed in test code.

Use for: Patterns that should not appear in production code.

## Default Patterns

### Rust

| Pattern | Default Mode | Comment Required |
|---------|--------------|------------------|
| `unsafe { }` | comment | `// SAFETY:` |
| `mem::transmute` | comment | `// SAFETY:` |

Lint suppressions (`#[allow(...)]`, `#[expect(...)]`) are configured separately via `[rust.suppress]`. See [langs/rust.md](../langs/rust.md#suppress).

### Go

| Pattern | Default Mode | Comment Required |
|---------|--------------|------------------|
| `unsafe.Pointer` | comment | `// SAFETY:` |
| `//go:linkname` | comment | `// LINKNAME:` |
| `//go:noescape` | comment | `// NOESCAPE:` |

Lint suppressions (`//nolint`) are configured separately via `[golang.suppress]`. See [langs/golang.md](../langs/golang.md#suppress).

### Shell

| Pattern | Default Mode | Comment Required | Threshold |
|---------|--------------|------------------|-----------|
| `# shellcheck disable=` | forbid | - | 0 |
| `set +e` | comment | `# OK:` | 0 |
| `eval ` | comment | `# OK:` | 0 |

## Source vs Test

Escape hatches are **counted separately** for source and test code:
- **Source code**: Thresholds and actions are enforced
- **Test code**: Counted for metrics, but never fails (test code is allowed escape hatches)

Test code identification uses the same patterns as the `loc` check:
- Files matching test patterns (`*_test.rs`, `tests/**`, etc.)
- Lines inside `#[cfg(test)]` blocks (Rust-specific)

## Per-Package Breakdown

When packages are configured, counts are tracked per-package:
- Each package has its own source/test counts
- Thresholds apply to **total source count** by default
- Per-package thresholds can be configured via overrides
- `by_package` field is **omitted from JSON** if no packages configured

## Comment Detection

For `comment` action, quench searches **upward** for the required comment:
1. On the same line as the pattern
2. On preceding lines, searching upward until a non-blank, non-comment line is found

This allows comments to be separated from the pattern by other comments or blank lines:

```rust
// SAFETY: The lock is held for the duration of this block
unsafe {                    // ✓ Comment found on preceding line
    critical_section();
}

unsafe { quick_op(); }      // SAFETY: Single atomic op  ✓ Comment on same line

// SAFETY: Pointer guaranteed valid by constructor invariant
//
// Additional context about why this is safe...
unsafe {                    // ✓ Comment found (searching upward through blanks/comments)
    indirect_call();
}

fn other_code() {}
unsafe {                    // ✗ No comment (search stopped at `fn other_code`)
    risky_call();
}
```

## Lint Suppression Messages

When a lint suppression is missing a required comment, the error message includes:

1. **General statement**: "Lint suppression requires justification."
2. **Lint-specific guidance**: A thoughtful question or statement about the specific lint
3. **Required patterns**: The list of acceptable comment patterns (if configured)

The lint-specific guidance is tailored to each built-in pattern to help developers think critically about whether the suppression is necessary.

**Message format:**
```
<file>:<line>: suppress_missing_comment: <directive>
  Lint suppression requires justification.
  <lint-specific guidance>
  <pattern instructions>
```

**Pattern instructions format:**
- Single pattern: `If so, add:\n  <pattern>`
- Multiple patterns: `If it should be kept, add one of:\n  <pattern1>\n  <pattern2>\n  ...`
- No specific pattern: `Add a comment above the directive or inline (<example>).`

See language-specific documentation for built-in lint-specific guidance:
- [Rust suppress guidance](../langs/rust.md#default-per-lint-guidance)
- [Go suppress guidance](../langs/golang.md#suppress)
- [Shell suppress guidance](../langs/shell.md#suppress)

## Configurable Advice

Each pattern can have custom advice:

```toml
[[check.escapes.patterns]]
name = "as_uuid"
pattern = " as UUID"
action = "forbid"
advice = "Use typed UUID fields instead of casting. Mark properties as 'id: UUID' not 'id: string'."
```

## Output

### Fail (comment violation)

```
escapes: FAIL
  src/parser.rs:47: unsafe block without // SAFETY: comment
    Add a // SAFETY: comment explaining why this unsafe block is sound.
```

### Fail (suppress missing comment)

When a lint suppression is missing a required comment, the message provides:
1. A general statement that justification is required
2. Lint-specific guidance tailored to the suppressed lint
3. The list of acceptable comment patterns

**Examples (Rust):**

```
escapes: FAIL
  src/git_hooks.rs:109: suppress_missing_comment: #[allow(dead_code)]
    Lint suppression requires justification.
    Is this code still needed?
    If it should be kept, add one of:
      // KEEP UNTIL: ...
      // NOTE(compat): ...
      // NOTE(compatibility): ...

  src/display.rs:106: suppress_missing_comment: #[allow(clippy::too_many_arguments)]
    Lint suppression requires justification.
    Can this function be refactored?
    If not, add:
      // TODO(refactor): ...

  src/daemon/runner.rs:598: suppress_missing_comment: #[allow(clippy::cast_possible_truncation)]
    Lint suppression requires justification.
    Is this cast safe?
    If so, add one of:
      // CORRECTNESS: ...
      // SAFETY: ...
```

**Examples (Shell):**

```
escapes: FAIL
  scripts/deploy.sh:23: shellcheck_missing_comment: # shellcheck disable=SC2086
    Lint suppression requires justification.
    Is unquoted expansion intentional here?
    Add a comment above the directive.
```

See language-specific documentation for complete per-lint guidance:
- [Rust suppress guidance](../langs/rust.md#default-per-lint-guidance)
- [Go suppress](../langs/golang.md#suppress)
- [Shell suppress guidance](../langs/shell.md#violation-messages)

### Fail (forbid violation)

```
escapes: FAIL
  src/client.rs:89: .unwrap() in production code
    Handle the error case or use .expect() with a message if truly infallible.
```

### Fail (threshold exceeded)

```
escapes: FAIL
  "todo": 23 occurrences (max: 10)
    src/parser.rs: 8 occurrences
    src/lexer.rs: 7 occurrences
    src/compiler.rs: 5 occurrences
    (3 more files...)
    Reduce TODO/FIXME comments or increase threshold.
```

### JSON Output

```json
{
  "name": "escapes",
  "passed": false,
  "violations": [
    {
      "file": "src/parser.rs",
      "line": 47,
      "type": "missing_comment",
      "pattern": "unsafe",
      "comment": "// SAFETY:",
      "advice": "Add a // SAFETY: comment explaining why this unsafe block is sound."
    },
    {
      "file": "src/client.rs",
      "line": 89,
      "type": "forbidden",
      "pattern": "unwrap",
      "advice": "Handle the error case or use .expect() with a message if truly infallible."
    }
  ],
  "metrics": {
    "source": { "unsafe": 3, "unwrap": 0, "expect": 0, "allow": 12 },
    "test": { "unsafe": 0, "unwrap": 47, "expect": 5, "allow": 0 }
  },
  "by_package": {
    "cli": {
      "source": { "unsafe": 1, "unwrap": 0, "expect": 0, "allow": 4 },
      "test": { "unsafe": 0, "unwrap": 23, "expect": 2, "allow": 0 }
    },
    "core": {
      "source": { "unsafe": 2, "unwrap": 0, "expect": 0, "allow": 8 },
      "test": { "unsafe": 0, "unwrap": 24, "expect": 3, "allow": 0 }
    }
  }
}
```

**Violation types**: `missing_comment`, `forbidden`, `threshold_exceeded`

## Configuration

```toml
[check.escapes]
check = "error"

# Custom or override patterns
[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
# Custom advice for this project
advice = "Use .context() from anyhow or handle the error explicitly."

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME|XXX"
action = "count"
threshold = 10          # Allow up to 10 (default: 0)
advice = "Reduce TODO/FIXME comments before shipping."

# Per-package overrides
[check.escapes.package.cli]
# Stricter for CLI package
[[check.escapes.package.cli.patterns]]
name = "todo"
threshold = 5
```

## Performance

Pattern matching must be fast:
1. Compile regex patterns once at startup
2. Use ripgrep's regex engine (`grep-regex`) for speed
3. Parallel file scanning
4. Early termination for fast checks (non-CI):
   - Stop scanning a file when threshold is exceeded AND max output violations reached
   - Prevents overwhelming agent context with violations
   - Full counts only computed in `--ci` mode for metrics storage
