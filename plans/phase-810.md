# Phase 810: Git Check - Validation

**Root Feature:** `quench-3153`
**Depends On:** Phase 805 (Git Check - Message Parsing)

## Overview

Implement commit message validation for the git check. This phase uses the parsing infrastructure from Phase 805 to validate commits against configuration, generating appropriate violations for invalid format, type, and scope.

The validation layer:
- **Format validation**: Ensures commits match `type:` or `type(scope):` structure
- **Type validation**: Checks type against allowed list (default or configured)
- **Scope validation**: Checks scope against configured list (if specified)
- **Violation generation**: Creates appropriate violations for each failure mode

This phase does NOT include agent documentation checking or template creation (future phases).

## Project Structure

```
crates/cli/src/
├── config/
│   └── mod.rs                # EXTEND: Expand GitCommitConfig
├── checks/
│   └── git/
│       ├── mod.rs            # EXTEND: Implement validation logic
│       └── mod_tests.rs      # EXTEND: Add validation tests
tests/
├── specs/checks/git.rs       # UPDATE: Remove #[ignore] from validation specs
└── fixtures/git/
    ├── invalid-type/         # NEW: Fixture for type validation
    └── invalid-scope/        # NEW: Fixture for scope validation
```

## Dependencies

No new external dependencies. Uses existing:
- `regex` (for parsing, already used)
- Git utilities from `git.rs` (already implemented)

## Implementation Phases

### Phase 1: Expand GitCommitConfig

Extend `crates/cli/src/config/mod.rs` to include all validation options.

```rust
/// Git commit message configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitCommitConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Commit format: "conventional" | "none" (default: "conventional")
    pub format: Option<String>,

    /// Allowed commit types (None = use defaults, Some([]) = any type)
    pub types: Option<Vec<String>>,

    /// Allowed scopes (None = any scope allowed)
    pub scopes: Option<Vec<String>>,

    /// Check that commit format is documented in agent files (default: true)
    #[serde(default = "GitCommitConfig::default_agents")]
    pub agents: bool,

    /// Create .gitmessage template with --fix (default: true)
    #[serde(default = "GitCommitConfig::default_template")]
    pub template: bool,
}

impl GitCommitConfig {
    fn default_agents() -> bool {
        true
    }

    fn default_template() -> bool {
        true
    }

    /// Get effective format (default: "conventional").
    pub fn effective_format(&self) -> &str {
        self.format.as_deref().unwrap_or("conventional")
    }
}
```

**Milestone:** Config parses all git.commit fields without errors.

### Phase 2: Implement Validation Logic

Update `crates/cli/src/checks/git/mod.rs` to validate commits.

```rust
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::git::{get_all_branch_commits, get_commits_since, Commit};

use parse::{parse_conventional_commit, ParseResult, DEFAULT_TYPES};

impl Check for GitCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Skip if not in a git repository
        if !is_git_repo(ctx.root) {
            return CheckResult::skipped(self.name(), "Not a git repository");
        }

        // Get check configuration
        let config = &ctx.config.git.commit;

        // Skip if check is disabled
        if config.check.as_deref() == Some("off") {
            return CheckResult::skipped(self.name(), "Check disabled");
        }

        // Skip format validation if format = "none"
        if config.effective_format() == "none" {
            return CheckResult::passed(self.name());
        }

        // Get commits to validate
        let commits = match get_commits_to_check(ctx) {
            Ok(commits) => commits,
            Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
        };

        // Skip if no commits to check
        if commits.is_empty() {
            return CheckResult::passed(self.name());
        }

        // Validate each commit
        let mut violations = Vec::new();
        for commit in &commits {
            validate_commit(commit, config, &mut violations);
        }

        if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        }
    }
}

/// Get commits to validate based on context.
fn get_commits_to_check(ctx: &CheckContext) -> anyhow::Result<Vec<Commit>> {
    // Staged mode: no commit message to check yet
    if ctx.staged {
        return Ok(Vec::new());
    }

    // CI mode or explicit base: check commits on branch
    if ctx.ci_mode {
        get_all_branch_commits(ctx.root)
    } else if let Some(base) = ctx.base_branch {
        get_commits_since(ctx.root, base)
    } else {
        // No base specified, no commits to check
        Ok(Vec::new())
    }
}

/// Validate a single commit and add violations if invalid.
fn validate_commit(
    commit: &Commit,
    config: &GitCommitConfig,
    violations: &mut Vec<Violation>,
) {
    match parse_conventional_commit(&commit.message) {
        ParseResult::NonConventional => {
            violations.push(Violation::commit_violation(
                &commit.hash,
                &commit.message,
                "invalid_format",
                "Expected: <type>(<scope>): <description>",
            ));
        }
        ParseResult::Conventional(parsed) => {
            // Check type
            let allowed_types = config.types.as_deref();
            if !parsed.is_type_allowed(allowed_types) {
                let advice = format_type_advice(allowed_types);
                violations.push(Violation::commit_violation(
                    &commit.hash,
                    &commit.message,
                    "invalid_type",
                    advice,
                ));
            }

            // Check scope (only if scopes are configured)
            if let Some(scopes) = config.scopes.as_ref() {
                if !parsed.is_scope_allowed(Some(scopes)) {
                    let advice = format!("Allowed scopes: {}", scopes.join(", "));
                    violations.push(Violation::commit_violation(
                        &commit.hash,
                        &commit.message,
                        "invalid_scope",
                        advice,
                    ));
                }
            }
        }
    }
}

/// Format advice for invalid type violations.
fn format_type_advice(allowed_types: Option<&[String]>) -> String {
    match allowed_types {
        None => format!("Allowed types: {}", DEFAULT_TYPES.join(", ")),
        Some(types) if types.is_empty() => "Any type allowed (check format only)".to_string(),
        Some(types) => format!("Allowed types: {}", types.join(", ")),
    }
}
```

