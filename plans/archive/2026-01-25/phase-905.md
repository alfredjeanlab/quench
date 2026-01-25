# Phase 905: CI Mode Infrastructure

**Root Feature:** `quench-1230`
**Follows:** Phase 901 (CI Mode Specs)

## Overview

This phase implements the CI mode infrastructure for quench, enabling pipeline integration with metrics persistence and benchmark regression tracking. Building on the specs written in Phase 901, this phase adds:

1. **`--save FILE` flag** - Write check metrics to a JSON file
2. **`--save-notes` flag** - Store metrics in git notes (`refs/notes/quench`)
3. **Benchmark regression detection** - Fail CI if performance regresses >20% from baseline

The core CI mode behaviors (slow checks enabled, violation limit disabled, base branch auto-detection) are already implemented. This phase focuses on metrics persistence and regression gating.

## Project Structure

```
crates/cli/src/
├── cli.rs              # MODIFY: Add --save, --save-notes flags
├── cmd_check.rs        # MODIFY: Implement save functionality
├── git.rs              # MODIFY: Add git notes functions
├── git_tests.rs        # MODIFY: Add git notes tests
├── ratchet.rs          # MODIFY: Add regression threshold check
└── ratchet_tests.rs    # MODIFY: Add regression threshold tests
tests/specs/cli/
└── ci_mode.rs          # MODIFY: Remove #[ignore] from implemented specs
docs/specs/
└── 01-cli.md           # MODIFY: Document save flags and regression behavior
```

## Dependencies

No new external dependencies. Uses existing:
- `git2` - Git notes operations (already in Cargo.toml)
- `serde_json` - JSON serialization for metrics (already in Cargo.toml)
- `chrono` - Timestamps in saved metrics (already in Cargo.toml)

## Implementation Phases

### Phase 1: Add CLI Flags

**Goal**: Add `--save` and `--save-notes` flags to `CheckArgs`.

**File**: `crates/cli/src/cli.rs`

```rust
// Add to CheckArgs struct after line 102 (after `timing` flag):

/// Save metrics to file (CI mode)
#[arg(long, value_name = "FILE")]
pub save: Option<PathBuf>,

/// Save metrics to git notes (refs/notes/quench)
#[arg(long)]
pub save_notes: bool,
```

**Verification**:
```bash
cargo build
quench check --help | grep -E "(save|save-notes)"
```

---

### Phase 2: Implement File Save

**Goal**: Write check output as JSON to the specified file path.

**File**: `crates/cli/src/cmd_check.rs`

Add after the output formatting section (around line 517):

```rust
// Save metrics to file if requested
if let Some(ref save_path) = args.save {
    save_metrics_to_file(save_path, &output)?;
}

// Helper function (add at end of file):
fn save_metrics_to_file(
    path: &Path,
    output: &CheckOutput,
) -> anyhow::Result<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Serialize and write
    let json = serde_json::to_string_pretty(output)?;
    std::fs::write(path, json)?;

    Ok(())
}
```

**Specs to enable** (remove `#[ignore]`):
- `save_writes_metrics_to_file`
- `save_creates_parent_directories`
- `save_works_only_with_ci_mode`

**Verification**:
```bash
cargo test --test specs -- save_writes
cargo test --test specs -- save_creates
```

---

### Phase 3: Implement Git Notes Save

**Goal**: Store metrics in git notes under `refs/notes/quench`.

**File**: `crates/cli/src/git.rs`

Add new functions:

```rust
/// Save content to git notes for HEAD commit.
///
/// Uses `refs/notes/quench` namespace to avoid conflicts with other tools.
pub fn save_to_git_notes(root: &Path, content: &str) -> anyhow::Result<()> {
    let repo = Repository::discover(root)
        .context("Failed to open repository")?;

    let head_commit = repo.head()
        .context("Failed to get HEAD")?
        .peel_to_commit()
        .context("HEAD is not a commit")?;

    let sig = repo.signature()
        .or_else(|_| git2::Signature::now("quench", "quench@local"))?;

    // Create note blob
    let blob_oid = repo.blob(content.as_bytes())?;

    // Get or create notes ref
    let notes_ref = "refs/notes/quench";

    // Add note (this handles creating the ref if needed)
    repo.note(
        &sig,           // author
        &sig,           // committer
        Some(notes_ref),
        head_commit.id(),
        content,
        false,          // don't overwrite existing notes
    )?;

    Ok(())
}

/// Read git note for a specific commit.
pub fn read_git_note(root: &Path, commit_ref: &str) -> anyhow::Result<Option<String>> {
    let repo = Repository::discover(root)
        .context("Failed to open repository")?;

    let commit = repo.revparse_single(commit_ref)
        .context("Failed to resolve commit ref")?
        .peel_to_commit()
        .context("Ref is not a commit")?;

    let notes_ref = "refs/notes/quench";

    match repo.find_note(Some(notes_ref), commit.id()) {
        Ok(note) => Ok(note.message().map(|s| s.to_string())),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e).context("Failed to read git note"),
    }
}
```

