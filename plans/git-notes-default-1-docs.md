# Plan 1: Git Notes Default - Docs & Specs

**Date:** 2026-01-27
**Scope:** Update documentation and tests/specs for git notes as default baseline storage

## Summary

Update all documentation and write/update behavioral tests to reflect git notes as the default baseline storage mechanism. Tests for unimplemented features use `#[ignore = "TODO: Phase 2"]` or `#[ignore = "TODO: Phase 3"]`.

## Motivation

Currently:
- `.quench/baseline.json` is the default baseline storage
- Git notes are opt-in via `--save-notes`
- Baseline file may become stale/out-of-sync with actual commit history

Proposed:
- Git notes become the default (per-commit metrics history)
- `.quench/latest.json` caches the most recent metrics locally
- `.quench/baseline.json` becomes opt-in for teams preferring file-based baselines

## Documentation Changes

### 1. `docs/specs/04-ratcheting.md`

**Current:**
```toml
[git]
baseline = ".quench/baseline.json"
```

**New:**
```toml
[git]
# Baseline source: "notes" (default) or path to file
baseline = "notes"

# Optional: explicit file path for file-based baseline
# baseline = ".quench/baseline.json"
```

**Section updates:**
- "Baseline Storage" → describe notes as default, file as alternative
- Update CI workflow examples (notes workflow becomes primary)
- Add note about `.quench/latest.json` for local caching

### 2. `docs/specs/01-cli.md`

**Flag changes:**

| Old | New |
|-----|-----|
| `--save-notes` | Remove (default behavior) |
| `--save <FILE>` | `--save <FILE>` (unchanged, explicit file) |
| N/A | `--no-notes` (disable git notes, use file only) |
| `--base <REF>` | Also used for baseline note lookup (unchanged flag, extended behavior) |

**New flags:**
```
--no-notes        Disable git notes, use file-based baseline only
```

**Extended behavior:**
```
--base <REF>      Now also determines which commit's note to use for ratchet comparison
```

### 3. `docs/specs/02-config.md`

**Update `[git]` section:**
```toml
[git]
# Baseline source (default: "notes")
#   "notes" - use git notes (refs/notes/quench)
#   "<path>" - use file at path (e.g., ".quench/baseline.json")
baseline = "notes"
```

### 4. `docs/specs/00-overview.md`

Update "Metrics storage to baseline file or git notes" → "Metrics storage to git notes (default) or baseline file"

---

## Test/Spec Changes

### File: `tests/specs/modes/ratchet.rs`

**New tests (implement in Phase 2):**

```rust
/// Spec: docs/specs/04-ratcheting.md
///
/// > Git notes is the default baseline source
#[test]
#[ignore = "TODO: Phase 2"]
fn ratchet_reads_baseline_from_git_notes_by_default() {
    // Setup: create project with git history and notes
    // Run: quench check (no --save-notes flag)
    // Assert: compares against notes, not file
}

/// > Baseline falls back to file when notes unavailable
#[test]
#[ignore = "TODO: Phase 2"]
fn ratchet_falls_back_to_file_when_no_notes() {
    // Setup: project with baseline.json but no notes
    // Run: quench check
    // Assert: uses file baseline
}

/// > --no-notes disables git notes entirely
#[test]
#[ignore = "TODO: Phase 3"]
fn no_notes_flag_uses_file_only() {
    // Setup: project with both notes and file
    // Run: quench check --no-notes
    // Assert: uses file, ignores notes
}

/// > --base <REF> uses baseline from that commit's note for ratchet comparison
#[test]
#[ignore = "TODO: Phase 2"]
fn base_ref_uses_baseline_from_that_commit() {
    // Setup: project with notes on multiple commits
    // Run: quench check --base main~5
    // Assert: uses baseline from that commit's note
}
```

### File: `tests/specs/cli/ci_mode.rs`

**Update existing tests:**

```rust
/// > --fix saves to git notes by default (was --save-notes)
#[test]
fn fix_saves_to_git_notes_by_default() {
    // ... (rename from save_notes_writes_to_git)
}

/// > --save <FILE> saves to explicit file path
#[test]
fn save_file_writes_to_specified_path() {
    // existing test, unchanged
}
```

**New tests (implement in Phase 3):**

```rust
/// > --fix also writes .quench/latest.json for local caching
#[test]
#[ignore = "TODO: Phase 3"]
fn fix_writes_latest_json_cache() {
    // Setup: git project
    // Run: quench check --fix
    // Assert: .quench/latest.json exists with current metrics
}
```

### File: `tests/specs/config/git.rs` (new file)

```rust
//! Tests for git configuration.
//!
//! Reference: docs/specs/02-config.md#git

use crate::prelude::*;

/// > baseline = "notes" uses git notes (default)
#[test]
#[ignore = "TODO: Phase 2"]
fn baseline_notes_config() {
    let temp = project_with_config(r#"
[git]
baseline = "notes"
"#);
    git_init(&temp);
    // Add note to HEAD...

    cli().on(temp.path()).passes();
    // Assert reads from notes
}

/// > baseline = "<path>" uses file
#[test]
#[ignore = "TODO: Phase 3"]
fn baseline_file_config() {
    let temp = project_with_config(r#"
[git]
baseline = ".quench/baseline.json"
"#);

    // Write baseline file...
    cli().on(temp.path()).passes();
    // Assert reads from file, not notes
}
```

---

## Init Template Changes

### File: `docs/specs/templates/init.default.toml`

Update comments:

```toml
[git]
# Baseline source for ratcheting (default: git notes)
# Use "notes" for per-commit history, or a file path for committed baseline
# baseline = "notes"
# baseline = ".quench/baseline.json"
```

---

## Checklist

- [ ] Update `docs/specs/04-ratcheting.md` - baseline storage section
- [ ] Update `docs/specs/01-cli.md` - flag documentation
- [ ] Update `docs/specs/02-config.md` - `[git]` section
- [ ] Update `docs/specs/00-overview.md` - overview mention
- [ ] Add ignored tests in `tests/specs/modes/ratchet.rs`
- [ ] Update tests in `tests/specs/cli/ci_mode.rs`
- [ ] Create `tests/specs/config/git.rs` with ignored tests
- [ ] Update `docs/specs/templates/init.default.toml`

---

## Notes

- All new tests use `#[ignore = "TODO: Phase N"]` pattern per CLAUDE.md
- Existing passing tests should not be broken by doc changes
- Phase 2 implements reading from notes
- Phase 3 changes defaults and adds `--no-notes`
