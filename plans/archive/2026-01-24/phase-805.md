# Phase 805: Git Check - Message Parsing

**Root Feature:** `quench-3153`
**Depends On:** Phase 801 (Git Check Specs)

## Overview

Implement the commit message parsing layer for the git check. This phase adds the infrastructure to extract commit messages from git history and parse them according to the conventional commit format (`type(scope): description`). This parsing foundation will be used by later phases to validate commits against configuration.

The parser extracts:
- **Type**: The commit type prefix (`feat`, `fix`, `chore`, etc.)
- **Scope**: Optional scope in parentheses (`api`, `cli`, etc.)
- **Description**: The commit description after the colon

## Project Structure

```
crates/cli/src/
├── git.rs                    # EXTEND: Add commit message retrieval
├── checks/
│   ├── git.rs               # EXTEND: Add parsing module import
│   ├── git_tests.rs         # EXTEND: Add parsing unit tests
│   └── git/                 # NEW: Git check submodules
│       ├── mod.rs           # Module re-exports
│       └── parse.rs         # Commit message parsing
│       └── parse_tests.rs   # Parser unit tests
```

## Dependencies

No new external dependencies. Uses:
- `regex` (already in workspace for other checks)
- `once_cell::Lazy` (already used for regex patterns)

## Implementation Phases

### Phase 1: Commit Message Retrieval

Extend `crates/cli/src/git.rs` with functions to retrieve commit messages.

```rust
/// A commit with its hash and message.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Short commit hash (7 characters).
    pub hash: String,
    /// Full commit message (subject line only).
    pub message: String,
}

/// Get commits since a base ref.
///
/// Returns commits from newest to oldest.
pub fn get_commits_since(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    let output = Command::new("git")
        .args([
            "log",
            "--format=%h%n%s",  // Short hash, newline, subject
            &format!("{}..HEAD", base),
        ])
        .current_dir(root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed: {}", stderr.trim());
    }

    parse_git_log_output(&String::from_utf8_lossy(&output.stdout))
}

/// Parse git log output with format "%h%n%s".
fn parse_git_log_output(output: &str) -> anyhow::Result<Vec<Commit>> {
    let lines: Vec<&str> = output.lines().collect();
    let mut commits = Vec::new();

    // Process pairs of lines (hash, message)
    for chunk in lines.chunks(2) {
        if chunk.len() == 2 && !chunk[0].is_empty() {
            commits.push(Commit {
                hash: chunk[0].to_string(),
                message: chunk[1].to_string(),
            });
        }
    }

    Ok(commits)
}

/// Get all commits on current branch (for CI mode).
pub fn get_all_branch_commits(root: &Path) -> anyhow::Result<Vec<Commit>> {
    // Detect base and delegate
    if let Some(base) = detect_base_branch(root) {
        get_commits_since(root, &base)
    } else {
        // No base branch found, get all commits
        let output = Command::new("git")
            .args(["log", "--format=%h%n%s"])
            .current_dir(root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git log failed: {}", stderr.trim());
        }

        parse_git_log_output(&String::from_utf8_lossy(&output.stdout))
    }
}
```

**Milestone:** `get_commits_since` returns commit hashes and messages.

### Phase 2: Create Parse Module Structure

Create `crates/cli/src/checks/git/mod.rs` and `parse.rs`.

**`crates/cli/src/checks/git/mod.rs`:**
```rust
//! Git check submodules.

pub mod parse;

pub use parse::{parse_conventional_commit, ParsedCommit};
```

**`crates/cli/src/checks/git/parse.rs`:**
```rust
//! Conventional commit message parsing.
//!
//! Parses commit messages in the format: `<type>(<scope>): <description>`
//! where scope is optional.

use once_cell::sync::Lazy;
use regex::Regex;

/// Pattern for conventional commit format.
///
/// Captures:
/// - Group 1: type (required)
/// - Group 2: scope with parens (optional)
/// - Group 3: scope without parens (optional)
/// - Group 4: description (required)
static CONVENTIONAL_COMMIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([a-z]+)(\(([^)]+)\))?:\s*(.+)$").unwrap()
});

/// A parsed conventional commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommit {
    /// Commit type (e.g., "feat", "fix").
    pub commit_type: String,
    /// Optional scope (e.g., "api", "cli").
    pub scope: Option<String>,
    /// Commit description.
    pub description: String,
}

/// Parse result for a commit message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// Successfully parsed conventional commit.
    Conventional(ParsedCommit),
    /// Message does not match conventional format.
    NonConventional,
}

/// Parse a commit message as a conventional commit.
///
/// Returns `ParseResult::Conventional` if the message matches the format,
/// or `ParseResult::NonConventional` if it doesn't.
///
/// # Examples
///
/// ```
/// use quench::checks::git::parse::parse_conventional_commit;
///
/// let result = parse_conventional_commit("feat(api): add endpoint");
/// assert!(matches!(result, ParseResult::Conventional(_)));
///
/// let result = parse_conventional_commit("update stuff");
/// assert!(matches!(result, ParseResult::NonConventional));
/// ```
pub fn parse_conventional_commit(message: &str) -> ParseResult {
    if let Some(caps) = CONVENTIONAL_COMMIT.captures(message) {
        let commit_type = caps.get(1).map(|m| m.as_str().to_string()).unwrap();
        let scope = caps.get(3).map(|m| m.as_str().to_string());
        let description = caps.get(4).map(|m| m.as_str().to_string()).unwrap();

        ParseResult::Conventional(ParsedCommit {
            commit_type,
            scope,
            description,
        })
    } else {
        ParseResult::NonConventional
    }
}
```

**Milestone:** Parse module compiles with basic structure.

### Phase 3: Type Extraction and Validation

Add helper methods for type validation.

```rust
// Add to parse.rs

