# Phase 901: CI Mode - Specs

**Root Feature:** `quench-5ed5`
**Follows:** checkpoint-10h-techdebt (Tech Debt cleanup)

## Overview

This phase establishes behavioral specifications for CI mode functionality. CI mode (`--ci`) is designed for pipeline integration, enabling slow checks, disabling output limits, auto-detecting base branches, and providing metrics persistence. The specs validate that the CLI correctly implements the behaviors documented in `docs/specs/01-cli.md`.

Additionally, this phase retires the legacy "bootstrap test convention check" which was previously used to detect literal `#[cfg(test)]` strings in non-test files. This check has been superseded by the escapes pattern system.

## Project Structure

```
tests/specs/
├── cli/
│   ├── ci_mode.rs          # CREATE: CI mode behavioral specs
│   └── mod.rs              # MODIFY: Add ci_mode module
├── modes/
│   └── mod.rs              # EXISTS: May need updates for save tests
crates/cli/
├── benches/
│   ├── tests.rs            # MODIFY: Remove bootstrap check workaround comments
│   ├── stress.rs           # MODIFY: Remove bootstrap check workaround comments
│   └── adapter.rs          # MODIFY: Remove bootstrap check workaround comments
├── src/
│   ├── cli.rs              # MODIFY: Add --save, --save-notes flags (Phase 5-6)
│   └── cmd_check.rs        # MODIFY: Implement save functionality (Phase 5-6)
tests/fixtures/
└── ci-mode/                # CREATE: Fixture for CI mode tests
    ├── quench.toml
    ├── CLAUDE.md
    └── src/lib.rs
```

## Dependencies

No new external dependencies. Uses existing:
- `tempfile` for temp directories in tests
- `git2` or git CLI for base branch detection tests
- Existing test harness in `tests/specs/prelude.rs`

## Implementation Phases

### Phase 1: Retire Bootstrap Test Convention Check

**Goal**: Remove references to the legacy "bootstrap check" that detected `#[cfg(test)]` in non-test files.

**Context**: The benchmark files contain workaround comments like:
```rust
// Avoid literal cfg(test) to bypass bootstrap check
let cfg_test_attr = concat!("#[cfg", "(test)]");
```

This was a legacy check that has been replaced by the escapes pattern system. The workarounds are no longer necessary but remain in the code.

**Tasks**:

1. Update benchmarks to remove workaround comments:
   - `crates/cli/benches/tests.rs:268` - Remove comment, use literal `#[cfg(test)]`
   - `crates/cli/benches/stress.rs:36,68` - Remove comments, use literals
   - `crates/cli/benches/adapter.rs:298,347,431` - Remove comments, use literals

2. Verify no bootstrap script exists (already deleted):
   ```bash
   ls scripts/bootstrap*  # Should show no files
   ```

**Example cleanup**:

```rust
// Before:
// Avoid literal cfg(test) to bypass bootstrap check
let cfg_test_attr = concat!("#[cfg", "(test)]");

// After:
let cfg_test_attr = "#[cfg(test)]";
```

**Verification**:
```bash
cargo build --benches
cargo test --test specs
```

---

### Phase 2: Spec - CI Mode Enables Slow Checks

**Goal**: Verify `--ci` enables build and license checks that are slow/skipped by default.

**Reference**: `docs/specs/01-cli.md#check-toggles`

> **CI mode**: `--ci` flag, enables slow checks (build, license).

**Create**: `tests/specs/cli/ci_mode.rs`

```rust
//! Behavioral specs for CI mode.
//!
//! Tests that quench correctly handles:
//! - --ci enables slow checks (build, license)
//! - --ci disables violation limit
//! - --ci auto-detects base branch
//!
//! Reference: docs/specs/01-cli.md#scope-flags

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SLOW CHECKS ENABLED
// =============================================================================

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > CI mode (`--ci`) enables slow checks (build, license).
#[test]
#[ignore = "TODO: Phase 901 - Verify CI mode enables slow checks"]
fn ci_mode_enables_build_check() {
    let temp = default_project();

    // Without --ci, build check should be skipped
    let result = cli().pwd(temp.path()).args(&["--build"]).json().passes();
    let build = result.checks().iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_eq!(
        build.get("skipped").and_then(|s| s.as_bool()),
        Some(true),
        "build check should be skipped without --ci"
    );

    // With --ci, build check should run
    let result = cli().pwd(temp.path()).args(&["--ci", "--build"]).json().passes();
    let build = result.checks().iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_ne!(
        build.get("skipped").and_then(|s| s.as_bool()),
        Some(true),
        "build check should run with --ci"
    );
}

/// Spec: docs/specs/01-cli.md#check-toggles
///
/// > CI mode (`--ci`) enables slow checks (build, license).
#[test]
#[ignore = "TODO: Phase 901 - Verify CI mode enables slow checks"]
fn ci_mode_enables_license_check() {
    let temp = default_project();

    // Without --ci, license check should be skipped
    let result = cli().pwd(temp.path()).args(&["--license"]).json().passes();
    let license = result.checks().iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("license"))
        .expect("license check should exist");
    assert_eq!(
        license.get("skipped").and_then(|s| s.as_bool()),
        Some(true),
        "license check should be skipped without --ci"
    );

    // With --ci, license check should run
    let result = cli().pwd(temp.path()).args(&["--ci", "--license"]).json().passes();
    let license = result.checks().iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("license"))
        .expect("license check should exist");
    assert_ne!(
        license.get("skipped").and_then(|s| s.as_bool()),
        Some(true),
        "license check should run with --ci"
    );
}
```