**Milestone:** Commit validation runs and generates violations.

### Phase 3: Add Test Fixtures

Create test fixtures for different validation scenarios.

**`tests/fixtures/git/invalid-type/quench.toml`:**
```toml
version = 1

[git.commit]
check = "error"
types = ["feat", "fix"]
```

**`tests/fixtures/git/invalid-type/CLAUDE.md`:**
```markdown
# Project

## Commits

Use feat: or fix: format.

## Directory Structure

Minimal.

## Landing the Plane

- Done
```

**`tests/fixtures/git/invalid-scope/quench.toml`:**
```toml
version = 1

[git.commit]
check = "error"
scopes = ["api", "cli"]
```

**`tests/fixtures/git/invalid-scope/CLAUDE.md`:**
```markdown
# Project

## Commits

Use feat(api): or feat(cli): format.

## Directory Structure

Minimal.

## Landing the Plane

- Done
```

Note: Fixtures need git repos with actual commits. The specs will use temp directories to create controlled git state.

**Milestone:** Fixtures exist and can be used by specs.

### Phase 4: Unit Tests for Validation

Add unit tests to `crates/cli/src/checks/git/mod_tests.rs`.

```rust
//! Unit tests for git check validation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::config::GitCommitConfig;
use crate::git::Commit;

// =============================================================================
// FORMAT VALIDATION TESTS
// =============================================================================

#[test]
fn validates_conventional_format() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat: add feature".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_non_conventional_format() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "update stuff".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_format");
    assert_eq!(violations[0].commit, Some("abc1234".to_string()));
}

// =============================================================================
// TYPE VALIDATION TESTS
// =============================================================================

#[test]
fn accepts_default_type() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat: add feature".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_invalid_type_with_defaults() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "custom: do something".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_type");
}

#[test]
fn accepts_custom_type_when_configured() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "custom: do something".to_string(),
    };
    let mut config = GitCommitConfig::default();
    config.types = Some(vec!["custom".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn any_type_allowed_with_empty_list() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "anything: do something".to_string(),
    };
    let mut config = GitCommitConfig::default();
    config.types = Some(vec![]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

// =============================================================================
// SCOPE VALIDATION TESTS
// =============================================================================

#[test]
fn any_scope_allowed_when_not_configured() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat(random): add feature".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn accepts_configured_scope() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat(api): add endpoint".to_string(),
    };
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string(), "cli".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_invalid_scope() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat(unknown): add feature".to_string(),
    };
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string(), "cli".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_scope");
}

#[test]
fn no_scope_allowed_when_scopes_configured() {
    // Commits without scope are allowed even when scopes are configured
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat: add feature".to_string(),
    };
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

// =============================================================================
// CONFIG TESTS
// =============================================================================

#[test]
fn effective_format_defaults_to_conventional() {
    let config = GitCommitConfig::default();
    assert_eq!(config.effective_format(), "conventional");
}

#[test]
fn effective_format_respects_config() {
    let mut config = GitCommitConfig::default();
    config.format = Some("none".to_string());
    assert_eq!(config.effective_format(), "none");
}
```

