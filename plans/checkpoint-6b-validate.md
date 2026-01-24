# Checkpoint 6B: Dogfooding Milestone 1 - Validation

**Root Feature:** `quench-2bcc`

## Overview

This checkpoint validates that quench can successfully lint itself (dogfooding) and that the agents check properly detects all violation types. The work involves:

1. Running `quench check` on the quench codebase with all enabled checks (cloc, escapes, agents)
2. Fixing any violations found
3. Completing the `quench.toml` configuration for the quench project
4. Adding snapshot tests for agents output
5. Documenting the dogfooding experience in `reports/dogfooding-milestone-1.md`

## Project Structure

```
quench/
├── quench.toml                    # Extend with agents config (Phase 1)
├── CLAUDE.md                      # Existing - verify passes agents check
├── reports/
│   └── dogfooding-milestone-1.md  # New - dogfooding documentation (Phase 6)
├── tests/
│   ├── specs/
│   │   └── checks/
│   │       └── agents.rs          # Extend with snapshot tests (Phase 4)
│   └── fixtures/
│       └── agents-project/        # Existing - multi-scope fixture
└── crates/cli/src/
    └── checks/agents/             # Already implemented - no changes needed
```

## Dependencies

No new external dependencies needed. The agents check and all testing infrastructure are already in place.

**Existing infrastructure:**
- `insta` crate for snapshot testing (already in dev-dependencies)
- `tests/specs/prelude.rs` with `check()` and `cli()` helpers
- Comprehensive agents fixtures in `tests/fixtures/agents/`

## Implementation Phases

### Phase 1: Extend quench.toml with agents configuration

**Goal:** Complete the quench.toml to configure all relevant checks for the quench project itself.

**File:** `quench.toml`

**Current state:**
```toml
version = 1

[check.cloc]
advice = "..."
advice_test = "..."
exclude = ["tests/fixtures/cloc/**"]
```

**Add agents configuration:**
```toml
[check.agents]
check = "error"
files = ["CLAUDE.md"]
required = ["CLAUDE.md"]
sync = false                    # Only one agent file in quench
tables = "forbid"
max_lines = 500
max_tokens = 20000

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Document the project layout"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before committing"
```

**Verification:**
```bash
quench check --agents
# Should pass
```

### Phase 2: Run quench check on quench and capture output

**Goal:** Run all default checks and document the current state.

**Commands:**
```bash
# Run with all default checks
quench check 2>&1 | tee reports/dogfooding-run-1.txt

# Run with JSON output for structured analysis
quench check -o json > reports/dogfooding-run-1.json
```

**Capture:**
- Exit code
- Violations found (if any)
- Metrics (cloc, escapes, agents)

### Phase 3: Fix any violations found

**Goal:** Make quench pass its own checks.

**Potential violations to fix:**

1. **cloc violations** - Files exceeding max_lines (750 source, 1100 test)
   - Split large files if needed
   - Exclude generated/fixture files if appropriate

2. **escapes violations** - Missing SAFETY comments, forbidden patterns
   - Add required `// SAFETY:` comments to unsafe blocks
   - Address any lint suppressions without justification

3. **agents violations** - CLAUDE.md issues
   - Add missing required sections
   - Remove any forbidden tables
   - Ensure file is within size limits

**Verification:**
```bash
quench check
# Should exit 0 with no violations
```

### Phase 4: Add snapshot tests for agents output

**Goal:** Add `insta` snapshot tests that capture exact output format for agents check.

**File:** `tests/specs/checks/agents.rs`

**Add snapshot tests for each violation type:**

```rust
// =============================================================================
// SNAPSHOT TESTS
// =============================================================================
// These tests use insta to capture exact output format for regression testing.

use insta::assert_snapshot;

/// Snapshot: Text output for missing file violation
#[test]
fn snapshot_missing_file_text() {
    let result = check("agents")
        .on("agents/missing-file")
        .fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: JSON output for missing file violation
#[test]
fn snapshot_missing_file_json() {
    let result = check("agents")
        .on("agents/missing-file")
        .json()
        .fails();
    assert_snapshot!(result.raw_json());
}

/// Snapshot: Text output for out-of-sync files
#[test]
fn snapshot_out_of_sync_text() {
    let result = check("agents")
        .on("agents/out-of-sync")
        .fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for forbidden table
#[test]
fn snapshot_forbidden_table_text() {
    let result = check("agents")
        .on("agents/with-table")
        .fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for missing section
#[test]
fn snapshot_missing_section_text() {
    let result = check("agents")
        .on("agents/missing-section")
        .fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: Text output for oversized file (lines)
#[test]
fn snapshot_oversized_lines_text() {
    let result = check("agents")
        .on("agents/oversized-lines")
        .fails();
    assert_snapshot!(result.stdout());
}

/// Snapshot: JSON output for multi-scope project
#[test]
fn snapshot_agents_project_json() {
    let result = check("agents")
        .on("agents-project")
        .json()
        .passes();
    assert_snapshot!(result.raw_json());
}
```

