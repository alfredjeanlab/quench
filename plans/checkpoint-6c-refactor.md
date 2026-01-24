# Checkpoint 6C: Refactor - Dogfooding Milestone 1

**Root Feature:** `quench-2bcc`

## Overview

This checkpoint addresses issues discovered during dogfooding milestone 1 (checkpoint 6B). After running quench on itself, we identified opportunities to improve:

1. **Behavioral gaps in agents check** - Edge cases and error handling improvements
2. **Dry-run mode issues** - Output consistency and formatting
3. **Code maintainability** - Simplify complex modules and clean up dead code

The dogfooding milestone successfully validated core functionality, but revealed areas where the implementation could be more robust and maintainable.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/agents/
│   │   ├── mod.rs        # Simplify orchestration, extract helpers
│   │   ├── sync.rs       # Clean up dead code (KEEP UNTIL comments)
│   │   ├── content.rs    # No changes
│   │   ├── detection.rs  # No changes
│   │   └── sections.rs   # No changes
│   ├── output/
│   │   └── text.rs       # Improve dry-run diff formatting
│   └── runner.rs         # Verify fix/dry-run context passing
├── tests/
│   ├── specs/
│   │   ├── cli/
│   │   │   └── dry_run.rs # Add edge case tests
│   │   └── checks/
│   │       └── agents.rs  # Add edge case tests
│   └── fixtures/
│       └── agents/        # Add edge case fixtures
└── reports/
    └── dogfooding-milestone-1.md  # Already exists
```

## Dependencies

No new external dependencies needed. This is a refactoring checkpoint that improves existing code.

**Existing infrastructure:**
- `insta` crate for snapshot testing
- All agents check infrastructure from Phase 520-525
- Dry-run support from Phase 525

## Implementation Phases

### Phase 1: Audit and Document Issues

**Goal:** Create a comprehensive list of issues found during dogfooding and code review.

**Tasks:**
1. Review `reports/dogfooding-milestone-1.md` for documented issues
2. Run full test suite and note any edge cases
3. Review agents/mod.rs for complexity and extract improvement opportunities
4. Review sync.rs for dead code to clean up

**Issues to document:**
- [ ] `sync.rs:91` - `target_line` field marked `KEEP UNTIL Phase 520` - now unused
- [ ] FixSummary dual structures (SyncedFile/SyncPreview) - evaluate simplification
- [ ] `runner_tests.rs:25` - `skip_reason` marked `KEEP UNTIL Phase 050`
- [ ] Dry-run diff output could show unified diff headers

**Verification:**
```bash
cargo test --all
grep -r "KEEP UNTIL" crates/cli/src/
```

### Phase 2: Clean Up Dead Code

**Goal:** Remove code marked with `KEEP UNTIL` that is no longer needed.

**File:** `crates/cli/src/checks/agents/sync.rs`

**Current state:**
```rust
#[allow(dead_code)] // KEEP UNTIL: Phase 520 - Agents Content Rules uses target_line for violations
pub target_line: Option<usize>,
```

**Action:** Review if `target_line` is used. If Phase 520 is complete and field is unused, remove it.

**File:** `crates/cli/src/runner_tests.rs`

**Current state:**
```rust
#[allow(dead_code)] // KEEP UNTIL: Phase 050 tests skip behavior
```

**Action:** Review if Phase 050 is complete. If so, remove the dead code.

**Verification:**
```bash
cargo build --all
cargo clippy --all-targets --all-features -- -D warnings
```

### Phase 3: Improve Dry-Run Output

**Goal:** Enhance dry-run output with clearer diff formatting.

**Current behavior:**
- Shows file path and "Would sync" message
- Shows old/new content diff via `write_unified_diff()`

**Improvements:**
1. Add unified diff headers (`--- old` / `+++ new`)
2. Show section-level diffs when possible
3. Improve color consistency (green for additions, red for removals)

**File:** `crates/cli/src/output/text.rs`

**Example output format:**
```
agents: PREVIEW
  .cursorrules would be synced from CLAUDE.md
  --- .cursorrules (original)
  +++ .cursorrules (synced)
  @@ -1,5 +1,9 @@
   # Project
  -## Different
  -Content B
  +## Directory Structure
  +Layout.
  +## Landing the Plane
  +- Done
```

**Verification:**
```bash
cargo test --test specs dry_run
```

### Phase 4: Add Edge Case Tests

**Goal:** Improve test coverage for edge cases discovered during dogfooding.

**File:** `tests/specs/checks/agents.rs`

**Add tests for:**
```rust
/// Edge case: sync_source file doesn't exist
#[test]
fn agents_sync_source_missing_gracefully_handles() {
    // Should not panic, should report meaningful error
}