**Verification**:
```bash
cargo test --test specs -- ci_mode --ignored  # Shows as TODO
```

---

### Phase 3: Spec - CI Mode Disables Violation Limit

**Goal**: Verify `--ci` disables the default 15 violation limit.

**Reference**: `docs/specs/01-cli.md#output-flags`

> **Violation Limit**: By default, quench shows at most **15 violations**...
> CI mode implicitly sets `--no-limit`.

**Add to**: `tests/specs/cli/ci_mode.rs`

```rust
// =============================================================================
// VIOLATION LIMIT DISABLED
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > CI mode implicitly disables the violation limit.
#[test]
#[ignore = "TODO: Phase 901 - Verify CI mode disables limit"]
fn ci_mode_shows_all_violations() {
    // Use fixture with >15 violations
    let result = cli()
        .on("ci-mode")
        .args(&["--ci"])
        .json()
        .fails();

    // Get total violations across all checks
    let total_violations: usize = result.checks().iter()
        .filter_map(|c| c.get("violations").and_then(|v| v.as_array()))
        .map(|v| v.len())
        .sum();

    assert!(
        total_violations > 15,
        "CI mode should show all violations (got {})",
        total_violations
    );
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Default limit is 15 violations (without --ci or --no-limit).
#[test]
#[ignore = "TODO: Phase 901 - Verify default limit works"]
fn default_mode_limits_violations() {
    // Use fixture with >15 violations
    let result = cli()
        .on("ci-mode")
        .json()
        .fails();

    // Violations should be capped at 15
    let total_violations: usize = result.checks().iter()
        .filter_map(|c| c.get("violations").and_then(|v| v.as_array()))
        .map(|v| v.len())
        .sum();

    assert!(
        total_violations <= 15,
        "default mode should limit to 15 violations (got {})",
        total_violations
    );
}
```

**Create fixture**: `tests/fixtures/ci-mode/`

```
tests/fixtures/ci-mode/
├── quench.toml           # Config with low limits to trigger many violations
├── CLAUDE.md             # Minimal agent file
└── src/
    └── lib.rs            # File that triggers >15 violations
```

**Fixture config** (`quench.toml`):
```toml
version = 1

[check.cloc]
max_lines = 5  # Very low limit to trigger violations

[check.agents]
required = ["CLAUDE.md"]
```

**Verification**:
```bash
cargo test --test specs -- ci_mode --ignored
```

---

### Phase 4: Spec - CI Mode Auto-Detects Base Branch

**Goal**: Verify `--ci` auto-detects the base branch (main > master > develop).

**Reference**: `docs/specs/01-cli.md#scope-flags`

> `--ci`: CI mode: slow checks + auto-detect base

**Add to**: `tests/specs/cli/ci_mode.rs`

```rust
// =============================================================================
// BASE BRANCH DETECTION
// =============================================================================

/// Spec: docs/specs/01-cli.md#scope-flags
///
/// > --ci auto-detects base branch (main > master > develop)
#[test]
#[ignore = "TODO: Phase 901 - Verify CI mode auto-detects base"]
fn ci_mode_auto_detects_main_branch() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    // Create a feature branch with changes
    git_branch(&temp, "feature");
    temp.file("src/new_file.rs", "// new file\n");
    git_commit(&temp, "feat: add new file");

    // CI mode should detect main as base and compare
    let result = cli()
        .pwd(temp.path())
        .args(&["--ci", "-v"])
        .passes();

    // Verbose output should mention the detected base
    result.stderr_has("main");
}

/// Spec: docs/specs/01-cli.md#scope-flags
///
/// > --ci falls back to master if main doesn't exist
#[test]
#[ignore = "TODO: Phase 901 - Verify CI mode falls back to master"]
fn ci_mode_falls_back_to_master() {
    let temp = default_project();
    git_init(&temp);

    // Rename main to master
    std::process::Command::new("git")
        .args(["branch", "-m", "master"])
        .current_dir(temp.path())
        .output()
        .expect("git branch rename should succeed");

    git_initial_commit(&temp);
    git_branch(&temp, "feature");
    temp.file("src/new_file.rs", "// new file\n");
    git_commit(&temp, "feat: add new file");

    // CI mode should detect master as base
    let result = cli()
        .pwd(temp.path())
        .args(&["--ci", "-v"])
        .passes();

    result.stderr_has("master");
}
```

