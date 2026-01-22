# Spec Rules

These are behavioral specifications for quench. They test the CLI as a black box.

See `docs/arch/e2e-tests.md` for full architecture details.

## Golden Rule

**Specs test behavior, not implementation.**

Write specs by reading `docs/specs/`, not by reading `src/`.

## DO

- Use `check("name").on("fixture").passes()` for simple cases
- Use `quench()` for complex multi-check scenarios
- Check stdout, stderr, and exit codes
- Use fixtures from `tests/fixtures/`
- Reference the spec doc section in a doc comment
- Use `#[ignore = "TODO: Phase N - description"]` for unimplemented specs
- Use `insta::assert_snapshot!` for output format tests
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
/// > File size limit checking (max_lines, default 750)
#[test]
fn cloc_fails_on_oversized_file() {
    check("cloc")
        .on("violations")
        .fails()
        .with_output("lines (max: 750)");
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
        .with_output("// SAFETY:");
}
```

## Helpers Available

```rust
use crate::prelude::*;

// High-level (preferred)
check("cloc").on("rust-simple").passes();
check("cloc").on("violations").fails().with_violation("oversized.rs");
check("cloc").json().on("rust-simple").passes().json(|v| { ... });

// Low-level (for complex cases)
quench()
    .args(["check", "--cloc", "--escapes"])
    .current_dir(fixture("rust-simple"))
    .assert()
    .success();

// Temp directories
let dir = tempdir().unwrap();
std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
check("cloc").in_dir(dir.path()).passes();
```

## Running Specs

```bash
cargo test --test specs              # Fast specs only
cargo test --test specs -- --ignored # Show unimplemented count
cargo insta test                     # Update snapshots
cargo insta review                   # Review snapshot changes
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