**File**: `crates/cli/src/cmd_check.rs`

Add after file save section:

```rust
// Save metrics to git notes if requested
if args.save_notes {
    if !is_git_repo(&root) {
        eprintln!("quench: error: --save-notes requires a git repository");
        return Ok(ExitCode::ConfigError);
    }

    let json = serde_json::to_string(&output)?;
    if let Err(e) = save_to_git_notes(&root, &json) {
        eprintln!("quench: warning: failed to save to git notes: {}", e);
    } else if args.verbose {
        eprintln!("Saved metrics to git notes (refs/notes/quench)");
    }
}
```

**Specs to enable** (remove `#[ignore]`):
- `save_notes_writes_to_git`
- `save_notes_fails_without_git`
- `save_notes_uses_quench_namespace`

**Verification**:
```bash
cargo test --test specs -- save_notes
```

---

### Phase 4: Add Regression Threshold

**Goal**: Fail CI if timing metrics regress more than 20% from baseline.

**File**: `crates/cli/src/config.rs`

Add to `RatchetConfig`:

```rust
/// Maximum allowed regression percentage for CI mode (default: 20%)
#[serde(default = "default_regression_threshold")]
pub regression_threshold_percent: u32,

fn default_regression_threshold() -> u32 {
    20
}
```

**File**: `crates/cli/src/ratchet.rs`

Modify `compare_timing` to use percentage threshold:

```rust
/// Compare a timing metric against baseline with percentage threshold.
fn compare_timing(
    name: &str,
    current: Option<Duration>,
    baseline: Option<f64>,
    tolerance: Option<Duration>,
    regression_threshold_percent: u32,  // New parameter
    comparisons: &mut Vec<MetricComparison>,
    improvements: &mut Vec<MetricImprovement>,
    passed: &mut bool,
) {
    if let (Some(curr), Some(base)) = (current, baseline) {
        let curr_secs = curr.as_secs_f64();

        // Calculate max allowed: baseline + max(absolute_tolerance, percentage_threshold)
        let tolerance_secs = tolerance.map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let percentage_tolerance = base * (regression_threshold_percent as f64 / 100.0);
        let max_allowed = base + tolerance_secs.max(percentage_tolerance);

        let comparison = MetricComparison {
            name: name.to_string(),
            current: curr_secs,
            baseline: base,
            tolerance: tolerance_secs.max(percentage_tolerance),
            threshold: max_allowed,
            passed: curr_secs <= max_allowed,
            improved: curr_secs < base,
        };

        if !comparison.passed {
            *passed = false;
        }
        // ... rest unchanged
    }
}
```

**Specs to add** to `tests/specs/cli/ci_mode.rs`:

```rust
// =============================================================================
// BENCHMARK REGRESSION TRACKING
// =============================================================================

/// Spec: docs/specs/01-cli.md#ratchet-configuration
///
/// > CI mode fails if timing metrics regress >20% from baseline.
#[test]
#[ignore = "TODO: Phase 905 - Benchmark regression tracking"]
fn ci_mode_fails_on_timing_regression() {
    let temp = default_project();

    // Create a baseline with build time
    temp.file(".quench/baseline.json", r#"{
        "version": 1,
        "updated": "2024-01-01T00:00:00Z",
        "metrics": {
            "build_time": {"cold": 10.0, "hot": 5.0}
        }
    }"#);

    // TODO: Mock build check to return regressed timing
    // Result should fail due to >20% regression
}
```

**Verification**:
```bash
cargo test --test specs -- timing_regression
cargo test ratchet
```

---

### Phase 5: Enable Slow Check Specs

**Goal**: Verify existing CI mode slow check behavior passes specs.

The slow check behavior is already implemented:
- `crates/cli/src/checks/build/mod.rs` returns stub if `!ctx.ci_mode`
- `crates/cli/src/checks/stub.rs` handles license check similarly

**Specs to enable** (remove `#[ignore]`):
- `ci_mode_enables_build_check`
- `ci_mode_enables_license_check`
- `ci_mode_shows_all_violations`
- `default_mode_limits_violations`

**Verification**:
```bash
cargo test --test specs -- ci_mode_enables
cargo test --test specs -- ci_mode_shows
cargo test --test specs -- default_mode_limits
```

---

### Phase 6: Enable Base Detection Specs

**Goal**: Verify existing base branch detection behavior passes specs.

The base detection is already implemented in `git.rs::detect_base_branch()`.