**Verification**:
```bash
cargo test --test specs -- ci_mode --ignored
```

---

### Phase 5: Spec - Save Metrics to File

**Goal**: Verify `--save FILE` writes metrics to the specified file.

**Reference**: `docs/specs/01-cli.md#output-flags`

> `--save <FILE>`: Save metrics to file (CI mode)

**Add to**: `tests/specs/cli/ci_mode.rs`

```rust
// =============================================================================
// METRICS PERSISTENCE - FILE
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save <FILE> saves metrics to file
#[test]
#[ignore = "TODO: Phase 901 - Implement --save flag"]
fn save_writes_metrics_to_file() {
    let temp = default_project();
    let save_path = temp.path().join(".quench/metrics.json");

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save", save_path.to_str().unwrap()])
        .passes();

    // File should exist and contain valid JSON
    assert!(save_path.exists(), "metrics file should be created");

    let content = std::fs::read_to_string(&save_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("metrics file should be valid JSON");

    // Should have metrics structure
    assert!(json.get("checks").is_some(), "should have checks field");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save creates parent directories if needed
#[test]
#[ignore = "TODO: Phase 901 - Implement --save flag"]
fn save_creates_parent_directories() {
    let temp = default_project();
    let save_path = temp.path().join("deep/nested/path/metrics.json");

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save", save_path.to_str().unwrap()])
        .passes();

    assert!(save_path.exists(), "metrics file should be created with parents");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save requires --ci mode (or warn?)
#[test]
#[ignore = "TODO: Phase 901 - Implement --save flag"]
fn save_works_only_with_ci_mode() {
    let temp = default_project();
    let save_path = temp.path().join("metrics.json");

    // Without --ci, --save should still work but may warn
    cli()
        .pwd(temp.path())
        .args(&["--save", save_path.to_str().unwrap()])
        .passes();

    // Metrics should still be saved
    assert!(save_path.exists(), "--save should work without --ci");
}
```

**CLI flag addition** (`crates/cli/src/cli.rs`):

```rust
/// Save metrics to file (CI mode)
#[arg(long, value_name = "FILE")]
pub save: Option<PathBuf>,
```

**Verification**:
```bash
cargo test --test specs -- ci_mode --ignored
```

---

### Phase 6: Spec - Save Metrics to Git Notes

**Goal**: Verify `--save-notes` writes metrics to git notes.

**Reference**: `docs/specs/01-cli.md#output-flags`

> `--save-notes`: Save metrics to git notes (CI mode)

**Add to**: `tests/specs/cli/ci_mode.rs`

```rust
// =============================================================================
// METRICS PERSISTENCE - GIT NOTES
// =============================================================================

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save-notes stores metrics in git notes
#[test]
#[ignore = "TODO: Phase 901 - Implement --save-notes flag"]
fn save_notes_writes_to_git() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes"])
        .passes();

    // Git notes should be created for HEAD
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "show", "HEAD"])
        .current_dir(temp.path())
        .output()
        .expect("git notes show should succeed");

    assert!(output.status.success(), "git notes should exist for HEAD");

    let content = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("git note should be valid JSON");

    assert!(json.get("checks").is_some(), "should have checks field");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save-notes requires git repository
#[test]
#[ignore = "TODO: Phase 901 - Implement --save-notes flag"]
fn save_notes_fails_without_git() {
    let temp = default_project();
    // No git init

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes"])
        .exits(2)
        .stderr_has("not a git repository");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > --save-notes uses refs/notes/quench namespace
#[test]
#[ignore = "TODO: Phase 901 - Implement --save-notes flag"]
fn save_notes_uses_quench_namespace() {
    let temp = default_project();
    git_init(&temp);
    git_initial_commit(&temp);

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--save-notes"])
        .passes();

    // Check that refs/notes/quench exists
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "list"])
        .current_dir(temp.path())
        .output()
        .expect("git notes list should succeed");

    assert!(output.status.success(), "quench notes ref should exist");
    assert!(!output.stdout.is_empty(), "should have at least one note");
}
```

