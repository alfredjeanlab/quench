# Phase 528: Dry-Run Mode Implementation

## Overview

Implement a `--dry-run` flag that shows what `--fix` would change without modifying any files. This allows users to preview fix operations before committing to them.

**Key behaviors:**
- `--dry-run` requires `--fix` (error without it)
- Shows files that would be modified with unified diff output
- Exits 0 even when fixes are available (success = preview complete)
- Never modifies any files

## Project Structure

Files to modify:

```
crates/cli/src/
├── cli.rs          # Add --dry-run flag to CheckArgs
├── main.rs         # Flag validation + exit code override
├── runner.rs       # Add dry_run to RunnerConfig
├── check.rs        # Add dry_run to CheckContext
├── checks/
│   └── agents/
│       └── mod.rs  # Skip fs::write when dry_run, collect preview data
└── output/
    └── text.rs     # Add diff display for dry-run previews
```

## Dependencies

No new external dependencies required. The existing diff comparison infrastructure in `checks/agents/sync.rs` already provides the data needed for diff display.

## Implementation Phases

### Phase 1: CLI Flag Infrastructure

Add the `--dry-run` flag and thread it through the execution pipeline.

**cli.rs** - Add flag to CheckArgs (around line 86):

```rust
/// Show what --fix would change without changing it
#[arg(long, requires = "fix")]
pub dry_run: bool,
```

Using `requires = "fix"` lets clap handle the validation automatically.

**runner.rs** - Add to RunnerConfig:

```rust
pub struct RunnerConfig {
    pub limit: Option<usize>,
    pub changed_files: Option<Vec<PathBuf>>,
    pub fix: bool,
    pub dry_run: bool,  // NEW
}
```

**check.rs** - Add to CheckContext:

```rust
pub struct CheckContext<'a> {
    // ... existing fields ...
    pub fix: bool,
    pub dry_run: bool,  // NEW
}
```

**main.rs** - Pass flag through runner construction:

```rust
let mut runner = CheckRunner::new(RunnerConfig {
    limit,
    changed_files,
    fix: args.fix,
    dry_run: args.dry_run,  // NEW
});
```

### Phase 2: Fix Preview Collection

Modify the agents check to collect preview data instead of writing files when in dry-run mode.

**checks/agents/mod.rs** - Extend FixSummary:

```rust
#[derive(Debug)]
struct SyncPreview {
    file: String,
    source: String,
    old_content: String,
    new_content: String,
    sections: usize,
}

#[derive(Debug, Default)]
struct FixSummary {
    files_synced: Vec<SyncedFile>,
    previews: Vec<SyncPreview>,  // NEW: for dry-run
}

impl FixSummary {
    fn add_preview(&mut self, file: String, source: String, old: String, new: String, sections: usize) {
        self.previews.push(SyncPreview {
            file,
            source,
            old_content: old,
            new_content: new,
            sections,
        });
    }
}
```

**checks/agents/mod.rs** - Modify fix logic (~line 292):

```rust
if ctx.fix {
    let section_count = comparison.differences.len();

    if ctx.dry_run {
        // Preview only: collect diff data without writing
        fixes.add_preview(
            target_name.clone(),
            source_name.to_string(),
            target_content.clone(),
            source_content.clone(),
            section_count,
        );
    } else if std::fs::write(&target_file.path, &source_content).is_ok() {
        // Actual fix: write and track
        fixes.add_sync(target_name, source_name.to_string(), section_count);
        continue;
    }
}
```

**Include preview data in JSON output:**

```rust
impl FixSummary {
    fn to_json(&self) -> JsonValue {
        json!({
            "files_synced": self.files_synced.iter().map(|s| json!({
                "file": s.file,
                "source": s.source,
                "sections": s.sections,
            })).collect::<Vec<_>>(),
            "previews": self.previews.iter().map(|p| json!({
                "file": p.file,
                "source": p.source,
                "old_content": p.old_content,
                "new_content": p.new_content,
                "sections": p.sections,
            })).collect::<Vec<_>>(),
        })
    }
}
```