**Specs to enable** (remove `#[ignore]`):
- `ci_mode_auto_detects_main_branch`
- `ci_mode_falls_back_to_master`

**Verification**:
```bash
cargo test --test specs -- ci_mode_auto_detects
cargo test --test specs -- ci_mode_falls_back
```

---

## Key Implementation Details

### Metrics JSON Structure

The `--save` flag writes the full `CheckOutput` structure:

```json
{
  "timestamp": "2024-01-25T12:00:00Z",
  "passed": true,
  "checks": [
    {
      "name": "cloc",
      "passed": true,
      "violations": [],
      "metrics": { "total": 1000, "ratio": 0.85 }
    }
  ]
}
```

### Git Notes Namespace

Using `refs/notes/quench` to:
1. Avoid conflicts with default git notes (`refs/notes/commits`)
2. Allow separate push/fetch of quench notes
3. Enable easy cleanup: `git notes --ref=quench prune`

To sync notes between remotes:
```bash
git push origin refs/notes/quench
git fetch origin refs/notes/quench:refs/notes/quench
```

### Regression Threshold Logic

For timing metrics (build_time, test_time):
- Default threshold: 20%
- Configurable via `quench.toml`:
  ```toml
  [ratchet]
  regression_threshold_percent = 15  # Stricter: 15%
  ```
- Applied per-metric: `max_allowed = baseline * 1.20`
- Absolute tolerance (if configured) takes precedence if larger

### CI Mode Flag Interactions

| Flag | Effect |
|------|--------|
| `--ci` | Enables slow checks (build, license) |
| `--ci` | Disables violation limit (shows all) |
| `--ci` | Auto-detects base branch (main > master) |
| `--save FILE` | Writes metrics JSON to FILE |
| `--save-notes` | Stores metrics in git notes |
| Both save flags | Can be used together |

---

## Verification Plan

### Per-Phase Verification

| Phase | Command | Expected |
|-------|---------|----------|
| 1 | `quench check --help` | Shows --save and --save-notes |
| 2 | `quench check --save /tmp/m.json && cat /tmp/m.json` | Valid JSON |
| 3 | `quench check --save-notes && git notes --ref=quench show HEAD` | Valid JSON |
| 4 | `cargo test ratchet::compare` | Regression threshold tests pass |
| 5 | `cargo test --test specs -- ci_mode_enables` | Specs pass |
| 6 | `cargo test --test specs -- ci_mode_auto` | Specs pass |

### Final Verification

```bash
# Full test suite
make check

# Count enabled specs (should show Phase 901 specs as passing)
cargo test --test specs -- ci_mode 2>&1 | grep -E "(PASSED|FAILED)"

# Integration test: full CI mode flow
quench check --ci --save /tmp/metrics.json --save-notes -v
cat /tmp/metrics.json
git notes --ref=quench show HEAD
```

### Success Criteria

1. **All existing tests pass**: `cargo test --all` exits 0
2. **No clippy warnings**: `cargo clippy` clean
3. **Save flags work**: `--save` creates valid JSON, `--save-notes` creates git note
4. **Regression detection works**: >20% timing regression fails in CI mode
5. **All Phase 901 specs pass**: No more `#[ignore]` on CI mode specs

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1. CLI Flags | Very Low | Simple clap additions |
| 2. File Save | Low | Straightforward file I/O |
| 3. Git Notes | Medium | git2 notes API less familiar; add comprehensive tests |
| 4. Regression | Medium | Threshold logic needs careful testing for edge cases |
| 5. Slow Checks | Low | Already implemented, just enabling specs |
| 6. Base Detection | Low | Already implemented, just enabling specs |

---

## Summary

| Phase | Deliverable | Lines Changed (est.) |
|-------|-------------|---------------------|
| 1 | CLI flags (`--save`, `--save-notes`) | ~10 |
| 2 | File save implementation | ~30 |
| 3 | Git notes save implementation | ~60 |
| 4 | Regression threshold check | ~40 |
| 5 | Enable slow check specs | ~10 (remove `#[ignore]`) |
| 6 | Enable base detection specs | ~5 (remove `#[ignore]`) |

**Total estimated lines**: ~155

---

## Completion Criteria

- [ ] Phase 1: `--save` and `--save-notes` flags added to CLI
- [ ] Phase 2: `--save FILE` writes valid JSON metrics
- [ ] Phase 3: `--save-notes` stores metrics in git notes
- [ ] Phase 4: Regression threshold fails CI on >20% slowdown
- [ ] Phase 5: `ci_mode_enables_*` specs pass
- [ ] Phase 6: `ci_mode_auto_detects_*` specs pass
- [ ] All Phase 901 specs pass (no `#[ignore]`)
- [ ] `make check` passes
- [ ] `./done` executed successfully
