# Phase 820: Git Check - Template

**Root Feature:** `quench-3153`
**Depends On:** Phase 815 (Git Check - Agent Documentation)

## Overview

Implement `.gitmessage` template generation and git configuration for the git check. When `template = true` (default) and `--fix` is passed, quench:

1. **Creates `.gitmessage`** with commit format documentation derived from config
2. **Configures git** via `git config commit.template .gitmessage`
3. **Respects existing files** - never overwrites existing `.gitmessage` or config

This provides developers and AI agents with an in-editor reminder of the expected commit format.

## Project Structure

```
crates/cli/src/
├── checks/
│   └── git/
│       ├── mod.rs              # EXTEND: Add template fix logic
│       ├── template.rs         # NEW: Template generation
│       └── template_tests.rs   # NEW: Unit tests
tests/
├── specs/checks/git.rs         # UPDATE: Remove #[ignore] from template specs
└── fixtures/git/
    └── (existing fixtures work for template tests)
```

## Dependencies

No new external dependencies. Uses existing:
- `std::process::Command` for git config
- `std::fs` for file operations

## Implementation Phases

### Phase 1: Create Template Generation Module

Create `crates/cli/src/checks/git/template.rs` with template generation logic.

```rust
//! Git commit message template generation.
//!
//! Generates `.gitmessage` content from configuration.

use crate::checks::git::parse::DEFAULT_TYPES;
use crate::config::GitCommitConfig;

/// Default template path.
pub const TEMPLATE_PATH: &str = ".gitmessage";

/// Generate .gitmessage content from configuration.
///
/// Template format:
/// ```text
/// # <type>(<scope>): <description>
/// #
/// # Types: feat, fix, chore, ...
/// # Scope: optional (api, cli, core)
/// #
/// # Examples:
/// #   feat(api): add export endpoint
/// #   fix: handle empty input
/// ```
pub fn generate_template(config: &GitCommitConfig) -> String {
    let types = effective_types(config);
    let scopes = config.scopes.as_ref();

    let mut lines = Vec::new();

    // Header with format reminder
    if scopes.is_some() {
        lines.push("# <type>(<scope>): <description>".to_string());
    } else {
        lines.push("# <type>: <description>".to_string());
    }
    lines.push("#".to_string());

    // Types line
    if types.is_empty() {
        lines.push("# Types: (any)".to_string());
    } else {
        lines.push(format!("# Types: {}", types.join(", ")));
    }

    // Scopes line (if configured)
    if let Some(scopes) = scopes {
        if scopes.is_empty() {
            lines.push("# Scope: optional".to_string());
        } else {
            lines.push(format!("# Scope: optional ({})", scopes.join(", ")));
        }
    }

    // Examples section
    lines.push("#".to_string());
    lines.push("# Examples:".to_string());

    let example_type = types.first().map(|s| s.as_str()).unwrap_or("feat");
    if let Some(scopes) = scopes
        && !scopes.is_empty()
    {
        let scope = scopes.first().unwrap();
        lines.push(format!("#   {}({}): add new feature", example_type, scope));
    } else {
        lines.push(format!("#   {}: add new feature", example_type));
    }

    // Second example without scope
    let fix_type = if types.contains(&"fix".to_string()) {
        "fix"
    } else {
        types.get(1).map(|s| s.as_str()).unwrap_or("fix")
    };
    lines.push(format!("#   {}: handle edge case", fix_type));

    // Trailing newline for clean file
    lines.push(String::new());

    lines.join("\n")
}

/// Get effective types list for template.
fn effective_types(config: &GitCommitConfig) -> Vec<String> {
    match &config.types {
        Some(types) => types.clone(),
        None => DEFAULT_TYPES.iter().map(|s| s.to_string()).collect(),
    }
}
```

**Milestone:** Module compiles and exports `generate_template()`.

### Phase 2: Add Fix Logic to Git Check

Update `crates/cli/src/checks/git/mod.rs` to handle `--fix` for template creation.

```rust
mod template;

use std::process::Command;
use template::{TEMPLATE_PATH, generate_template};

impl Check for GitCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // ... existing validation code ...

        // Handle --fix for template creation
        let fix_summary = if ctx.fix && config.template {
            fix_template(ctx.root, config, ctx.dry_run)
        } else {
            None
        };

        if violations.is_empty() {
            if let Some(summary) = fix_summary {
                CheckResult::fixed(self.name(), summary)
            } else {
                CheckResult::passed(self.name())
            }
        } else {
            CheckResult::failed(self.name(), violations)
        }
    }
}

