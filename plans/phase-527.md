# Phase 527: Dry-Run Mode - Specs

**Root Feature:** `quench-909f`

## Overview

Add behavioral specifications for the `--dry-run` flag. This flag allows users to preview what `--fix` would change without actually modifying any files. The specs will be marked with `#[ignore]` until the implementation phase.

Key behaviors to specify:
- `--dry-run` requires `--fix` (error without it)
- Shows files that would be modified
- Shows diff of proposed changes
- Exits 0 even when fixes are needed
- Does not modify any files

Reference: `docs/specs/01-cli.md#output-flags`

## Project Structure

```
tests/
├── specs/
│   ├── cli/
│   │   ├── mod.rs              # Add dry_run module
│   │   └── dry_run.rs          # NEW: Dry-run mode specs
│   └── prelude.rs              # Existing test helpers (sufficient)
└── fixtures/
    └── agents/                 # Existing fixtures sufficient for dry-run tests
```

## Dependencies

No new dependencies. Uses existing:
- `tempfile` for temp directories
- `assert_cmd` for CLI testing

## Implementation Phases

### Phase 1: Create Spec File Structure

Create the new spec module file and register it.

**Tasks:**
1. Create `tests/specs/cli/dry_run.rs`
2. Add module declaration in `tests/specs/cli/mod.rs` (if exists) or main spec file

**File: `tests/specs/cli/dry_run.rs`:**
```rust
//! Behavioral specs for the --dry-run flag.
//!
//! Tests that quench correctly handles dry-run mode:
//! - Requires --fix flag
//! - Shows files that would be modified
//! - Shows diff of proposed changes
//! - Exits 0 even when fixes needed
//! - Does not modify any files
//!
//! Reference: docs/specs/01-cli.md#output-flags

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
```

**Verification:**
```bash
cargo test --test specs dry_run
```

### Phase 2: Spec - --dry-run Without --fix Is Error

`--dry-run` only makes sense with `--fix`. Using it alone should produce a configuration error.

**Spec:**
```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run: Show what --fix would change without changing it
/// > Using --dry-run without --fix is an error.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_without_fix_is_error() {
    let dir = temp_project();
    cli()
        .pwd(dir.path())
        .args(&["--dry-run"])
        .exits(2)  // Configuration error
        .stderr_has("--dry-run requires --fix");
}
```

**Exit code 2** per the CLI spec indicates configuration/argument error.

### Phase 3: Spec - --dry-run Shows Files That Would Be Modified

When there are fixable violations, `--dry-run` should list which files would be modified.

**Spec:**
```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows files that would be modified without modifying them.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_shows_files_that_would_be_modified() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has(".cursorrules");
}
```

### Phase 4: Spec - --dry-run Shows Diff of Changes

The output should include a diff showing what would change in each file.

**Spec:**
```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run shows diff of proposed changes.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_shows_diff_of_changes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Diff output should show both old and new content
    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes()
        .stdout_has("Content B")  // Old content (being removed)
        .stdout_has("Content A"); // New content (being added)
}
```

### Phase 5: Spec - --dry-run Exits 0 Even When Fixes Needed

Unlike normal `--fix` behavior (which might exit 1 for unfixable violations), `--dry-run` should exit 0 as long as it successfully shows what would be fixed.

**Spec:**
```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run exits 0 even when fixes are needed.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_exits_0_when_fixes_needed() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Files are out of sync, fixes are needed, but --dry-run exits 0
    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes();  // passes() expects exit code 0
}
```

### Phase 6: Spec - --dry-run Does Not Modify Any Files

The core invariant: `--dry-run` must never write to disk.

**Spec:**
```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --dry-run does not modify any files.
#[test]
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
fn dry_run_does_not_modify_files() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Target\nContent B").unwrap();

    // Run with --dry-run
    cli()
        .pwd(dir.path())
        .args(&["--fix", "--dry-run"])
        .passes();

    // Verify .cursorrules was NOT modified
    let content = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(content, "# Target\nContent B", "file should not be modified");
}
```

## Key Implementation Details

### Spec Organization

All specs go in `tests/specs/cli/dry_run.rs` organized by behavior:

```rust
// =============================================================================
// ERROR HANDLING SPECS
// =============================================================================

// dry_run_without_fix_is_error

// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

// dry_run_shows_files_that_would_be_modified
// dry_run_shows_diff_of_changes

// =============================================================================
// EXIT CODE SPECS
// =============================================================================

// dry_run_exits_0_when_fixes_needed

// =============================================================================
// FILE INTEGRITY SPECS
// =============================================================================

// dry_run_does_not_modify_files
```

### Test Fixture Strategy

All specs use `temp_project()` with inline file creation rather than fixtures, because:
1. Dry-run behavior depends on file content differences
2. Tests need to verify file content after running
3. Inline setup makes the test self-documenting

### Expected Ignore Format

All specs use consistent ignore annotation:
```rust
#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]
```

This allows:
- Finding all unimplemented specs: `cargo test --test specs -- --ignored`
- Tracking implementation progress
- Clear phase dependency

## Verification Plan

### Spec Compilation

```bash
# Verify specs compile (even when ignored)
cargo test --test specs dry_run
```

### Ignored Spec Count

```bash
# Show ignored specs count
cargo test --test specs dry_run -- --ignored 2>&1 | grep "ignored"
```

Expected: 5 tests ignored

### Full Test Suite

```bash
# Ensure all existing tests still pass
make check
```

### Spec Review Checklist

| Spec | Behavior | Exit Code | Expected |
|------|----------|-----------|----------|
| `dry_run_without_fix_is_error` | Error handling | 2 | stderr has error |
| `dry_run_shows_files_that_would_be_modified` | Output | 0 | stdout has filename |
| `dry_run_shows_diff_of_changes` | Output | 0 | stdout has old+new |
| `dry_run_exits_0_when_fixes_needed` | Exit code | 0 | passes() succeeds |
| `dry_run_does_not_modify_files` | Invariant | 0 | file unchanged |

### Acceptance Criteria

1. New file `tests/specs/cli/dry_run.rs` exists
2. 5 specs defined, all with `#[ignore = "TODO: Phase 528 - Dry-Run Implementation"]`
3. Specs compile without errors
4. Each spec has doc comment referencing `docs/specs/01-cli.md`
5. `make check` passes

## Spec Status (After This Phase)

| Spec | Status |
|------|--------|
| dry_run_without_fix_is_error | Ignored (528) |
| dry_run_shows_files_that_would_be_modified | Ignored (528) |
| dry_run_shows_diff_of_changes | Ignored (528) |
| dry_run_exits_0_when_fixes_needed | Ignored (528) |
| dry_run_does_not_modify_files | Ignored (528) |