/// Edge case: file permissions prevent reading
#[test]
fn agents_unreadable_file_gracefully_handles() {
    // Should continue with other files, log warning
}

/// Edge case: sync with identical files reports in_sync
#[test]
fn agents_identical_files_reports_in_sync() {
    // Should pass with in_sync: true
}

/// Edge case: empty agent file
#[test]
fn agents_empty_file_validates_sections() {
    // Should fail if required sections configured
}
```

**File:** `tests/specs/cli/dry_run.rs`

**Add tests for:**
```rust
/// Edge case: dry-run with no changes needed
#[test]
fn dry_run_no_changes_shows_clean() {
    // Should show PASS, no preview needed
}

/// Edge case: dry-run with JSON output
#[test]
fn dry_run_json_output_includes_previews() {
    // Should include previews in fix_summary
}
```

**Verification:**
```bash
cargo test --test specs agents
cargo test --test specs dry_run
```

### Phase 5: Simplify FixSummary

**Goal:** Evaluate whether SyncedFile and SyncPreview can be unified.

**Current state:**
```rust
struct FixSummary {
    files_synced: Vec<SyncedFile>,
    previews: Vec<SyncPreview>,
}

struct SyncedFile {
    file: String,
    source: String,
    sections: usize,
}

struct SyncPreview {
    file: String,
    source: String,
    old_content: String,
    new_content: String,
    sections: usize,
}
```

**Evaluation criteria:**
- If `SyncPreview` is only used for dry-run output, and the content can be reconstructed from file reads, unify the structures
- If both are needed for different purposes, document why

**Potential unified design:**
```rust
struct SyncAction {
    file: String,
    source: String,
    sections: usize,
    /// Only populated in dry-run mode for diff display
    preview: Option<SyncPreviewContent>,
}

struct SyncPreviewContent {
    old_content: String,
    new_content: String,
}
```

**Verification:**
```bash
cargo test --all
```

### Phase 6: Final Verification

**Goal:** Ensure all changes maintain backward compatibility and pass CI.

**Steps:**
1. Run `make check` to verify all CI checks pass
2. Run quench on quench to verify dogfooding still works
3. Verify snapshot tests are still valid (no regressions)

**Verification:**
```bash
make check
cargo run -- check -o json | jq '.passed'  # Should be true
```

## Key Implementation Details

### Dead Code Cleanup Strategy

When removing `KEEP UNTIL` marked code:
1. Verify the referenced phase is complete
2. Search for any usage of the field/function
3. Remove the code and the `#[allow(dead_code)]` attribute
4. Run `cargo build` to confirm no dependencies

### Dry-Run Diff Format

Use standard unified diff format for clarity:
```
--- a/path/to/file
+++ b/path/to/file
@@ -start,count +start,count @@
 context line
-removed line
+added line
 context line
```

### Edge Case Handling Patterns

```rust
// Graceful handling of read errors
let content = match std::fs::read_to_string(&path) {
    Ok(c) => c,
    Err(e) => {
        tracing::warn!("cannot read {}: {}", path.display(), e);
        continue; // Skip this file, continue with others
    }
};
```

### JSON Output Consistency

Ensure dry-run mode produces consistent JSON:
```json
{
  "name": "agents",
  "passed": true,
  "fixed": false,
  "fix_summary": {
    "files_synced": [],
    "previews": [
      {
        "file": ".cursorrules",
        "source": "CLAUDE.md",
        "sections": 3
      }
    ]
  }
}
```

## Verification Plan

### Phase 1 Verification
```bash
grep -r "KEEP UNTIL" crates/cli/src/
# Document all occurrences
```

### Phase 2 Verification
```bash
cargo build --all
cargo clippy --all-targets --all-features -- -D warnings
# No dead_code warnings for removed items
```

### Phase 3 Verification
```bash
cargo test --test specs dry_run
# All tests pass, output looks correct
```

### Phase 4 Verification
```bash
cargo test --test specs agents
cargo test --test specs dry_run
# All new edge case tests pass
```

### Phase 5 Verification
```bash
cargo test --all
# No regressions from FixSummary changes
```

### Phase 6 (Final) Verification
```bash
make check
# All CI checks pass
```

## Exit Criteria

- [ ] All `KEEP UNTIL` code reviewed and cleaned up where appropriate
- [ ] Dry-run output improved with unified diff format
- [ ] Edge case tests added for agents check
- [ ] Edge case tests added for dry-run mode
- [ ] FixSummary evaluated and simplified if appropriate
- [ ] All tests pass: `make check`
- [ ] Dogfooding still works: `quench check` passes on quench