**CLI flag addition** (`crates/cli/src/cli.rs`):

```rust
/// Save metrics to git notes (CI mode)
#[arg(long)]
pub save_notes: bool,
```

**Verification**:
```bash
cargo test --test specs -- ci_mode --ignored
```

---

## Key Implementation Details

### CI Mode Flag Interactions

| Flag | Without `--ci` | With `--ci` |
|------|---------------|-------------|
| Build check | Skipped | Runs |
| License check | Skipped | Runs |
| Violation limit | 15 (default) | Unlimited |
| Base detection | Manual (`--base`) | Auto-detect |
| `--save` | Allowed | Intended use |
| `--save-notes` | Allowed | Intended use |

### Base Branch Detection Order

1. `main` (GitHub default)
2. `master` (legacy default)
3. `develop` (GitFlow)
4. First remote tracking branch
5. `None` (no comparison)

### Git Notes Namespace

Using `refs/notes/quench` to avoid conflicts with other tools:

```bash
git notes --ref=quench add -m '{"checks": [...]}' HEAD
git notes --ref=quench show HEAD
git push origin refs/notes/quench
```

### Fixture Design for Limit Testing

The `ci-mode` fixture should have a file structure that triggers >15 violations:

```
ci-mode/
├── quench.toml     # max_lines = 5
├── CLAUDE.md
└── src/
    ├── file1.rs    # 10 lines (5 violations each = many files)
    ├── file2.rs
    ├── file3.rs
    └── ... (enough files to exceed 15 violations)
```

---

## Verification Plan

### Per-Phase Verification

| Phase | Command | Expected |
|-------|---------|----------|
| 1 | `cargo build --benches` | Benchmarks compile with literal `#[cfg(test)]` |
| 2 | `cargo test --test specs -- ci_mode_enables` | Specs exist as ignored TODOs |
| 3 | `cargo test --test specs -- ci_mode_shows` | Specs exist as ignored TODOs |
| 4 | `cargo test --test specs -- ci_mode_auto` | Specs exist as ignored TODOs |
| 5 | `cargo test --test specs -- save_writes` | Specs exist as ignored TODOs |
| 6 | `cargo test --test specs -- save_notes` | Specs exist as ignored TODOs |

### Final Verification

```bash
# Full test suite
make check

# Count ignored specs (should show new CI mode specs)
cargo test --test specs -- --ignored 2>&1 | grep "TODO: Phase 901"

# Verify benchmarks still work
cargo bench --bench tests -- --test

# Verify no bootstrap check references remain
grep -r "bootstrap check" crates/cli/benches/ && echo "FAIL: cleanup needed" || echo "OK: no references"
```

### Success Criteria

1. **All existing tests pass**: `cargo test --all` exits 0
2. **No clippy warnings**: `cargo clippy` clean
3. **Benchmark cleanup complete**: No workaround comments for bootstrap check
4. **New specs exist**: 10+ new `#[ignore]` specs for CI mode
5. **Fixture created**: `tests/fixtures/ci-mode/` exists with proper structure

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| 1. Bootstrap Cleanup | Very Low | Simple comment removal, benchmarks still work |
| 2. Slow Checks Spec | Low | Specs are ignored TODOs, no implementation |
| 3. Limit Spec | Low | Fixture creation is straightforward |
| 4. Base Detection Spec | Medium | Git operations in tests need careful handling |
| 5. Save File Spec | Low | Building on existing baseline.rs patterns |
| 6. Save Notes Spec | Medium | Git notes API less familiar, need careful testing |

---

## Summary

| Phase | Deliverable | Purpose |
|-------|-------------|---------|
| 1 | Benchmark cleanup | Remove legacy bootstrap workarounds |
| 2 | `ci_mode_enables_*` specs | Document slow check behavior |
| 3 | `ci_mode_shows_all_violations` | Document limit behavior |
| 4 | `ci_mode_auto_detects_*` specs | Document base detection |
| 5 | `save_writes_*` specs | Document file persistence |
| 6 | `save_notes_*` specs | Document git notes persistence |

---

## Completion Criteria

- [ ] Phase 1: Bootstrap check workarounds removed from benchmarks
- [ ] Phase 2: Slow check specs written (ignored)
- [ ] Phase 3: Limit behavior specs written (ignored)
- [ ] Phase 4: Base detection specs written (ignored)
- [ ] Phase 5: File save specs written (ignored)
- [ ] Phase 6: Git notes specs written (ignored)
- [ ] CI mode fixture created
- [ ] `make check` passes
- [ ] `./done` executed successfully