/// Fix template and git config if needed.
///
/// Returns fix summary if anything was fixed, None otherwise.
fn fix_template(
    root: &Path,
    config: &GitCommitConfig,
    dry_run: bool,
) -> Option<serde_json::Value> {
    let template_path = root.join(TEMPLATE_PATH);
    let mut actions = Vec::new();

    // Create .gitmessage if missing
    if !template_path.exists() {
        let content = generate_template(config);
        if !dry_run {
            if let Err(e) = std::fs::write(&template_path, &content) {
                // Log error but continue - this is a best-effort fix
                eprintln!("Warning: Failed to create {}: {}", TEMPLATE_PATH, e);
            } else {
                actions.push(format!("Created {} (commit template)", TEMPLATE_PATH));
            }
        } else {
            actions.push(format!("Would create {} (commit template)", TEMPLATE_PATH));
        }
    }

    // Configure git commit.template if not set
    if !is_template_configured(root) {
        if !dry_run {
            if configure_git_template(root) {
                actions.push("Configured git commit.template".to_string());
            }
        } else {
            actions.push("Would configure git commit.template".to_string());
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(serde_json::json!({
            "actions": actions
        }))
    }
}

/// Check if commit.template is already configured.
fn is_template_configured(root: &Path) -> bool {
    Command::new("git")
        .args(["config", "commit.template"])
        .current_dir(root)
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false)
}