/// Default conventional commit types.
pub const DEFAULT_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style",
];

impl ParsedCommit {
    /// Check if the commit type is in the allowed list.
    ///
    /// If `allowed_types` is empty, any type is accepted (structure-only validation).
    /// If `allowed_types` is `None`, default types are used.
    pub fn is_type_allowed(&self, allowed_types: Option<&[String]>) -> bool {
        match allowed_types {
            None => DEFAULT_TYPES.contains(&self.commit_type.as_str()),
            Some(types) if types.is_empty() => true, // Empty = any type
            Some(types) => types.iter().any(|t| t == &self.commit_type),
        }
    }
}
```

**Milestone:** Type validation works with default and custom type lists.

### Phase 4: Scope Extraction and Validation

Add helper methods for scope validation.

```rust
// Add to parse.rs

impl ParsedCommit {
    /// Check if the scope is in the allowed list.
    ///
    /// If `allowed_scopes` is `None`, any scope (or no scope) is accepted.
    /// If `allowed_scopes` is `Some(&[])`, no scopes are allowed.
    /// If `allowed_scopes` is `Some(&[...])`, only those scopes are allowed.
    pub fn is_scope_allowed(&self, allowed_scopes: Option<&[String]>) -> bool {
        match (allowed_scopes, &self.scope) {
            // No scope restriction configured
            (None, _) => true,
            // Scope restriction configured, but no scope in commit
            (Some(_), None) => true, // Allow commits without scope
            // Scope restriction configured and scope present
            (Some(scopes), Some(scope)) => scopes.iter().any(|s| s == scope),
        }
    }

    /// Get the scope if present, for error reporting.
    pub fn scope_str(&self) -> Option<&str> {
        self.scope.as_deref()
    }
}
```

**Milestone:** Scope validation works with configured scope lists.

### Phase 5: Parser Unit Tests

Create `crates/cli/src/checks/git/parse_tests.rs` with comprehensive tests.

```rust
//! Unit tests for conventional commit parsing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// BASIC PARSING TESTS
// =============================================================================

#[test]
fn parses_type_and_description() {
    let result = parse_conventional_commit("fix: handle empty input");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.commit_type, "fix");
    assert_eq!(parsed.scope, None);
    assert_eq!(parsed.description, "handle empty input");
}

#[test]
fn parses_type_scope_and_description() {
    let result = parse_conventional_commit("feat(api): add export endpoint");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.commit_type, "feat");
    assert_eq!(parsed.scope, Some("api".to_string()));
    assert_eq!(parsed.description, "add export endpoint");
}

#[test]
fn rejects_message_without_type_prefix() {
    let result = parse_conventional_commit("update stuff");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn rejects_message_without_colon() {
    let result = parse_conventional_commit("feat add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn rejects_message_with_uppercase_type() {
    // Conventional commits use lowercase types
    let result = parse_conventional_commit("FEAT: add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn handles_description_with_colons() {
    let result = parse_conventional_commit("docs: update README: add examples");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.description, "update README: add examples");
}

#[test]
fn handles_empty_scope_parentheses() {
    // Empty parens should be rejected (no scope)
    let result = parse_conventional_commit("feat(): add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn handles_scope_with_hyphen() {
    let result = parse_conventional_commit("fix(user-auth): resolve login issue");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope, Some("user-auth".to_string()));
}

#[test]
fn handles_scope_with_underscore() {
    let result = parse_conventional_commit("feat(user_settings): add theme option");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope, Some("user_settings".to_string()));
}

#[test]
fn handles_minimal_description() {
    let result = parse_conventional_commit("fix: x");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.description, "x");
}

#[test]
fn trims_description_whitespace() {
    let result = parse_conventional_commit("fix:   lots of spaces   ");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    // Regex captures everything after colon+space, but leading space is in \s*
    assert!(parsed.description.starts_with("lots"));
}

// =============================================================================
// TYPE VALIDATION TESTS
// =============================================================================

#[test]
fn default_types_accepted() {
    for commit_type in DEFAULT_TYPES {
        let msg = format!("{}: test", commit_type);
        let result = parse_conventional_commit(&msg);
        let ParseResult::Conventional(parsed) = result else {
            panic!("expected Conventional for {}", commit_type);
        };
        assert!(
            parsed.is_type_allowed(None),
            "{} should be in default types",
            commit_type
        );
    }
}

