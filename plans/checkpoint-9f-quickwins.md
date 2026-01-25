# Checkpoint 9F: Quick Wins - Git Check

**Follows:** checkpoint-9e-perf (git2 migration)

## Overview

Complete the git2 migration started in 9e by replacing the remaining subprocess-based diff operations, and add metrics reporting to the git check. These are low-risk, high-value improvements that build on the git2 foundation.

**Goals:**
- Eliminate remaining `git` subprocess calls in `git.rs`
- Add metrics to git check for CI reporting
- Improve test coverage for edge cases

## Project Structure

```
quench/
├── crates/cli/
│   └── src/
│       ├── git.rs                    # MODIFY: migrate diff functions to git2
│       ├── git_tests.rs              # MODIFY: add diff operation tests
│       └── checks/git/
│           ├── mod.rs                # MODIFY: add metrics
│           └── mod_tests.rs          # MODIFY: add edge case tests
└── plans/
    └── checkpoint-9f-quickwins.md    # THIS FILE
```

## Dependencies

**Existing:**
- `git2 = "0.19"` - Already added in 9e

No new dependencies required.

## Implementation Phases

### Phase 1: Migrate `get_staged_files()` to git2

Replace the subprocess-based staged file detection with git2's index API.

**File:** `crates/cli/src/git.rs`

**Current approach:**
```rust
// Spawns: git diff --name-only --cached
let output = Command::new("git")
    .args(["diff", "--name-only", "--cached"])
    .current_dir(root)
    .output()?;
```

**New approach:**
```rust
/// Get list of staged files (for --staged flag).
pub fn get_staged_files(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;
    let head = repo.head()?.peel_to_tree()?;
    let index = repo.index()?;

    let diff = repo.diff_tree_to_index(Some(&head), Some(&index), None)?;

    let mut files = Vec::new();
    for delta in diff.deltas() {
        if let Some(path) = delta.new_file().path() {
            files.push(root.join(path));
        }
    }

    Ok(files)
}
```

**Verification:**
- `cargo test -p quench -- get_staged` passes
- `quench check --staged` works correctly

### Phase 2: Migrate `get_changed_files()` to git2

Replace the two subprocess calls with a single git2 diff operation.

**Current approach:**
```rust
// Spawns: git diff --name-only base
// Spawns: git diff --name-only --cached base
```

**New approach:**
```rust
/// Get list of changed files compared to a git base ref.
pub fn get_changed_files(root: &Path, base: &str) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    // Resolve base to a tree
    let base_tree = repo
        .revparse_single(base)
        .with_context(|| format!("Failed to resolve base ref: {}", base))?
        .peel_to_tree()
        .context("Failed to get tree for base ref")?;

    // Get HEAD tree
    let head_tree = repo.head()?.peel_to_tree()?;

    // Get index for staged changes
    let index = repo.index()?;

    let mut files = std::collections::HashSet::new();

    // Compare HEAD to base (committed changes on branch)
    let head_diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    for delta in head_diff.deltas() {
        if let Some(path) = delta.new_file().path() {
            files.insert(root.join(path));
        }
    }

    // Compare index to base (staged changes)
    let index_diff = repo.diff_tree_to_index(Some(&base_tree), Some(&index), None)?;
    for delta in index_diff.deltas() {
        if let Some(path) = delta.new_file().path() {
            files.insert(root.join(path));
        }
    }

    // Compare workdir to index (unstaged changes)
    let workdir_diff = repo.diff_index_to_workdir(Some(&index), None)?;
    for delta in workdir_diff.deltas() {
        if let Some(path) = delta.new_file().path() {
            files.insert(root.join(path));
        }
    }

    Ok(files.into_iter().collect())
}
```

**Verification:**
- `cargo test -p quench -- get_changed` passes
- `quench check --base main` shows correct changed files
- Results match `git diff --name-only main` output

### Phase 3: Add Metrics to Git Check

Report validation statistics in the check result for CI dashboards.

**File:** `crates/cli/src/checks/git/mod.rs`

```rust
impl Check for GitCheck {
    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // ... existing validation logic ...

        // Build metrics
        let metrics = if !commits.is_empty() {
            Some(serde_json::json!({
                "commits_checked": commits.len(),
                "commits_valid": commits.len() - violations.iter()
                    .filter(|v| v.commit.is_some())
                    .count(),
            }))
        } else {
            None
        };

        let mut result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        };

        result.metrics = metrics;
        result
    }
}
```

**JSON output:**
```json
{
  "name": "git",
  "passed": true,
  "metrics": {
    "commits_checked": 5,
    "commits_valid": 5
  }
}
```

