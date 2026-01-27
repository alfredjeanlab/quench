# Plan 3: Git Notes Default - Switch Defaults & Add latest.json

**Date:** 2026-01-27
**Scope:** Make git notes the default, add `.quench/latest.json` cache, deprecate auto-write to `baseline.json`

## Summary

Complete the transition to git notes as the default baseline storage:
1. Git notes becomes the default comparison source
2. `.quench/latest.json` caches the most recent metrics locally (always written)
3. `.quench/baseline.json` becomes opt-in for teams preferring committed baselines
4. Add `--no-notes` flag to disable git notes mode

## Motivation

**Why `.quench/latest.json`?**
- Local cache for quick metric viewing without git operations
- Enables `quench report` without requiring git notes fetch
- Named "latest" to distinguish from "baseline" (which implies comparison target)

**Why keep file-based baseline option?**
- Some teams prefer committed, reviewable baseline files
- CI systems may not preserve git notes between runs
- Simpler mental model for smaller teams

## Implementation

### 1. Add `.quench/` to Default `.gitignore`

**Behavior:** `quench init` should add `.quench/` to `.gitignore` by default.

**File:** `crates/cli/src/cmd_init.rs` (or equivalent)

```rust
const DEFAULT_GITIGNORE_ENTRIES: &[&str] = &[
    ".quench/",
];

fn ensure_gitignored(root: &Path) -> anyhow::Result<()> {
    let gitignore = root.join(".gitignore");
    // Append .quench/ if not already present
}
```

### 2. Add `--no-notes` Flag

**File:** `crates/cli/src/cli.rs`

```rust
#[derive(Parser)]
pub struct CheckArgs {
    // ... existing fields ...

    /// Disable git notes; use file-based baseline only.
    #[arg(long)]
    pub no_notes: bool,

    // Remove --save-notes (now default behavior)
    // #[arg(long)]
    // pub save_notes: bool,
}
```

**Deprecation:** Keep `--save-notes` as hidden alias for one release cycle:

```rust
    /// [DEPRECATED] Git notes are now the default. This flag is ignored.
    #[arg(long, hide = true)]
    pub save_notes: bool,
```

### 3. Always Write `.quench/latest.json`

**File:** `crates/cli/src/cmd_check.rs`

After check completion (regardless of `--fix`):

```rust
// Always write latest.json for local caching
let latest_path = root.join(".quench/latest.json");
let latest = LatestMetrics {
    updated: Utc::now(),
    commit: get_head_commit(&root).ok(),
    output: output.clone(),
};
if let Err(e) = latest.save(&latest_path) {
    if debug_logging() {
        eprintln!("quench: debug: failed to write latest.json: {}", e);
    }
}
```

**New struct:** `crates/cli/src/latest.rs`

```rust
//! Latest metrics cache for local viewing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::check::CheckOutput;

/// Latest metrics cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMetrics {
    pub updated: DateTime<Utc>,
    pub commit: Option<String>,
    pub output: CheckOutput,
}

impl LatestMetrics {
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }
}
```

### 4. Update `--fix` Behavior

**File:** `crates/cli/src/cmd_check.rs`

```rust
if args.fix {
    let current = CurrentMetrics::from_output(&output);
    let mut baseline = baseline
        .map(|b| b.with_commit(&root))
        .unwrap_or_else(|| Baseline::new().with_commit(&root));

    ratchet::update_baseline(&mut baseline, &current);

    // Determine save target based on config and flags
    let use_notes = config.git.uses_notes() && !args.no_notes && is_git_repo(&root);

    if use_notes {
        // Default: save to git notes
        let json = serde_json::to_string_pretty(&baseline)?;
        match save_to_git_notes(&root, &json) {
            Ok(()) => report_baseline_update(&ratchet_result, "git notes"),
            Err(e) => eprintln!("quench: warning: failed to save to git notes: {}", e),
        }
    }

    // Also save to file if explicitly configured
    if let Some(path) = config.git.baseline_path() {
        let baseline_path = root.join(path);
        let baseline_existed = baseline_path.exists();
        if let Err(e) = baseline.save(&baseline_path) {
            eprintln!("quench: warning: failed to save baseline: {}", e);
        } else if !use_notes {
            // Only report file update if not using notes
            report_baseline_update_file(&ratchet_result, &baseline_path, baseline_existed);
        }
    }
}
```

### 5. Update Config Defaults

**File:** `crates/cli/src/config/mod.rs`

