# Phase 815: Git Check - Agent Documentation

**Root Feature:** `quench-3153`
**Depends On:** Phase 810 (Git Check - Validation)

## Overview

Implement agent documentation checking for the git check. When `agents = true` (default), quench verifies that commit format is documented in agent-readable files (CLAUDE.md, AGENTS.md, .cursorrules).

The agent documentation check:
- **Detects commit format documentation** via type prefixes (`feat:`, `fix(`) or "conventional commits" phrase
- **Searches agent files** at project root (CLAUDE.md, AGENTS.md, .cursorrules)
- **Generates `missing_docs` violation** when no documentation is found
- **Provides actionable advice** with example commit section

This phase does NOT include template creation (future phase).

## Project Structure

```
crates/cli/src/
├── checks/
│   └── git/
│       ├── mod.rs            # EXTEND: Add agent docs check call
│       ├── docs.rs           # NEW: Agent documentation detection
│       └── docs_tests.rs     # NEW: Unit tests for detection
tests/
├── specs/checks/git.rs       # UPDATE: Remove #[ignore] from docs specs
└── fixtures/git/
    └── missing-docs/         # NEW: Fixture for missing docs test
```

## Dependencies

No new external dependencies. Uses existing:
- `regex` for pattern matching (already in Cargo.toml)
- File detection patterns from agents check

## Implementation Phases

### Phase 1: Create Documentation Detection Module

Create `crates/cli/src/checks/git/docs.rs` with detection logic.