### Phase 3: Diff Output Formatting

Add unified diff display to the text formatter for dry-run previews.

**output/text.rs** - Extend write_fix_summary:

```rust
fn write_fix_summary(&mut self, summary: &serde_json::Value) -> std::io::Result<()> {
    // Existing: show files_synced for actual fixes
    if let Some(synced) = summary.get("files_synced").and_then(|s| s.as_array()) {
        for entry in synced {
            let file = entry.get("file").and_then(|f| f.as_str()).unwrap_or("?");
            let source = entry.get("source").and_then(|s| s.as_str()).unwrap_or("?");
            let sections = entry.get("sections").and_then(|n| n.as_i64()).unwrap_or(0);
            writeln!(
                self.stdout,
                "  Synced {} from {} ({} sections updated)",
                file, source, sections
            )?;
        }
    }

    // NEW: show previews for dry-run
    if let Some(previews) = summary.get("previews").and_then(|p| p.as_array()) {
        for entry in previews {
            self.write_diff_preview(entry)?;
        }
    }
    Ok(())
}

fn write_diff_preview(&mut self, preview: &serde_json::Value) -> std::io::Result<()> {
    let file = preview.get("file").and_then(|f| f.as_str()).unwrap_or("?");
    let source = preview.get("source").and_then(|s| s.as_str()).unwrap_or("?");
    let old_content = preview.get("old_content").and_then(|c| c.as_str()).unwrap_or("");
    let new_content = preview.get("new_content").and_then(|c| c.as_str()).unwrap_or("");
    let sections = preview.get("sections").and_then(|n| n.as_i64()).unwrap_or(0);

    // Header
    writeln!(self.stdout, "  Would sync {} from {} ({} sections)", file, source, sections)?;

    // Unified diff
    self.write_unified_diff(file, old_content, new_content)?;

    Ok(())
}

fn write_unified_diff(&mut self, file: &str, old: &str, new: &str) -> std::io::Result<()> {
    writeln!(self.stdout, "  --- a/{}", file)?;
    writeln!(self.stdout, "  +++ b/{}", file)?;

    // Simple line-by-line diff
    for line in old.lines() {
        self.stdout.set_color(&scheme::diff_remove())?;
        writeln!(self.stdout, "  -{}", line)?;
        self.stdout.reset()?;
    }
    for line in new.lines() {
        self.stdout.set_color(&scheme::diff_add())?;
        writeln!(self.stdout, "  +{}", line)?;
        self.stdout.reset()?;
    }

    Ok(())
}
```

**color/scheme.rs** - Add diff colors (if not present):

```rust
pub fn diff_remove() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Red));
    spec
}

pub fn diff_add() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Green));
    spec
}
```

### Phase 4: Exit Code Override

Ensure dry-run always exits 0 regardless of whether fixes are available.

**main.rs** - Modify exit code logic (~line 355):

```rust
let exit_code = if args.dry_run {
    // Dry-run always succeeds: preview is complete
    ExitCode::Success
} else if !output.passed {
    ExitCode::CheckFailed
} else {
    ExitCode::Success
};
```

### Phase 5: Result Status for Dry-Run

The check result should reflect dry-run state appropriately.

**check.rs** - Add preview result constructor:

```rust
impl CheckResult {
    /// Create a result showing fixes that would be applied (dry-run mode).
    pub fn preview(name: impl Into<String>, summary: JsonValue) -> Self {
        Self {
            name: name.into(),
            passed: true,  // Preview is "success" - we showed what would happen
            skipped: false,
            stub: false,
            fixed: false,
            preview: true,  // NEW field
            error: None,
            violations: Vec::new(),
            fix_summary: Some(summary),
            metrics: None,
            by_package: None,
        }
    }
}
```

