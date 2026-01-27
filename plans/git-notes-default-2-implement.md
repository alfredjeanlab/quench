# Plan 2: Git Notes Default - Implement Notes-Based Ratcheting

**Date:** 2026-01-27
**Scope:** Implement baseline reading from git notes for ratchet comparison

## Summary

Add the ability to read baseline metrics from git notes when comparing ratchet metrics. This enables per-commit metric history without requiring file commits.

## Current State

**Reading baseline:**
- `cmd_check.rs:391-421` - loads from `config.git.baseline` file path only
- `Baseline::load()` - reads JSON from file

**Writing baseline:**
- `--fix` writes to `config.git.baseline` file path
- `--save-notes` writes to `refs/notes/quench` (separate from ratchet flow)

**Git notes functions (already exist):**
- `git::save_to_git_notes()` - writes note to HEAD
- `git::read_git_note()` - reads note for a commit ref

## Implementation

### 1. Extend `GitConfig` to Support Notes Mode

**File:** `crates/cli/src/config/mod.rs`

```rust
/// Git configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitConfig {
    /// Baseline source: "notes" or file path.
    #[serde(default = "GitConfig::default_baseline")]
    pub baseline: String,
    // ...
}

impl GitConfig {
    fn default_baseline() -> String {
        "notes".to_string()  // Changed from ".quench/baseline.json"
    }

    /// Check if baseline is configured to use git notes.
    pub fn uses_notes(&self) -> bool {
        self.baseline == "notes"
    }

    /// Get baseline file path (returns None if using notes mode).
    pub fn baseline_path(&self) -> Option<&str> {
        if self.uses_notes() {
            None
        } else {
            Some(&self.baseline)
        }
    }
}
```

### 2. Add Baseline Loading from Notes

**File:** `crates/cli/src/baseline.rs`

```rust
use crate::git::read_git_note;

impl Baseline {
    /// Load baseline from git notes for a specific commit.
    ///
    /// Returns None if no note exists for the commit.
    pub fn load_from_notes(root: &Path, commit_ref: &str) -> Result<Option<Self>, BaselineError> {
        let note_content = read_git_note(root, commit_ref)
            .map_err(|e| BaselineError::Read(e.to_string()))?;

        match note_content {
            Some(content) => {
                let baseline: Baseline = serde_json::from_str(&content)
                    .map_err(|e| BaselineError::Parse(e.to_string()))?;

                if baseline.version > BASELINE_VERSION {
                    return Err(BaselineError::Version {
                        found: baseline.version,
                        supported: BASELINE_VERSION,
                    });
                }

                Ok(Some(baseline))
            }
            None => Ok(None),
        }
    }
}
```

### 3. Determine Comparison Base Commit

**File:** `crates/cli/src/git.rs`

```rust
/// Find the merge-base commit for ratchet comparison.
///
/// If --base is provided, uses that ref.
/// Otherwise, finds merge-base with main/master.
/// Falls back to HEAD~1 if no merge-base found.
pub fn find_ratchet_base(root: &Path, base_ref: Option<&str>) -> anyhow::Result<String> {
    let repo = Repository::discover(root)?;

    if let Some(base_ref) = base_ref {
        // Explicit --base REF provided
        let commit = repo.revparse_single(base_ref)?
            .peel_to_commit()?;
        return Ok(commit.id().to_string());
    }

    // Try to find merge-base with main branch
    let head = repo.head()?.peel_to_commit()?;

    for main_branch in &["origin/main", "origin/master", "main", "master"] {
        if let Ok(main_ref) = repo.revparse_single(main_branch) {
            if let Ok(main_commit) = main_ref.peel_to_commit() {
                if let Ok((base, _)) = repo.merge_base_many(&[head.id(), main_commit.id()]) {
                    return Ok(base.to_string());
                }
            }
        }
    }

    // Fallback: use HEAD~1 (parent commit)
    if let Some(parent) = head.parent(0).ok() {
        return Ok(parent.id().to_string());
    }

    // Last resort: HEAD itself (initial commit)
    Ok(head.id().to_string())
}
```

### 4. Update `cmd_check.rs` Baseline Loading

**File:** `crates/cli/src/cmd_check.rs`

Replace baseline loading logic (~line 390-424):

