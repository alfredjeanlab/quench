# Checkpoint 7H: Tech Debt - Docs Check

## Overview

Clean up tech debt in the docs check by moving implemented features from "Future Work" documentation to the main spec and adding missing behavioral tests. The explicit TOC code block syntax feature (`toc` and `no-toc` language tags) is fully implemented but incorrectly documented as future work and lacks behavioral test coverage.

## Project Structure

```
quench/
├── docs/specs/
│   ├── checks/docs.md         # Main docs check specification (UPDATE)
│   └── 99-todo.md             # Future work (REMOVE toc section)
├── crates/cli/src/checks/docs/
│   ├── toc/
│   │   ├── detect.rs          # TOC_LANGUAGE const, looks_like_tree()
│   │   └── mod.rs             # invalid_toc_format violation
│   └── toc_tests.rs           # Unit tests (already exist)
└── tests/specs/checks/docs/
    └── toc.rs                 # Behavioral tests (ADD new tests)
```

## Dependencies

No new dependencies required. Uses existing test infrastructure:
- `crate::prelude::*` for test helpers
- `check("docs")` for single-check tests
- `default_project()` / `Project::empty()` for temp directories

## Implementation Phases

### Phase 1: Add Behavioral Tests for Explicit TOC Tag

Add behavioral (black-box) tests to `tests/specs/checks/docs/toc.rs` for the explicit `toc` language tag feature.

**Files to modify:**
- `tests/specs/checks/docs/toc.rs`

**Tests to add:**

```rust
/// Spec: docs/specs/checks/docs.md#explicit-toc-syntax
///
/// > Code blocks tagged `toc` are always validated as directory trees.
#[test]
fn explicit_toc_tag_forces_validation() {
    let temp = default_project();
    temp.file("src/lib.rs", "// lib\n");
    temp.file(
        "CLAUDE.md",
        r#"# Project

```toc
src/
  lib.rs
```
"#,
    );
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#explicit-toc-syntax
///
/// > Code blocks tagged `toc` report missing files.
#[test]
fn explicit_toc_tag_reports_missing_files() {
    let temp = default_project();
    temp.file(
        "CLAUDE.md",
        r#"# Project

```toc
src/
  missing.rs
```
"#,
    );
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("broken_toc")
        .stdout_has("missing.rs");
}

/// Spec: docs/specs/checks/docs.md#explicit-toc-syntax
///
/// > Code blocks tagged `toc` fail with invalid_toc_format if format is wrong.
#[test]
fn explicit_toc_tag_invalid_format_generates_violation() {
    let temp = default_project();
    temp.file(
        "CLAUDE.md",
        r#"# Project

```toc
This is not a valid tree format
Just some random text here
```
"#,
    );
    check("docs")
        .pwd(temp.path())
        .fails()
        .stdout_has("invalid_toc_format")
        .stdout_has("doesn't match box-drawing or indentation format");
}

/// Spec: docs/specs/checks/docs.md#explicit-toc-syntax
///
/// > Code blocks tagged `no-toc` are never validated.
#[test]
fn no_toc_tag_skips_validation() {
    let temp = default_project();
    temp.file(
        "CLAUDE.md",
        r#"# Project

```no-toc
src/
├── definitely-missing.rs
└── also-missing.rs
```
"#,
    );
    // Should pass because no-toc blocks are skipped
    check("docs").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/docs.md#explicit-toc-syntax
///
/// > Code blocks tagged `ignore` are never validated (alias for no-toc).
#[test]
fn ignore_tag_skips_validation() {
    let temp = default_project();
    temp.file(
        "CLAUDE.md",
        r#"# Project

```ignore
src/
├── missing.rs
```
"#,
    );
    check("docs").pwd(temp.path()).passes();
}
```

**Milestone:** All new behavioral tests pass with `cargo test --test specs`.

### Phase 2: Update Main Documentation Spec

Move the "Explicit TOC Code Block Syntax" section from `docs/specs/99-todo.md` to `docs/specs/checks/docs.md`.

**Files to modify:**
- `docs/specs/checks/docs.md` - Add new section
- `docs/specs/99-todo.md` - Remove implemented section

**Add to docs.md** (after "What Gets Validated" section):

```markdown
### Explicit TOC Syntax

Use the `toc` language tag to force validation of a code block as a directory tree:

~~~markdown
```toc
src/
├── lib.rs
├── parser.rs
└── utils/
    └── helpers.rs