/// Configure git commit.template to use .gitmessage.
fn configure_git_template(root: &Path) -> bool {
    Command::new("git")
        .args(["config", "commit.template", TEMPLATE_PATH])
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
```

**Milestone:** `quench check --git --fix` creates template and configures git.

### Phase 3: Unit Tests for Template Generation

Create `crates/cli/src/checks/git/template_tests.rs`.

```rust
//! Unit tests for git template generation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::template::*;
use crate::config::GitCommitConfig;

// =============================================================================
// TEMPLATE CONTENT TESTS
// =============================================================================

#[test]
fn generates_template_with_default_config() {
    let config = GitCommitConfig::default();
    let template = generate_template(&config);

    assert!(template.contains("# <type>: <description>"));
    assert!(template.contains("# Types: feat, fix, chore"));
    assert!(template.contains("# Examples:"));
}

#[test]
fn generates_template_with_custom_types() {
    let config = GitCommitConfig {
        types: Some(vec!["feat".to_string(), "fix".to_string()]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# Types: feat, fix"));
    assert!(!template.contains("chore"));
}

#[test]
fn generates_template_with_scopes() {
    let config = GitCommitConfig {
        scopes: Some(vec!["api".to_string(), "cli".to_string()]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# <type>(<scope>): <description>"));
    assert!(template.contains("# Scope: optional (api, cli)"));
    assert!(template.contains("(api):")); // Example uses first scope
}

#[test]
fn generates_template_with_empty_types() {
    let config = GitCommitConfig {
        types: Some(vec![]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# Types: (any)"));
}

#[test]
fn generates_template_with_empty_scopes() {
    let config = GitCommitConfig {
        scopes: Some(vec![]),
        ..Default::default()
    };
    let template = generate_template(&config);

    assert!(template.contains("# Scope: optional"));
    assert!(!template.contains("# Scope: optional ("));
}

#[test]
fn template_ends_with_newline() {
    let config = GitCommitConfig::default();
    let template = generate_template(&config);

    assert!(template.ends_with('\n'));
}

// =============================================================================
// INTEGRATION TESTS (with temp directories)
// =============================================================================

#[test]
fn fix_creates_gitmessage_when_missing() {
    let temp = tempfile::tempdir().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let config = GitCommitConfig::default();
    let template_path = temp.path().join(TEMPLATE_PATH);

    // File should not exist
    assert!(!template_path.exists());

    // Write template
    let content = generate_template(&config);
    std::fs::write(&template_path, &content).unwrap();

    // File should now exist
    assert!(template_path.exists());
    let written = std::fs::read_to_string(&template_path).unwrap();
    assert!(written.contains("# Types:"));
}

#[test]
fn is_template_configured_returns_false_for_new_repo() {
    let temp = tempfile::tempdir().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(!super::is_template_configured(temp.path()));
}

#[test]
fn configure_git_template_sets_config() {
    let temp = tempfile::tempdir().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(super::configure_git_template(temp.path()));
    assert!(super::is_template_configured(temp.path()));

    // Verify the value
    let output = std::process::Command::new("git")
        .args(["config", "commit.template"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    let value = String::from_utf8_lossy(&output.stdout);
    assert_eq!(value.trim(), TEMPLATE_PATH);
}
```

**Milestone:** All unit tests pass.

### Phase 4: Enable Behavioral Specs

Update `tests/specs/checks/git.rs` to remove `#[ignore]` from template specs.

Specs to enable:
- `git_fix_creates_gitmessage_template` - creates .gitmessage with --fix
- `git_fix_configures_commit_template` - sets git config
- `git_fix_does_not_overwrite_existing_gitmessage` - respects existing file

Update ignore annotation from:
```rust
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
```

To: (remove entirely)

**Milestone:** Enabled specs pass with `cargo test --test specs git`.

### Phase 5: Update CheckResult for Fix Summary

Verify `CheckResult::fixed()` handles the fix summary correctly. The current implementation should already support this, but verify integration.

```rust
// In check.rs, verify this pattern works:
CheckResult::fixed(self.name(), serde_json::json!({
    "actions": ["Created .gitmessage", "Configured git commit.template"]
}))
```

If needed, update the CheckResult struct to handle fix summaries appropriately.

**Milestone:** JSON output includes fix_summary with actions.

### Phase 6: Integration Testing

Verify the full flow works end-to-end.

```bash
# Create project with template enabled
mkdir -p /tmp/git-template-test && cd /tmp/git-template-test
echo 'version = 1
[git.commit]
check = "error"
template = true
types = ["feat", "fix", "chore"]
scopes = ["api", "cli"]' > quench.toml
echo '# Test

## Commits

Use feat: format.

## Directory Structure

Minimal.

## Landing the Plane

- Done' > CLAUDE.md

# Initialize git
git init

# Verify .gitmessage doesn't exist
ls -la .gitmessage  # Should fail

# Run fix
quench check --git --fix

# Verify .gitmessage was created
cat .gitmessage
# Should show template with types and scopes

# Verify git config was set
git config commit.template
# Should output: .gitmessage

# Run fix again - should be idempotent
quench check --git --fix
# Should pass without creating new files
```

**Milestone:** Real projects work correctly with template generation.

## Key Implementation Details

### Template Content Structure

The generated template follows this structure:

```
# <type>(<scope>): <description>
#
# Types: feat, fix, chore, docs, test, refactor
# Scope: optional (api, cli, core)
#
# Examples:
#   feat(api): add export endpoint
#   fix: handle empty input
```

Key design decisions:
1. **Comment-only** - Lines starting with `#` are ignored by git
2. **Config-derived** - Types and scopes come from `[git.commit]` config
3. **Example-driven** - Shows concrete examples using actual configured types/scopes

### Idempotent Fix Behavior

| Scenario | Action |
|----------|--------|
| `.gitmessage` missing | Create it |
| `.gitmessage` exists | Leave it alone (never overwrite) |
| `commit.template` not set | Set it to `.gitmessage` |
| `commit.template` already set | Leave it alone (any value) |

This ensures running `--fix` multiple times is safe.

### Dry Run Support

With `--dry-run --fix`:
- Reports what would be created/configured
- Does not modify any files
- Does not run any git commands

Fix summary for dry run:
```json
{
  "actions": [
    "Would create .gitmessage (commit template)",
    "Would configure git commit.template"
  ]
}
```

### Error Handling

Template creation errors are logged but don't fail the check:
- File write failures → warning to stderr, continue
- Git config failures → warning to stderr, continue

This matches the "best effort" nature of `--fix` - the primary check (commit validation) still runs.

## Verification Plan

### Unit Tests

```bash
# Run template generation unit tests
cargo test --package quench checks::git::template

# Run all git check tests
cargo test --package quench checks::git
```

### Behavioral Specs

```bash
# Run git check specs
cargo test --test specs git

# Show any remaining ignored specs
cargo test --test specs git -- --ignored
```

### Full Suite

```bash
# Run complete check suite
make check
```

## Checklist

- [ ] Create `crates/cli/src/checks/git/template.rs`
- [ ] Add `TEMPLATE_PATH` constant
- [ ] Implement `generate_template()` function
- [ ] Add `effective_types()` helper
- [ ] Update `mod.rs` to add `mod template;`
- [ ] Add `fix_template()` function
- [ ] Add `is_template_configured()` helper
- [ ] Add `configure_git_template()` helper
- [ ] Integrate fix logic into `GitCheck::run()`
- [ ] Create `template_tests.rs` with unit tests
- [ ] Remove `#[ignore]` from `git_fix_creates_gitmessage_template`
- [ ] Remove `#[ignore]` from `git_fix_configures_commit_template`
- [ ] Remove `#[ignore]` from `git_fix_does_not_overwrite_existing_gitmessage`
- [ ] Run `make check` - all tests pass

## Deliverables

This phase produces:
1. **Template module**: `template.rs` with generation logic
2. **Fix integration**: Git check handles `--fix` for template creation
3. **Git configuration**: Automatic `commit.template` setup
4. **Idempotent behavior**: Safe to run multiple times
5. **Test coverage**: Unit tests and enabled behavioral specs

After this phase, the git check is fully functional with commit validation, agent documentation checking, and template generation.