```rust
// Ratchet checking (uses --base ref for baseline note lookup)
let (ratchet_result, baseline) = if config.ratchet.check != CheckLevel::Off {
    load_baseline_for_ratchet(&root, &config, args.base.as_deref())?
} else {
    (None, None)
};

// ... later in the file ...

fn load_baseline_for_ratchet(
    root: &Path,
    config: &Config,
    base_ref: Option<&str>,  // from args.base
) -> anyhow::Result<(Option<RatchetResult>, Option<Baseline>)> {
    // Determine baseline source
    if config.git.uses_notes() && is_git_repo(root) {
        // Git notes mode (default)
        let base_commit = find_ratchet_base(root, base_ref)?;

        match Baseline::load_from_notes(root, &base_commit) {
            Ok(Some(baseline)) => {
                if baseline.is_stale(config.ratchet.stale_days) {
                    eprintln!(
                        "warning: baseline is {} days old. Consider refreshing with --fix.",
                        baseline.age_days()
                    );
                }
                let current = CurrentMetrics::from_output(&output);
                let result = ratchet::compare(&current, &baseline.metrics, &config.ratchet);
                Ok((Some(result), Some(baseline)))
            }
            Ok(None) => {
                if debug_logging() {
                    eprintln!(
                        "No baseline note found for {}. Run with --fix to create.",
                        base_commit
                    );
                }
                Ok((None, None))
            }
            Err(e) => {
                eprintln!("quench: warning: failed to load baseline from notes: {}", e);
                Ok((None, None))
            }
        }
    } else if let Some(path) = config.git.baseline_path() {
        // File-based baseline mode
        let baseline_path = root.join(path);
        match Baseline::load(&baseline_path) {
            Ok(Some(baseline)) => {
                if baseline.is_stale(config.ratchet.stale_days) {
                    eprintln!(
                        "warning: baseline is {} days old. Consider refreshing with --fix.",
                        baseline.age_days()
                    );
                }
                let current = CurrentMetrics::from_output(&output);
                let result = ratchet::compare(&current, &baseline.metrics, &config.ratchet);
                Ok((Some(result), Some(baseline)))
            }
            Ok(None) => {
                if debug_logging() {
                    eprintln!(
                        "No baseline found at {}. Run with --fix to create.",
                        baseline_path.display()
                    );
                }
                Ok((None, None))
            }
            Err(e) => {
                eprintln!("quench: warning: failed to load baseline: {}", e);
                Ok((None, None))
            }
        }
    } else {
        // Not in git repo and using notes mode - skip ratchet
        if debug_logging() {
            eprintln!("Ratcheting requires git repository when using notes mode.");
        }
        Ok((None, None))
    }
}
```

### 5. Use Existing `--base` Flag for Baseline Lookup

The existing `--base <REF>` flag (already spec'd in `docs/specs/01-cli.md`) should be used for baseline note lookup. When `--base main` is specified, ratchet comparison uses the baseline from that commit's note.

**No new CLI flag needed** - reuse `args.base` in the baseline loading logic.

### 6. Update `--fix` to Write Notes by Default

**File:** `crates/cli/src/cmd_check.rs`

Update the `--fix` handling:

```rust
if args.fix {
    let current = CurrentMetrics::from_output(&output);
    let mut baseline = baseline
        .map(|b| b.with_commit(&root))
        .unwrap_or_else(|| Baseline::new().with_commit(&root));

    ratchet::update_baseline(&mut baseline, &current);

    // Save based on config
    if config.git.uses_notes() && is_git_repo(&root) {
        // Save to git notes (default)
        let json = serde_json::to_string_pretty(&baseline)?;
        if let Err(e) = save_to_git_notes(&root, &json) {
            eprintln!("quench: warning: failed to save to git notes: {}", e);
        } else {
            report_baseline_update(&ratchet_result, "git notes");
        }
    } else if let Some(path) = config.git.baseline_path() {
        // Save to file
        let baseline_path = root.join(path);
        let baseline_existed = baseline_path.exists();
        if let Err(e) = baseline.save(&baseline_path) {
            eprintln!("quench: warning: failed to save baseline: {}", e);
        } else {
            report_baseline_update_file(&ratchet_result, &baseline_path, baseline_existed);
        }
    }
}
```

---

## Unit Tests

**File:** `crates/cli/src/baseline_tests.rs`

```rust
#[test]
fn load_from_notes_parses_valid_json() {
    // Setup temp git repo with note
    // Assert baseline loads correctly
}

#[test]
fn load_from_notes_returns_none_for_missing_note() {
    // Setup temp git repo without note
    // Assert returns Ok(None)
}

#[test]
fn load_from_notes_rejects_future_version() {
    // Setup note with version > BASELINE_VERSION
    // Assert returns Version error
}
```

**File:** `crates/cli/src/git_tests.rs`

```rust
#[test]
fn find_ratchet_base_uses_base_ref() {
    // Assert returns the --base ref when provided
}

#[test]
fn find_ratchet_base_finds_merge_base() {
    // Setup repo with feature branch
    // Assert returns merge-base with main
}

#[test]
fn find_ratchet_base_falls_back_to_parent() {
    // Setup repo with no remote
    // Assert returns HEAD~1
}
```

---

## Migration Notes

- Existing `baseline = ".quench/baseline.json"` configs continue to work
- New projects default to `baseline = "notes"`
- No breaking changes - file-based baseline is still supported

---

## Checklist

- [ ] Add `uses_notes()` and `baseline_path()` to `GitConfig`
- [ ] Add `Baseline::load_from_notes()` in `baseline.rs`
- [ ] Add `find_ratchet_base()` in `git.rs`
- [ ] Refactor baseline loading in `cmd_check.rs` to use `args.base` for note lookup
- [ ] Update `--fix` to write notes by default
- [ ] Add unit tests for new functions
- [ ] Remove `#[ignore]` from Phase 2 tests in specs
- [ ] Run `make check`
- [ ] Bump `CACHE_VERSION` if needed

---

## Dependencies

- **Requires:** Plan 1 (docs/specs updates with ignored tests)
- **Blocks:** Plan 3 (making notes the default)
