# Phase 801: Git Check - Specs

**Root Feature:** `quench-3153`

## Overview

Write behavioral specifications (black-box tests) for the `git` check feature. These specs validate commit message format enforcement, type/scope restrictions, agent documentation requirements, and `--fix` template creation behavior. All specs will be marked with `#[ignore = "TODO: Phase N"]` since the git check is not yet implemented.

Reference: `docs/specs/checks/git.md`

## Project Structure

```
tests/
├── specs/
│   └── checks/
│       └── git.rs              # NEW: Git check behavioral specs
├── fixtures/
│   └── git/                    # NEW: Test fixtures for git check
│       ├── conventional-ok/    # Valid conventional commit messages
│       ├── invalid-format/     # Missing type prefix
│       ├── invalid-type/       # Type not in allowed list
│       ├── invalid-scope/      # Scope not in allowed list
│       ├── missing-docs/       # No commit format in CLAUDE.md
│       └── with-docs/          # Has commit format documentation
└── specs.rs                    # Add git module declaration
```

## Dependencies

No new external dependencies. Uses existing test infrastructure:
- `assert_cmd` for CLI invocation
- `tempfile` for `Project::empty()` temp directories
- Existing prelude helpers (`check()`, `cli()`, etc.)

## Implementation Phases

### Phase 1: Create Spec File Structure

Create `tests/specs/checks/git.rs` with module header and imports.

```rust
//! Behavioral specs for the git check.
//!
//! Tests that quench correctly:
//! - Validates commit message format (conventional commits)
//! - Enforces type restrictions
//! - Enforces scope restrictions (when configured)
//! - Checks for commit format documentation in agent files
//! - Creates .gitmessage template with --fix
//!
//! Reference: docs/specs/checks/git.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
```

Add to `tests/specs.rs`:
```rust
#[path = "specs/checks/git.rs"]
mod checks_git;
```

**Milestone:** Spec file compiles with `cargo test --test specs`.

### Phase 2: Create Test Fixtures

Create minimal fixture directories for git check scenarios.

**`tests/fixtures/git/conventional-ok/`**
```
quench.toml:
  version = 1
  [git.commit]
  check = "error"

CLAUDE.md:
  # Project

  ## Commits

  Use conventional commit format: `type(scope): description`
  Types: feat, fix, chore, docs, test, refactor

  ## Directory Structure

  Minimal.

  ## Landing the Plane

  - Done
```

**`tests/fixtures/git/missing-docs/`**
```
quench.toml:
  version = 1
  [git.commit]
  check = "error"
  agents = true

CLAUDE.md:
  # Project

  (No commit format documentation)

  ## Directory Structure

  Minimal.

  ## Landing the Plane

  - Done
```

**Note:** Commit message fixtures require actual git repos. Most tests will use `Project::empty()` with mock git state or test the documentation check which doesn't need git history.

**Milestone:** Fixture directories created and valid.

### Phase 3: Commit Format Validation Specs

Write specs for commit message format validation.

```rust
// =============================================================================
// COMMIT FORMAT VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#conventional-format
///
/// > When `format = "conventional"`, commits must match: <type>(<scope>): <description>
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_validates_conventional_commit_format() {
    // Valid format should pass
    check("git").on("git/conventional-ok").passes();
}

/// Spec: docs/specs/checks/git.md#output
///
/// > abc123: "update stuff" - missing type prefix
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_invalid_format_generates_violation() {
    let git = check("git").on("git/invalid-format").json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_format")),
        "should have invalid_format violation"
    );
}
```

**Milestone:** Format validation specs compile.

### Phase 4: Type and Scope Restriction Specs

Write specs for type and scope validation.