**Verification:**
```bash
cargo test --test specs snapshot
# Run `cargo insta review` to approve snapshots
```

### Phase 5: Verify --fix functionality on agents-project

**Goal:** Ensure `quench check --agents --fix` correctly syncs files.

**Test setup:**
1. Create a temporary copy of agents-project with out-of-sync files
2. Run `quench check --agents --fix`
3. Verify files are now in sync

**Add test in `tests/specs/checks/agents.rs`:**

```rust
/// Spec: docs/specs/checks/agents.md#fix-mode
///
/// > --fix syncs target files to match sync_source
#[test]
fn fix_syncs_files_to_source() {
    // Create temp dir with out-of-sync files
    let dir = tempfile::tempdir().unwrap();

    // Copy fixture content
    let config = r#"
version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
required = ["CLAUDE.md"]
sync = true
sync_source = "CLAUDE.md"
"#;
    std::fs::write(dir.path().join("quench.toml"), config).unwrap();

    let source_content = "# Project\n\n## Directory Structure\n\nLayout\n\n## Landing the Plane\n\n- Done\n";
    let target_content = "# Different\n\nOld content\n";

    std::fs::write(dir.path().join("CLAUDE.md"), source_content).unwrap();
    std::fs::write(dir.path().join(".cursorrules"), target_content).unwrap();

    // Verify initially out of sync
    check("agents").pwd(dir.path()).json().fails();

    // Run fix
    check("agents").pwd(dir.path()).args(&["--fix"]).passes();

    // Verify now in sync
    let fixed_target = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(fixed_target, source_content, ".cursorrules should match CLAUDE.md after --fix");
}
```

### Phase 6: Create dogfooding report

**Goal:** Document the dogfooding experience for future reference.

**File:** `reports/dogfooding-milestone-1.md`

**Template:**

```markdown
# Dogfooding Milestone 1 Report

Date: YYYY-MM-DD

## Summary

First dogfooding milestone - running quench on the quench project itself.

## quench check Output

### Initial Run

[Paste output from Phase 2]

### After Fixes

[Paste clean output]

## Violations Found and Fixed

| Check | File | Violation | Fix Applied |
|-------|------|-----------|-------------|
| ... | ... | ... | ... |

## quench.toml Configuration

```toml
[Final quench.toml contents]
```

## Unexpected Behaviors

- [Document any surprises or issues encountered]

## Recommendations

- [Any improvements to quench discovered during dogfooding]
```

## Key Implementation Details

### Snapshot Test Helpers

The test prelude may need a `raw_json()` method to get the raw JSON string for snapshot testing:

```rust
// In tests/specs/prelude.rs
impl CheckJson {
    pub fn raw_json(&self) -> String {
        serde_json::to_string_pretty(&self.json).unwrap()
    }
}
```

If `raw_json()` doesn't exist, use the existing `stdout()` capture for JSON output tests.

### Test Fixture Requirements

The `agents-project` fixture already contains a multi-scope setup:
- Root: CLAUDE.md, .cursorrules (in sync)
- Package: crates/api/CLAUDE.md

This fixture is ideal for testing scope inheritance and sync behavior.

### Fix Mode Behavior

The `--fix` flag for agents:
1. Reads content from `sync_source` file
2. Overwrites all other configured files with that content
3. Reports files that were updated

### Output Format Consistency

Snapshot tests should verify:
- Text output includes check name, file path, violation type, and advice
- JSON output includes all structured fields (passed, violations, metrics)
- Consistent ordering and formatting

## Verification Plan

### Phase 1 Verification
```bash
quench check --agents
# Exit 0, no violations
```

### Phase 2-3 Verification
```bash
quench check
# Exit 0, all checks pass
```

### Phase 4 Verification
```bash
cargo test --test specs snapshot
cargo insta review  # Approve new snapshots
cargo test --test specs  # All tests pass
```

### Phase 5 Verification
```bash
cargo test --test specs fix_syncs
# Test passes
```

### Final Verification
```bash
make check
# All CI checks pass including:
# - cargo test --all (includes new snapshot tests)
# - ./scripts/bootstrap (project conventions)
```

## Exit Criteria

- [ ] `quench check` passes on quench project itself
- [ ] `quench.toml` includes agents configuration
- [ ] Snapshot tests exist for all violation types
- [ ] Fix functionality tested and working
- [ ] `reports/dogfooding-milestone-1.md` documents the experience
- [ ] All tests pass: `make check`