#[test]
fn custom_type_rejected_with_defaults() {
    let result = parse_conventional_commit("custom: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert!(!parsed.is_type_allowed(None));
}

#[test]
fn custom_type_accepted_with_empty_list() {
    let result = parse_conventional_commit("custom: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let empty: Vec<String> = vec![];
    assert!(parsed.is_type_allowed(Some(&empty)));
}

#[test]
fn type_checked_against_custom_list() {
    let result = parse_conventional_commit("feat: add feature");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let allowed = vec!["feat".to_string(), "fix".to_string()];
    assert!(parsed.is_type_allowed(Some(&allowed)));

    let not_allowed = vec!["fix".to_string()];
    assert!(!parsed.is_type_allowed(Some(&not_allowed)));
}

// =============================================================================
// SCOPE VALIDATION TESTS
// =============================================================================

#[test]
fn any_scope_allowed_when_not_configured() {
    let result = parse_conventional_commit("feat(random): something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert!(parsed.is_scope_allowed(None));
}

#[test]
fn no_scope_allowed_when_scopes_configured() {
    let result = parse_conventional_commit("feat: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let scopes = vec!["api".to_string()];
    assert!(parsed.is_scope_allowed(Some(&scopes)));
}

#[test]
fn scope_checked_against_configured_list() {
    let result = parse_conventional_commit("feat(api): add endpoint");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };

    let allowed = vec!["api".to_string(), "cli".to_string()];
    assert!(parsed.is_scope_allowed(Some(&allowed)));

    let not_allowed = vec!["cli".to_string()];
    assert!(!parsed.is_scope_allowed(Some(&not_allowed)));
}
```

Add test module reference to `parse.rs`:
```rust
#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
```

**Milestone:** All parser unit tests pass.

### Phase 6: Integration with Git Check

Update `crates/cli/src/checks/git.rs` to use the parse module.

```rust
// Add module declaration at top
mod git;
pub use git::{parse_conventional_commit, ParsedCommit};

// Update GitCheck::run to use parsing (still returning stub, but with import)
impl Check for GitCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Check if we're in a git repository
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(ctx.root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                // We're in a git repo
                // TODO (Phase 806+): Use parse module for validation
                CheckResult::stub(self.name())
            }
            _ => CheckResult::skipped(self.name(), "Not a git repository"),
        }
    }
}
```

**Milestone:** Git check imports parse module; all tests pass.

## Key Implementation Details

### Regex Pattern

The conventional commit regex:
```
^([a-z]+)(\(([^)]+)\))?:\s*(.+)$
```

Breakdown:
- `^` - Start of string
- `([a-z]+)` - Type: one or more lowercase letters (group 1)
- `(\(([^)]+)\))?` - Optional scope with parens (group 2), scope without parens (group 3)
- `:` - Literal colon separator
- `\s*` - Optional whitespace after colon
- `(.+)` - Description: one or more characters (group 4)
- `$` - End of string

### Git Log Format

Using `%h%n%s` format:
- `%h` - Abbreviated commit hash (7 chars)
- `%n` - Newline
- `%s` - Subject line (first line of message)

This avoids parsing multi-line commit bodies and keeps output predictable.

### Type Validation Strategy

Three modes:
1. `types = None` - Use DEFAULT_TYPES (conventional commit standard)
2. `types = Some([])` - Any type allowed (structure-only check)
3. `types = Some([...])` - Only listed types allowed

### Scope Validation Strategy

Two modes:
1. `scopes = None` - Any scope (or no scope) allowed
2. `scopes = Some([...])` - Only listed scopes allowed

Note: Commits without a scope are always allowed, even when scopes are configured.

## Verification Plan

### Unit Tests

```bash
# Run parser unit tests
cargo test --package quench checks::git::parse

# Run all git check tests
cargo test --package quench git
```

### Integration Tests

```bash
# Verify git utilities work
cargo test --package quench git::

# Verify git check still compiles
cargo test --test specs -- git --list
```

### Full Suite

```bash
# Run complete check suite
make check
```

## Checklist

- [ ] Add `Commit` struct to `crates/cli/src/git.rs`
- [ ] Add `get_commits_since` function to `git.rs`
- [ ] Add `get_all_branch_commits` function to `git.rs`
- [ ] Create `crates/cli/src/checks/git/mod.rs`
- [ ] Create `crates/cli/src/checks/git/parse.rs`
- [ ] Create `crates/cli/src/checks/git/parse_tests.rs`
- [ ] Implement `parse_conventional_commit` function
- [ ] Implement `ParsedCommit` with `is_type_allowed`
- [ ] Implement `ParsedCommit` with `is_scope_allowed`
- [ ] Add 15+ unit tests for parser
- [ ] Update `crates/cli/src/checks/git.rs` to import parse module
- [ ] Run `make check` - all tests pass

## Deliverables

This phase produces:
1. **Commit retrieval**: Functions to get commits from git history
2. **Message parsing**: Regex-based conventional commit parser
3. **Validation helpers**: Type and scope validation methods
4. **Test coverage**: Comprehensive unit tests for parsing logic

The parsing layer is ready for Phase 806+ to implement full commit validation against configuration.