```rust
// =============================================================================
// TYPE RESTRICTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#types
///
/// > `["feat", "fix"]` - Only these types allowed
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_invalid_type_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
types = ["feat", "fix"]
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: or fix: only\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );
    // Simulate commit with disallowed type "chore"
    // (Actual git state mocking TBD in implementation phase)

    let git = check("git").pwd(temp.path()).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_type")),
        "should have invalid_type violation"
    );
}

// =============================================================================
// SCOPE RESTRICTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#scopes
///
/// > `["api", "cli"]` - Only these scopes allowed
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_invalid_scope_generates_violation_when_scopes_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
scopes = ["api", "cli"]
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat(api): or feat(cli): only\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );
    // Simulate commit with disallowed scope "unknown"

    let git = check("git").pwd(temp.path()).json().fails();
    let violations = git.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_scope")),
        "should have invalid_scope violation"
    );
}

/// Spec: docs/specs/checks/git.md#scopes
///
/// > omitted - Any scope allowed (or none)
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_any_scope_allowed_when_not_configured() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
# scopes not specified - any scope allowed
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat(anything): allowed\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    check("git").pwd(temp.path()).passes();
}
```

**Milestone:** Type and scope specs compile.

### Phase 5: Agent Documentation Specs

Write specs for CLAUDE.md documentation requirements.