```rust
impl GitConfig {
    fn default_baseline() -> String {
        "notes".to_string()
    }
}
```

### 6. Update `quench report` to Use `latest.json`

**File:** `crates/cli/src/cmd_report.rs`

```rust
pub fn run(args: ReportArgs, config: &Config) -> anyhow::Result<()> {
    let root = &config.root;

    // Try sources in order:
    // 1. Explicit --baseline flag
    // 2. .quench/latest.json (local cache)
    // 3. Git notes for HEAD
    // 4. Configured baseline file

    let baseline = if let Some(ref path) = args.baseline {
        Baseline::load(&root.join(path))?
            .ok_or_else(|| anyhow::anyhow!("Baseline not found at {}", path))?
    } else {
        load_latest_or_baseline(root, config)?
    };

    // ... render report ...
}

fn load_latest_or_baseline(root: &Path, config: &Config) -> anyhow::Result<Baseline> {
    // Try latest.json first
    let latest_path = root.join(".quench/latest.json");
    if let Some(latest) = LatestMetrics::load(&latest_path)? {
        // Convert LatestMetrics to Baseline for report
        return Ok(latest.into_baseline());
    }

    // Try git notes
    if config.git.uses_notes() && is_git_repo(root) {
        if let Some(baseline) = Baseline::load_from_notes(root, "HEAD")? {
            return Ok(baseline);
        }
    }

    // Try baseline file
    if let Some(path) = config.git.baseline_path() {
        if let Some(baseline) = Baseline::load(&root.join(path))? {
            return Ok(baseline);
        }
    }

    Err(anyhow::anyhow!("No metrics found. Run 'quench check' first."))
}
```

---

## CI Workflow Updates

### Recommended (git notes):

```yaml
- name: Fetch notes
  run: git fetch origin refs/notes/quench:refs/notes/quench || true

- name: Check quality
  run: quench check --ci

- name: Update baseline on main
  if: github.ref == 'refs/heads/main'
  run: |
    quench check --ci --fix
    git push origin refs/notes/quench
```

### Alternative (file-based):

```yaml
# quench.toml
[git]
baseline = ".quench/baseline.json"
```

```yaml
- name: Check quality
  run: quench check --ci --no-notes

- name: Update baseline on main
  if: github.ref == 'refs/heads/main'
  run: |
    quench check --ci --fix --no-notes
    git add .quench/baseline.json
    git commit -m "chore: update quality baseline" || true
    git push
```

---

## Migration Guide

Add to `CHANGELOG.md`:

```markdown
## [X.Y.0] - 2026-XX-XX

### Changed

- **BREAKING:** Git notes are now the default baseline storage
  - Baselines are stored in `refs/notes/quench` per-commit
  - Previous behavior: `.quench/baseline.json` committed to repo
  - Migration: Add `[git] baseline = ".quench/baseline.json"` to keep file-based baseline

- `.quench/latest.json` is now written after every check for local caching
  - Add `.quench/` to `.gitignore` (done automatically by `quench init`)

### Deprecated

- `--save-notes` flag is deprecated (notes are now default)
  - Use `--no-notes` to opt out of git notes

### Added

- `--no-notes` flag to disable git notes and use file-based baseline only

### Enhanced

- `--base <REF>` now also determines which commit's note to use for ratchet comparison
```

---

## Checklist

- [ ] Add `--no-notes` flag to CLI
- [ ] Deprecate `--save-notes` flag (keep as hidden no-op)
- [ ] Create `latest.rs` with `LatestMetrics` struct
- [ ] Update `cmd_check.rs` to always write `latest.json`
- [ ] Update `cmd_check.rs` `--fix` to use notes by default
- [ ] Update `cmd_report.rs` to read from `latest.json` first
- [ ] Update `cmd_init.rs` to add `.quench/` to `.gitignore`
- [ ] Change default in `GitConfig::default_baseline()` to `"notes"`
- [ ] Update CI workflow documentation
- [ ] Add migration guide to CHANGELOG
- [ ] Remove `#[ignore]` from Phase 3 tests in specs
- [ ] Run `make check`

---

## Dependencies

- **Requires:** Plan 2 (notes-based ratcheting implementation)

---

## Rollback Plan

If issues arise, users can immediately revert by adding to `quench.toml`:

```toml
[git]
baseline = ".quench/baseline.json"
```

This restores the previous file-based behavior with no code changes needed.