**Milestone:** All unit tests pass.

### Phase 5: Enable Behavioral Specs

Update `tests/specs/checks/git.rs` to remove `#[ignore]` from validation specs.

Specs to enable:
- `git_validates_conventional_commit_format` - format validation
- `git_invalid_format_generates_violation` - format violation
- `git_invalid_type_generates_violation` - type violation
- `git_invalid_scope_generates_violation_when_scopes_configured` - scope violation
- `git_any_scope_allowed_when_not_configured` - scope permissiveness
- `git_violation_type_is_one_of_expected_values` - JSON output
- `git_commit_violations_have_commit_field` - JSON structure

Note: Agent documentation specs and fix specs remain ignored (future phases).

**Milestone:** Enabled specs pass with `cargo test --test specs git`.

### Phase 6: Integration Testing

Verify the git check works end-to-end with real git repositories.

```bash
# Create a test repo with invalid commits
mkdir -p /tmp/git-test && cd /tmp/git-test
git init
echo "version = 1\n[git.commit]\ncheck = \"error\"" > quench.toml
echo "# Test\n\n## Commits\n\nfeat: format\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done" > CLAUDE.md
git add .
git commit -m "update stuff"  # Invalid format!

# Run quench
quench check --ci  # Should fail with invalid_format

# Fix the commit and verify
git commit --amend -m "feat: initial setup"
quench check --ci  # Should pass
```

**Milestone:** Real git repos validate correctly.

## Key Implementation Details

### Commit Retrieval Logic

The validation uses different commit retrieval strategies based on context:

| Mode | Behavior |
|------|----------|
| `--staged` | Skip validation (no commit yet) |
| `--ci` | Validate all branch commits via `get_all_branch_commits()` |
| `--base <ref>` | Validate commits since ref via `get_commits_since()` |
| Default (no flags) | Skip validation (no base to compare) |

This ensures validation only runs when there are commits to check and the scope is well-defined.

### Type Validation Modes

Three validation modes based on `types` configuration:

| Config | Behavior |
|--------|----------|
| Not set | Use DEFAULT_TYPES (conventional commit standard) |
| `[]` (empty) | Any type allowed (structure-only check) |
| `["feat", "fix"]` | Only listed types allowed |

### Scope Validation

| Config | Behavior |
|--------|----------|
| Not set | Any scope (or no scope) allowed |
| `["api", "cli"]` | Only listed scopes allowed, but no scope is always allowed |

### Violation JSON Structure

Commit violations use `commit` and `message` fields instead of `file`:

```json
{
  "type": "invalid_format",
  "commit": "abc1234",
  "message": "update stuff",
  "advice": "Expected: <type>(<scope>): <description>"
}
```

## Verification Plan

### Unit Tests

```bash
# Run validation unit tests
cargo test --package quench checks::git::tests

# Run parser tests (from Phase 805)
cargo test --package quench checks::git::parse
```

### Behavioral Specs

```bash
# Run git check specs
cargo test --test specs git

# Show remaining ignored specs
cargo test --test specs git -- --ignored
```

### Full Suite

```bash
# Run complete check suite
make check
```

## Checklist

- [ ] Expand `GitCommitConfig` with format, types, scopes, agents, template fields
- [ ] Add `effective_format()` helper method
- [ ] Implement `get_commits_to_check()` function
- [ ] Implement `validate_commit()` function
- [ ] Implement `format_type_advice()` helper
- [ ] Update `GitCheck::run()` to call validation logic
- [ ] Add unit tests for format validation
- [ ] Add unit tests for type validation
- [ ] Add unit tests for scope validation
- [ ] Create invalid-type fixture
- [ ] Create invalid-scope fixture
- [ ] Remove `#[ignore]` from format validation specs
- [ ] Remove `#[ignore]` from type validation specs
- [ ] Remove `#[ignore]` from scope validation specs
- [ ] Remove `#[ignore]` from JSON output specs
- [ ] Run `make check` - all tests pass

## Deliverables

This phase produces:
1. **Expanded config**: GitCommitConfig with all validation options
2. **Validation logic**: Commit validation against format, type, and scope rules
3. **Violation generation**: Proper violation objects for each failure mode
4. **Test coverage**: Unit tests and enabled behavioral specs

The validation layer is ready for Phase 815+ to implement agent documentation checking and template creation.