```rust
// =============================================================================
// AGENT DOCUMENTATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#agent-documentation-check
///
/// > When `agents = true` (default), quench verifies that commit format
/// > is documented in agent-readable files.
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_missing_format_documentation_generates_violation() {
    check("git")
        .on("git/missing-docs")
        .json()
        .fails()
        .stdout_has("missing_docs");
}

/// Spec: docs/specs/checks/git.md#detection
///
/// > Searches for type prefixes followed by `:` or `(` (e.g., `feat:`, `fix(`)
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_detects_commit_format_via_type_prefixes() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nUse `feat:` or `fix(scope):` format.\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    check("git").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/git.md#detection
///
/// > Searches for the phrase "conventional commits" (case-insensitive)
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_detects_commit_format_via_conventional_commits_phrase() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nWe use Conventional Commits.\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    check("git").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/git.md#disable
///
/// > agents = false - Don't check CLAUDE.md
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_skips_docs_check_when_agents_disabled() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n(No commit docs)\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Should pass even without commit format documentation
    check("git").pwd(temp.path()).passes();
}
```

**Milestone:** Agent documentation specs compile.

### Phase 6: Fix Behavior Specs

Write specs for `--fix` template creation.

```rust
// =============================================================================
// FIX BEHAVIOR SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#template-creation
///
/// > When `template = true` (default), `--fix` creates a `.gitmessage` file.
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_fix_creates_gitmessage_template() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Initialize git repo for template config
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify .gitmessage was created
    let gitmessage_path = temp.path().join(".gitmessage");
    assert!(gitmessage_path.exists(), ".gitmessage should be created");

    let content = std::fs::read_to_string(&gitmessage_path).unwrap();
    assert!(
        content.contains("# <type>"),
        ".gitmessage should contain template"
    );
}

/// Spec: docs/specs/checks/git.md#git-config
///
/// > `--fix` also runs: git config commit.template .gitmessage
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_fix_configures_commit_template() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify git config was set
    let output = std::process::Command::new("git")
        .args(["config", "commit.template"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    let template_value = String::from_utf8_lossy(&output.stdout);
    assert!(
        template_value.trim() == ".gitmessage",
        "commit.template should be set to .gitmessage"
    );
}

/// Spec: docs/specs/checks/git.md#behavior
///
/// > `.gitmessage` exists - Leave it alone
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_fix_does_not_overwrite_existing_gitmessage() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
template = true
"#,
    );
    temp.file(
        "CLAUDE.md",
        "# Project\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
    );
    temp.file(".gitmessage", "# Custom template\n");

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    check("git").pwd(temp.path()).args(&["--fix"]).passes();

    // Verify original content preserved
    let content = std::fs::read_to_string(temp.path().join(".gitmessage")).unwrap();
    assert_eq!(content, "# Custom template\n");
}
```

**Milestone:** Fix behavior specs compile.

### Phase 7: Violation Type and JSON Output Specs

Write specs for violation types and JSON structure.

```rust
// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: docs/specs/checks/git.md#json-output
///
/// > Violation types: `invalid_format`, `invalid_type`, `invalid_scope`, `missing_docs`
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_violation_type_is_one_of_expected_values() {
    // Use a fixture that triggers violations
    let git = check("git").on("git/invalid-format").json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let valid_types = ["invalid_format", "invalid_type", "invalid_scope", "missing_docs"];

    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

/// Spec: docs/specs/checks/git.md#json-output
///
/// > Commit-related violations have `file: null` with `commit` field instead.
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_commit_violations_have_commit_field() {
    let git = check("git").on("git/invalid-format").json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let commit_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("invalid_format"))
        .expect("should have invalid_format violation");

    assert!(
        commit_violation.get("commit").is_some(),
        "commit violation should have commit field"
    );
    assert!(
        commit_violation.get("file").map(|f| f.is_null()).unwrap_or(true),
        "commit violation should have null file"
    );
}

/// Spec: docs/specs/checks/git.md#json-output
///
/// > missing_docs violations reference the agent file
#[test]
#[ignore = "TODO: Phase 802 - Git Check Implementation"]
fn git_missing_docs_violation_references_file() {
    let git = check("git").on("git/missing-docs").json().fails();
    let violations = git.require("violations").as_array().unwrap();

    let docs_violation = violations
        .iter()
        .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("missing_docs"))
        .expect("should have missing_docs violation");

    assert!(
        docs_violation
            .get("file")
            .and_then(|f| f.as_str())
            .is_some(),
        "missing_docs violation should reference file"
    );
}
```

**Milestone:** All specs compile and are properly ignored.

## Key Implementation Details

### Test Fixture Requirements

Most tests use `Project::empty()` with inline content. Static fixtures are needed for:
- `git/missing-docs` - Agent file without commit documentation
- `git/conventional-ok` - Valid setup for passing tests (may need git mocking)

### Git Repository Mocking

Commit message validation requires git history. Options for implementation phase:
1. Use `git2` crate to programmatically create commits
2. Shell out to `git commit --allow-empty -m "message"`
3. Mock git command output via environment variable

The spec tests should be written to work with whichever approach is chosen.

### Violation Output Format

Text format (from spec):
```
git: FAIL
  abc123: "update stuff" - missing type prefix
    Expected: <type>(<scope>): <description>
```

JSON format:
```json
{
  "file": null,
  "line": null,
  "type": "invalid_format",
  "commit": "abc123",
  "message": "update stuff",
  "advice": "Expected: <type>(<scope>): <description>"
}
```

## Verification Plan

### Compile Check

```bash
# All specs should compile (tests are ignored)
cargo test --test specs -- git --list
```

### Ignored Test Count

```bash
# Show ignored tests for git check
cargo test --test specs -- git --ignored --list
```

Expected output should show 7+ ignored tests matching the spec outline.

### Full Spec Suite

```bash
# Verify no regressions in other specs
cargo test --test specs

# Run make check
make check
```

## Checklist

- [ ] Create `tests/specs/checks/git.rs`
- [ ] Add `checks_git` module to `tests/specs.rs`
- [ ] Create `tests/fixtures/git/missing-docs/` fixture
- [ ] Write spec: validates commit message format
- [ ] Write spec: invalid type generates violation
- [ ] Write spec: invalid scope generates violation (when scopes configured)
- [ ] Write spec: missing format documentation generates violation
- [ ] Write spec: --fix creates .gitmessage template
- [ ] Write spec: --fix configures git commit.template
- [ ] Write spec: violation.type is one of expected values
- [ ] Verify all specs compile with `#[ignore]`
- [ ] Run `make check` to verify no regressions