```
~~~

Use `no-toc` or `ignore` to explicitly skip validation:

~~~markdown
```no-toc
hypothetical/
├── future-feature.rs
└── not-yet-implemented.rs
```
~~~

| Tag | Behavior |
|-----|----------|
| `toc` | Always validate as directory tree |
| `no-toc` | Never validate (explicit skip) |
| `ignore` | Never validate (alias for `no-toc`) |

**Invalid Format Error:**

If a `toc`-tagged block doesn't match box-drawing or indentation format:

```
docs: FAIL
  CLAUDE.md:5: invalid_toc_format
    Code block marked as `toc` doesn't match box-drawing or indentation format.
    Use box-drawing (├──, └──, │) or consistent indentation.
```
```

**Milestone:** Documentation accurately reflects implementation; `quench check` passes on the project itself.

### Phase 3: Remove from Future Work

Remove the "Explicit TOC Code Block Syntax" section from `docs/specs/99-todo.md` since it is now implemented and documented.

**Files to modify:**
- `docs/specs/99-todo.md` - Delete the entire "Explicit TOC Code Block Syntax" section (lines ~80-132)

**Milestone:** No duplicate documentation; `99-todo.md` only contains unimplemented features.

### Phase 4: Verification and Cleanup

Run full verification suite:

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `quench check` (dogfooding)
- `cargo audit`
- `cargo deny check`

**Milestone:** All checks pass; changes ready for commit.

## Key Implementation Details

### Violation Types

The explicit TOC feature uses two violation types:

| Type | When | Advice |
|------|------|--------|
| `broken_toc` | File in tree doesn't exist | "File does not exist..." |
| `invalid_toc_format` | `toc` block has invalid format | "Code block marked as `toc` doesn't match..." |

### Detection Logic (Already Implemented)

From `crates/cli/src/checks/docs/toc/detect.rs`:

```rust
pub(crate) const TOC_LANGUAGE: &str = "toc";

const NON_TREE_LANGUAGES: &[&str] = &[
    "no-toc", "ignore",
    // ... other known languages
];

pub(crate) fn looks_like_tree(block: &FencedBlock) -> bool {
    // Explicit toc tag forces validation
    if block.language.as_deref() == Some(TOC_LANGUAGE) {
        return true;
    }
    // Blocks with known non-tree language tags are skipped
    if let Some(ref lang) = block.language
        && NON_TREE_LANGUAGES.contains(&lang.as_str())
    {
        return false;
    }
    // ... heuristic detection
}
```

### Format Validation (Already Implemented)

From `crates/cli/src/checks/docs/toc/mod.rs`:

```rust
// For explicit toc tag, validate format
if block.language.as_deref() == Some(TOC_LANGUAGE) && !is_valid_tree_format(&block) {
    violations.push(Violation::file(
        relative_path,
        block.start_line,
        "invalid_toc_format",
        "Code block marked as `toc` doesn't match box-drawing or indentation format.\n\
         Use box-drawing (├──, └──, │) or consistent indentation.",
    ));
    continue;
}
```

## Verification Plan

### Unit Test Coverage

Existing unit tests in `crates/cli/src/checks/docs/toc_tests.rs`:
- `no_toc_block_skipped` - verifies `no-toc` tag skips
- `ignore_block_skipped` - verifies `ignore` tag skips
- `toc_tag_forces_validation` - verifies `toc` tag forces validation
- `toc_tag_with_box_drawing` - verifies `toc` with box drawing
- `toc_tag_invalid_format_detected` - verifies invalid format detection
- `toc_tag_valid_indentation_format` - verifies valid indentation
- `toc_tag_valid_box_drawing_format` - verifies valid box drawing
- `toc_tag_empty_block_invalid` - verifies empty block is invalid

### Behavioral Test Coverage (To Add)

New tests in `tests/specs/checks/docs/toc.rs`:
- `explicit_toc_tag_forces_validation` - end-to-end validation
- `explicit_toc_tag_reports_missing_files` - missing file detection
- `explicit_toc_tag_invalid_format_generates_violation` - format error
- `no_toc_tag_skips_validation` - skip behavior
- `ignore_tag_skips_validation` - alias behavior

### Manual Verification

1. Create test file with `toc` block containing missing files - verify failure
2. Create test file with `no-toc` block containing missing files - verify pass
3. Create test file with `toc` block with invalid format - verify `invalid_toc_format` error
4. Run `quench check` on the project itself - verify dogfooding passes

### Final Checklist

- [ ] All new behavioral tests pass
- [ ] Documentation moved from `99-todo.md` to `docs.md`
- [ ] `make check` passes
- [ ] Commit message lists passing specs
