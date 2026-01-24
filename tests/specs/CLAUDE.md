# Spec Rules

These are behavioral specifications for quench. They test the CLI as a black box.

See `docs/arch/e2e-tests.md` for full architecture details.

## Golden Rule

**Specs test behavior, not implementation.**

Write specs by reading `docs/specs/`, not by reading `src/`.

## DO

- Use `check("name").on("fixture").passes()` for single-check tests
- Use `cli()` for multi-check scenarios
- **Prefer `stdout_eq()` for exact output comparison** - catches format regressions
- Use `stdout_has()` only when exact comparison isn't practical
- Check stdout, stderr, and exit codes
- Use fixtures from `tests/fixtures/`
- Reference the spec doc section in a doc comment
- Use `#[ignore = "TODO: Phase N - description"]` for unimplemented specs
- Create temp dirs for config-only tests

## DO NOT

- Import anything from `quench::*` or `crate::*`
- Read or inspect internal state
- Call internal functions directly
- Write specs by looking at the implementation
- Remove or modify `#[ignore]` without implementing the feature
- Hardcode paths (use `fixture()` helper)

## Spec Template

```rust
/// Spec: docs/specs/checks/cloc.md#file-size-limits
///
/// > max_lines = 750 (default for source files)
#[test]
fn cloc_fails_on_source_file_over_max_lines() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v.get("file").and_then(|f| f.as_str()).unwrap().ends_with("big.rs")
    }));
}
```

## Unimplemented Spec Template

```rust
/// Spec: docs/specs/checks/escapes.md#comment-action
#[test]
#[ignore = "TODO: Phase 10 - Escapes Check Actions"]
fn escapes_requires_safety_comment() {
    check("escapes")
        .on("violations")
        .fails()
        .stdout_has("// SAFETY:");
}
```

## Output Comparison

**Prefer exact comparison** - catches format regressions and unexpected changes:

```rust
// GOOD: Exact output comparison with diff on failure
cli().on("fixture").exits(1).stdout_eq(
    "cloc: FAIL
  src/file.rs: file_too_large (lines: 100 vs 50)
    Split into smaller modules.

FAIL: cloc
"
);

// ACCEPTABLE: Pattern matching when exact comparison isn't practical
cli().on("fixture").fails().stdout_has("file_too_large");

// AVOID: Vague checks that miss format regressions
cli().on("fixture").fails(); // No output validation at all
```

**When to use each:**
- `stdout_eq(expected)` - **Default choice.** Use for format specs and stable output
- `stdout_has(pattern)` - When output varies (timestamps, file counts, etc.)
- `stdout_lacks(pattern)` - Verify absence (no ANSI codes, no debug output)

## Helpers Available

```rust
use crate::prelude::*;

// Exact output comparison (preferred for format specs)
cli().on("fixture").exits(1).stdout_eq("cloc: FAIL\n  ...\n");
cli().on("fixture").passes().stderr_eq(""); // No errors

// Pattern matching (when exact comparison isn't practical)
check("cloc").on("fixture").fails().stdout_has("big.rs");
cli().on("fixture").env("CLAUDE_CODE", "1").exits(1).stdout_lacks("\x1b[");

// Single check -> CheckJson
let cloc = check("cloc").on("cloc/basic").json().passes();
assert!(cloc.require("metrics").get("ratio").is_some());

// Violation helpers on CheckJson
assert!(cloc.has_violation("file_too_large"));
let v = cloc.require_violation("file_too_large");
let vs = cloc.violations_of_type("file_too_large");
assert!(cloc.has_violation_for_file("big.rs"));

// All checks -> ChecksJson
let result = cli().on("output-test").json().fails();
assert!(result.checks().len() > 0);

// Temp directories (with defaults: quench.toml + CLAUDE.md)
let temp = default_project();
temp.config("[check.cloc]\nmax_lines = 5");
temp.file("src/lib.rs", "fn main() {}");
check("cloc").pwd(temp.path()).fails();

// Empty temp directory (for init tests)
let temp = Project::empty();
temp.config("[check.agents]\nrequired = [\"CLAUDE.md\"]");
temp.file("CLAUDE.md", "# Project\n...");
check("agents").pwd(temp.path()).passes();
```

## Running Specs

```bash
cargo test --test specs              # Fast specs only
cargo test --test specs -- --ignored # Show unimplemented count
```

## When Adding a New Spec

1. Find the relevant section in `docs/specs/`
2. Quote the spec text in your doc comment
3. Write the test to verify that behavior
4. If not yet implemented, add `#[ignore = "TODO: Phase N - ..."]`
5. Run `cargo test --test specs` to verify it compiles

## When Implementing a Feature

1. Find specs marked `#[ignore]` for your phase
2. Implement the feature in `src/`
3. Remove the `#[ignore]` attribute
4. Run `cargo test --test specs` to verify specs pass
5. List passing specs in your commit message