**Verification:**
- `quench check --git --ci -o json` includes metrics
- Metrics count is accurate for validation results

### Phase 4: Add Unit Tests for Edge Cases

Expand test coverage for scenarios not currently tested.

**File:** `crates/cli/src/git_tests.rs`

```rust
// =============================================================================
// DIFF OPERATION TESTS
// =============================================================================

#[test]
fn get_staged_files_empty_staging() {
    // Test with no staged files
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    let files = get_staged_files(temp.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn get_staged_files_with_staged_file() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    // Create and stage a file
    std::fs::write(temp.path().join("test.txt"), "content").unwrap();
    git_add(&temp, "test.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.txt"));
}

#[test]
fn get_changed_files_includes_committed() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Create new branch with changes
    git_checkout_b(&temp, "feature");
    std::fs::write(temp.path().join("new.txt"), "content").unwrap();
    git_add(&temp, "new.txt");
    git_commit(&temp, "feat: add new file");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("new.txt"));
}
```

**File:** `crates/cli/src/checks/git/mod_tests.rs`

```rust
// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[test]
fn validates_merge_commit_subject() {
    // Merge commits often have format "Merge branch 'x' into y"
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "Merge branch 'feature' into main".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    // Default: merge commits violate conventional format
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_format");
}

#[test]
fn validates_breaking_change_marker() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat!: breaking change".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty(), "Breaking change marker should be valid");
}

#[test]
fn validates_breaking_change_with_scope() {
    let commit = Commit {
        hash: "abc1234".to_string(),
        message: "feat(api)!: breaking API change".to_string(),
    };
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty(), "Breaking change with scope should be valid");
}
```

**Verification:**
- All new tests pass with `cargo test -p quench`
- Tests cover edge cases mentioned in conventional commit spec

### Phase 5: Remove Subprocess Fallback Code

After confirming git2 implementation is stable, clean up the subprocess fallback functions.

**File:** `crates/cli/src/git.rs`

Remove:
- `get_commits_since_subprocess()` function
- `parse_git_log_output()` function
- Associated `#[allow(dead_code)]` attributes

**Note:** Only remove after verification that git2 implementations work correctly across different repository states (shallow clones, bare repos, worktrees).

**Verification:**
- `cargo build` succeeds without warnings
- No dead code warnings from removed functions

## Key Implementation Details

### git2 Diff API

The git2 diff API provides several comparison modes:

| Function | Compares | Use Case |
|----------|----------|----------|
| `diff_tree_to_tree` | Two trees | Branch comparison |
| `diff_tree_to_index` | Tree to staging | Staged vs base |
| `diff_index_to_workdir` | Staging to working dir | Unstaged changes |
| `diff_tree_to_workdir` | Tree to working dir | All changes vs base |

For `get_changed_files()`, we need committed + staged + unstaged changes, requiring multiple diff operations.

### Error Handling

Convert git2 errors to anyhow with context:

```rust
let repo = Repository::discover(root)
    .context("Failed to open repository")?;

let tree = repo.revparse_single(base)
    .with_context(|| format!("Failed to resolve ref: {}", base))?
    .peel_to_tree()
    .context("Base ref is not a commit")?;
```

### Detached HEAD Handling

When HEAD is detached (no branch), `repo.head()` still works but returns a reference to a commit rather than a branch. The diff operations work the same way.

```rust
let head = repo.head().context("Failed to get HEAD")?;
// Works for both attached and detached HEAD
let head_tree = head.peel_to_tree()?;
```

## Verification Plan

1. **Phase 1:** `cargo test -p quench -- staged` - Staged file detection works
2. **Phase 2:** `cargo test -p quench -- changed` - Changed file detection works
3. **Phase 3:** `quench check --git --ci -o json | jq .` - Metrics present
4. **Phase 4:** `cargo test -p quench -- git` - All edge case tests pass
5. **Phase 5:** `cargo build` - No dead code warnings

**Final verification:**
```bash
make check                           # All tests pass
quench check --staged               # Works on quench repo
quench check --base main            # Works on quench repo
quench check --git --ci -o json     # Outputs metrics
```

## Checklist

- [ ] Migrate `get_staged_files()` to use git2 index API
- [ ] Migrate `get_changed_files()` to use git2 diff API
- [ ] Add metrics (commits_checked, commits_valid) to git check
- [ ] Add unit tests for staged/changed file detection
- [ ] Add edge case tests (merge commits, breaking changes)
- [ ] Remove subprocess fallback functions
- [ ] Bump `CACHE_VERSION` if check behavior changed
- [ ] Run `make check`