Alternatively, reuse `fixed: true` but set a flag in the summary to indicate preview mode. The simpler approach is to check `summary.get("previews")` presence.

**checks/agents/mod.rs** - Return appropriate result:

```rust
if !fixes.is_empty() {
    if ctx.dry_run {
        // Return as passed with preview data
        return CheckResult::passed(&self.name).with_fix_summary(fixes.to_json());
    } else {
        return CheckResult::fixed(&self.name, fixes.to_json());
    }
}
```

## Key Implementation Details

### Flag Validation

Using clap's `requires` attribute handles validation at the parse level:

```rust
#[arg(long, requires = "fix")]
pub dry_run: bool,
```

This produces the error message automatically. If the default message doesn't match the spec exactly (`"--dry-run requires --fix"`), add manual validation in main.rs:

```rust
if args.dry_run && !args.fix {
    eprintln!("--dry-run requires --fix");
    return ExitCode::ConfigError;
}
```

### Content Threading

The dry-run mode needs access to both old and new content at the point where fixes are applied. The agents check already has both:
- `source_content`: what would be written (new)
- `target_content`: current file contents (old)

### Diff Format

The specs require showing both old content (being removed) and new content (being added). A simple unified diff format shows:
- `--- a/file` header for old
- `+++ b/file` header for new
- `-line` for removed lines (red)
- `+line` for added lines (green)

For more sophisticated diff output (context, hunks), consider using the `similar` crate, but the simple format satisfies the spec requirements.

### JSON Output

The preview data is included in the fix_summary JSON, making it available for `--output json`:

```json
{
  "checks": [{
    "name": "agents",
    "passed": true,
    "fix_summary": {
      "files_synced": [],
      "previews": [{
        "file": ".cursorrules",
        "source": "CLAUDE.md",
        "old_content": "# Target\nContent B",
        "new_content": "# Source\nContent A",
        "sections": 1
      }]
    }
  }]
}
```

## Verification Plan

### Spec Tests

Enable and verify all specs in `tests/specs/cli/dry_run.rs`:

1. **dry_run_without_fix_is_error** - Verify exit code 2 and error message
2. **dry_run_shows_files_that_would_be_modified** - Verify filename in stdout
3. **dry_run_shows_diff_of_changes** - Verify both old and new content shown
4. **dry_run_exits_0_when_fixes_needed** - Verify passes() succeeds
5. **dry_run_does_not_modify_files** - Verify file unchanged after run

### Manual Testing

```bash
# Setup test files
mkdir -p /tmp/dryrun-test && cd /tmp/dryrun-test
cat > quench.toml << 'EOF'
version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
EOF
echo "# Source\nContent A" > CLAUDE.md
echo "# Target\nContent B" > .cursorrules

# Test: --dry-run without --fix (should fail)
quench check --dry-run
# Expected: exit 2, stderr: "--dry-run requires --fix"

# Test: --dry-run with --fix (should show preview)
quench check --fix --dry-run
# Expected: exit 0, stdout shows .cursorrules, shows Content A and Content B

# Verify file unchanged
cat .cursorrules
# Expected: "# Target\nContent B"

# Test: actual --fix (should modify)
quench check --fix
cat .cursorrules
# Expected: "# Source\nContent A"
```

### CI Verification

```bash
make check  # Runs all tests including specs
```

## Summary

| Phase | Task | Files |
|-------|------|-------|
| 1 | CLI flag infrastructure | cli.rs, runner.rs, check.rs, main.rs |
| 2 | Fix preview collection | checks/agents/mod.rs |
| 3 | Diff output formatting | output/text.rs, color/scheme.rs |
| 4 | Exit code override | main.rs |
| 5 | Result status handling | check.rs, checks/agents/mod.rs |

After implementation, remove `#[ignore]` from the 5 specs in `tests/specs/cli/dry_run.rs` and verify all pass.