```rust
//! Agent documentation detection for commit format.
//!
//! Checks that commit format is documented in agent files.

use std::path::Path;

use regex::Regex;

/// Agent files to check for commit documentation.
const AGENT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", ".cursorrules"];

/// Default commit types to search for.
const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style",
];

/// Result of searching for commit format documentation.
#[derive(Debug)]
pub enum DocsResult {
    /// Documentation found in the specified file.
    Found(String),
    /// No documentation found; lists checked files.
    NotFound(Vec<String>),
    /// No agent files exist at root.
    NoAgentFiles,
}

/// Check if commit format is documented in agent files.
///
/// Searches for:
/// - Type prefixes followed by `:` or `(` (e.g., `feat:`, `fix(`)
/// - The phrase "conventional commits" (case-insensitive)
pub fn check_commit_docs(root: &Path) -> DocsResult {
    let mut checked_files = Vec::new();

    for filename in AGENT_FILES {
        let path = root.join(filename);
        if !path.exists() {
            continue;
        }

        checked_files.push(filename.to_string());

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        if has_commit_documentation(&content) {
            return DocsResult::Found(filename.to_string());
        }
    }

    if checked_files.is_empty() {
        DocsResult::NoAgentFiles
    } else {
        DocsResult::NotFound(checked_files)
    }
}

/// Check if content contains commit format documentation.
fn has_commit_documentation(content: &str) -> bool {
    has_type_prefix(content) || has_conventional_commits_phrase(content)
}

/// Check for type prefixes followed by `:` or `(`.
///
/// Matches: `feat:`, `fix(`, `chore:`, etc.
fn has_type_prefix(content: &str) -> bool {
    // Build regex pattern: (feat|fix|chore|...)[:({]
    let types_pattern = COMMIT_TYPES.join("|");
    let pattern = format!(r"(?i)\b({})[:(\(]", types_pattern);

    Regex::new(&pattern)
        .map(|re| re.is_match(content))
        .unwrap_or(false)
}

/// Check for "conventional commits" phrase (case-insensitive).
fn has_conventional_commits_phrase(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("conventional commits") || lower.contains("conventional commit")
}

/// Get the primary agent file name for violation reporting.
///
/// Returns the first agent file that exists, or "CLAUDE.md" as default.
pub fn primary_agent_file(root: &Path) -> &'static str {
    for filename in AGENT_FILES {
        if root.join(filename).exists() {
            return filename;
        }
    }
    "CLAUDE.md"
}
```

**Milestone:** Module compiles and exports detection functions.

### Phase 2: Integrate with Git Check

Update `crates/cli/src/checks/git/mod.rs` to call documentation check.

```rust
mod docs;

use docs::{DocsResult, check_commit_docs, primary_agent_file};

impl Check for GitCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // ... existing validation code ...

        let mut violations = Vec::new();

        // Check agent documentation (if enabled)
        if config.agents {
            check_agent_docs(ctx.root, &mut violations);
        }

        // ... rest of commit validation ...
    }
}

/// Check that commit format is documented in agent files.
fn check_agent_docs(root: &Path, violations: &mut Vec<Violation>) {
    match check_commit_docs(root) {
        DocsResult::Found(_) => {
            // Documentation found, nothing to do
        }
        DocsResult::NotFound(checked) => {
            // Files exist but lack documentation
            let file = primary_agent_file(root);
            violations.push(
                Violation::file_only(file, "missing_docs", format!(
                    "Add a Commits section describing the format, e.g.:\n\n\
                    ## Commits\n\n\
                    Use conventional commit format: `type(scope): description`\n\
                    Types: feat, fix, chore, docs, test, refactor"
                ))
            );
        }
        DocsResult::NoAgentFiles => {
            // No agent files to check - this is handled by agents check
            // or the user may not want agent files at all
        }
    }
}
```

**Milestone:** `quench check --git` reports `missing_docs` violations.

### Phase 3: Add Test Fixture

Create `tests/fixtures/git/missing-docs/` for behavioral specs.

**`tests/fixtures/git/missing-docs/quench.toml`:**
```toml
version = 1

[git.commit]
check = "error"
agents = true
```

**`tests/fixtures/git/missing-docs/CLAUDE.md`:**
```markdown
# Project

This project does something.

## Directory Structure

Minimal project.

## Landing the Plane

- Done
```

Note: This CLAUDE.md intentionally lacks commit format documentation.

**Milestone:** Fixture exists and has no commit documentation.

### Phase 4: Unit Tests for Detection

Create `crates/cli/src/checks/git/docs_tests.rs`.

```rust
//! Unit tests for agent documentation detection.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::docs::*;

// =============================================================================
// TYPE PREFIX DETECTION
// =============================================================================

#[test]
fn detects_feat_colon() {
    let content = "Use `feat:` for new features.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_fix_paren() {
    let content = "Use `fix(scope):` for bug fixes.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_type_in_example() {
    let content = "Example: `chore: update deps`";
    assert!(has_commit_documentation(content));
}

#[test]
fn requires_colon_or_paren_after_type() {
    // "feat" alone should not match
    let content = "This project features cool stuff.";
    assert!(!has_commit_documentation(content));
}

#[test]
fn case_insensitive_type_detection() {
    let content = "Use FEAT: for features";
    assert!(has_commit_documentation(content));
}

// =============================================================================
// CONVENTIONAL COMMITS PHRASE
// =============================================================================

#[test]
fn detects_conventional_commits_phrase() {
    let content = "We use Conventional Commits.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_conventional_commit_singular() {
    let content = "Follow the conventional commit format.";
    assert!(has_commit_documentation(content));
}

#[test]
fn conventional_commits_case_insensitive() {
    let content = "Use CONVENTIONAL COMMITS format.";
    assert!(has_commit_documentation(content));
}

// =============================================================================
// NEGATIVE CASES
// =============================================================================

#[test]
fn no_detection_in_unrelated_content() {
    let content = "# Project\n\nThis is a project about features.";
    assert!(!has_commit_documentation(content));
}

#[test]
fn no_detection_in_empty_content() {
    let content = "";
    assert!(!has_commit_documentation(content));
}

// =============================================================================
// INTEGRATION WITH FILE CHECKING
// =============================================================================

#[test]
fn check_commit_docs_finds_in_claude_md() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\n## Commits\n\nUse feat: format.\n"
    ).unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::Found(file) => assert_eq!(file, "CLAUDE.md"),
        other => panic!("Expected Found, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_not_found_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("CLAUDE.md"),
        "# Project\n\nNo commit info.\n"
    ).unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::NotFound(files) => {
            assert!(files.contains(&"CLAUDE.md".to_string()));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_no_agent_files() {
    let temp = tempfile::tempdir().unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::NoAgentFiles => {}
        other => panic!("Expected NoAgentFiles, got {:?}", other),
    }
}
```

**Milestone:** All unit tests pass.

### Phase 5: Enable Behavioral Specs

Update `tests/specs/checks/git.rs` to remove `#[ignore]` from documentation specs.

Specs to enable:
- `git_missing_format_documentation_generates_violation` - missing docs violation
- `git_detects_commit_format_via_type_prefixes` - type prefix detection
- `git_detects_commit_format_via_conventional_commits_phrase` - phrase detection
- `git_skips_docs_check_when_agents_disabled` - agents = false
- `git_missing_docs_violation_references_file` - JSON output

Note: Fix specs (`git_fix_creates_gitmessage_template`, etc.) remain ignored for future phase.

**Milestone:** Enabled specs pass with `cargo test --test specs git`.

### Phase 6: Integration Testing

Verify the full flow works end-to-end.

```bash
# Create project with CLAUDE.md missing commit docs
mkdir -p /tmp/git-docs-test && cd /tmp/git-docs-test
echo 'version = 1\n[git.commit]\ncheck = "error"\nagents = true' > quench.toml
echo '# Test\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done' > CLAUDE.md

# Should fail with missing_docs
quench check --git --json | jq '.violations[].type'
# Should output: "missing_docs"

# Add commit documentation
echo '# Test\n\n## Commits\n\nfeat: or fix: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done' > CLAUDE.md

# Should pass
quench check --git
```

**Milestone:** Real projects validate correctly.

## Key Implementation Details

### Detection Algorithm

The detection searches for two patterns:

1. **Type prefixes**: Any conventional commit type followed by `:` or `(`
   - Pattern: `\b(feat|fix|chore|...)[:(\(]`
   - Case-insensitive
   - Matches: `feat:`, `fix(scope):`, `CHORE:`

2. **Conventional commits phrase**: The phrase itself
   - Matches: "conventional commits", "conventional commit"
   - Case-insensitive

### Agent File Priority

Files are checked in order: CLAUDE.md → AGENTS.md → .cursorrules

If any file contains documentation, the check passes. If none do, the primary file (first existing) is referenced in the violation.

### Violation Structure

```json
{
  "file": "CLAUDE.md",
  "line": null,
  "type": "missing_docs",
  "advice": "Add a Commits section describing the format..."
}
```

Unlike commit violations (`invalid_format`, etc.), `missing_docs` violations reference a file, not a commit.

### Integration Point

The agent docs check runs before commit validation and adds to the same violations list:

```
GitCheck::run()
├── check if git repo
├── check if enabled
├── check agent docs (if config.agents)  ← NEW
├── get commits to check
├── validate each commit
└── return result
```

## Verification Plan

### Unit Tests

```bash
# Run docs detection unit tests
cargo test --package quench checks::git::docs

# Run all git check tests
cargo test --package quench checks::git
```

### Behavioral Specs

```bash
# Run git check specs
cargo test --test specs git

# Show remaining ignored specs (fix phase)
cargo test --test specs git -- --ignored
```

### Full Suite

```bash
# Run complete check suite
make check
```

## Checklist

- [ ] Create `crates/cli/src/checks/git/docs.rs` with detection logic
- [ ] Add `has_commit_documentation()` function
- [ ] Add `has_type_prefix()` function with regex
- [ ] Add `has_conventional_commits_phrase()` function
- [ ] Add `check_commit_docs()` function
- [ ] Add `primary_agent_file()` helper
- [ ] Update `mod.rs` to add `mod docs;`
- [ ] Add `check_agent_docs()` function to `mod.rs`
- [ ] Call `check_agent_docs()` in `GitCheck::run()`
- [ ] Create `docs_tests.rs` with unit tests
- [ ] Create `tests/fixtures/git/missing-docs/` fixture
- [ ] Remove `#[ignore]` from `git_missing_format_documentation_generates_violation`
- [ ] Remove `#[ignore]` from `git_detects_commit_format_via_type_prefixes`
- [ ] Remove `#[ignore]` from `git_detects_commit_format_via_conventional_commits_phrase`
- [ ] Remove `#[ignore]` from `git_skips_docs_check_when_agents_disabled`
- [ ] Remove `#[ignore]` from `git_missing_docs_violation_references_file`
- [ ] Run `make check` - all tests pass

## Deliverables

This phase produces:
1. **Detection module**: `docs.rs` with commit documentation detection
2. **Integration**: Git check calls agent docs check when enabled
3. **Violation generation**: `missing_docs` violation with actionable advice
4. **Test coverage**: Unit tests and enabled behavioral specs

The agent documentation check is ready for Phase 820+ to implement template creation (`--fix` creates `.gitmessage`).
