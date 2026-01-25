# Checkpoint 9E: Performance - Git Check

**Root Feature:** `quench-2744`

## Overview

Optimize the git check to reduce subprocess overhead, which benchmarks (checkpoint-9d) identified as ~90% of end-to-end time. The primary strategy replaces `git` subprocess calls with the `git2` crate (libgit2 bindings) for direct repository access, eliminating process spawn overhead for repository detection, branch detection, and commit retrieval.

**Performance targets:**
- Fast check (warm): < 100ms
- CI check (500 commits): < 500ms
- Subprocess elimination: 6+ subprocess calls → 0

## Project Structure

```
quench/
├── crates/cli/
│   ├── Cargo.toml                  # MODIFY: add git2 dependency
│   ├── src/
│   │   └── git.rs                  # MODIFY: replace subprocess with git2
│   └── benches/
│       └── git.rs                  # EXISTING: verify improvements
└── plans/
    └── checkpoint-9e-perf.md       # THIS FILE
```

## Dependencies

**New:**
- `git2 = "0.19"` - Rust bindings to libgit2 for direct repository access

**Rationale:**
- `git2` is the standard library for git operations in Rust (used by Cargo)
- Eliminates subprocess spawn overhead (~5-10ms per call)
- Provides typed access to repository objects
- Well-maintained, stable API

## Implementation Phases

### Phase 1: Add git2 Dependency and Replace is_git_repo

Replace the subprocess-based `is_git_repo()` with direct `.git` directory detection.

**File:** `crates/cli/Cargo.toml`
```toml
[dependencies]
git2 = "0.19"
```

**File:** `crates/cli/src/git.rs`

```rust
use git2::Repository;

/// Check if a path is in a git repository.
pub fn is_git_repo(root: &Path) -> bool {
    Repository::discover(root).is_ok()
}
```

**Verification:**
- `cargo test -p quench -- git` passes
- `quench check --git` on a git repo still works

### Phase 2: Replace detect_base_branch with git2

Replace the two-subprocess branch detection with direct ref lookup.

**Current approach:**
```rust
// Spawns: git rev-parse --verify main
// Spawns: git rev-parse --verify master (if main fails)
```

**New approach:**
```rust
/// Detect base branch for CI mode (main or master).
pub fn detect_base_branch(root: &Path) -> Option<String> {
    let repo = Repository::discover(root).ok()?;

    // Check if main branch exists
    if repo.find_branch("main", git2::BranchType::Local).is_ok() {
        return Some("main".to_string());
    }

    // Fall back to master
    if repo.find_branch("master", git2::BranchType::Local).is_ok() {
        return Some("master".to_string());
    }

    // Check for remote branches if local don't exist
    for name in ["origin/main", "origin/master"] {
        if repo.revparse_single(name).is_ok() {
            return Some(name.to_string());
        }
    }

    None
}
```

**Verification:**
- `cargo test -p quench -- detect_base` passes
- Branch detection works on repos with `main`, `master`, or remote-only branches

### Phase 3: Replace get_commits_since with git2

Replace `git log` subprocess with direct commit walking.

**Current approach:**
```rust
// Spawns: git log --format=%h%n%s base..HEAD
```

**New approach:**
```rust
/// Get commits since a base ref.
///
/// Returns commits from newest to oldest.
pub fn get_commits_since(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    let repo = Repository::discover(root)?;

    // Resolve base and HEAD
    let base_oid = repo.revparse_single(base)?.id();
    let head_oid = repo.head()?.target()
        .ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;

    // Walk commits from HEAD, stopping at base
    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_oid)?;
    revwalk.hide(base_oid)?;

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        // Short hash (7 chars)
        let hash = oid.to_string()[..7].to_string();

        // Subject line only (first line of message)
        let message = commit
            .summary()
            .unwrap_or("")
            .to_string();

        commits.push(Commit { hash, message });
    }

    Ok(commits)
}
```

**Verification:**
- `cargo test -p quench -- get_commits` passes
- Commits match those from `git log` on test fixtures

### Phase 4: Replace get_all_branch_commits with git2

Replace full branch commit retrieval using the updated functions.

```rust
/// Get all commits on current branch (for CI mode).
pub fn get_all_branch_commits(root: &Path) -> anyhow::Result<Vec<Commit>> {
    if let Some(base) = detect_base_branch(root) {
        get_commits_since(root, &base)
    } else {
        // No base branch found, get all commits
        let repo = Repository::discover(root)?;
        let head_oid = repo.head()?.target()
            .ok_or_else(|| anyhow::anyhow!("HEAD has no target"))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            let hash = oid.to_string()[..7].to_string();
            let message = commit.summary().unwrap_or("").to_string();
            commits.push(Commit { hash, message });
        }

        Ok(commits)
    }
}
```

**Verification:**
- `cargo test -p quench -- get_all_branch` passes
- CI mode still collects all branch commits correctly

### Phase 5: Replace Template Configuration Check

Replace `git config commit.template` subprocess calls with direct config access.

**File:** `crates/cli/src/checks/git/mod.rs`

```rust
use git2::Repository;

/// Check if commit.template is already configured.
fn is_template_configured(root: &Path) -> bool {
    Repository::discover(root)
        .and_then(|repo| repo.config())
        .and_then(|config| config.get_string("commit.template"))
        .is_ok()
}

/// Configure git commit.template to use .gitmessage.
fn configure_git_template(root: &Path) -> bool {
    let Ok(repo) = Repository::discover(root) else {
        return false;
    };
    let Ok(mut config) = repo.config() else {
        return false;
    };

    config.set_str("commit.template", TEMPLATE_PATH).is_ok()
}
```

**Verification:**
- `quench check --git --fix` still creates and configures template
- Tests pass

### Phase 6: Run Benchmarks and Validate Performance

Run the existing benchmarks from checkpoint-9d to measure improvements.

```bash
# Run all git benchmarks
cargo bench --bench git

# Compare against baseline from 9d
# Expected: 5-10x improvement on subprocess-heavy operations
```

**Expected results:**
| Operation | Before (subprocess) | After (git2) | Improvement |
|-----------|---------------------|--------------|-------------|
| is_git_repo | ~5ms | <0.1ms | 50x+ |
| detect_base_branch | ~10ms (2 calls) | <0.5ms | 20x+ |
| get_commits_since (10) | ~15ms | <1ms | 15x+ |
| get_commits_since (500) | ~50ms | <5ms | 10x+ |
| E2E small (10 commits) | ~30ms | <5ms | 6x+ |
| E2E large (500 commits) | ~80ms | <20ms | 4x+ |

**Verification:**
- All benchmarks run without error
- E2E times meet performance targets
- `make check` passes

## Key Implementation Details

### Error Handling

Convert `git2::Error` to `anyhow::Error` for consistency with existing API:

```rust
use anyhow::Context;

pub fn get_commits_since(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    let repo = Repository::discover(root)
        .context("Failed to open repository")?;
    // ...
}
```

### Repository Caching

For multiple operations in a single check run, consider caching the `Repository` object:

```rust
// In CheckContext or similar
struct GitContext {
    repo: Option<Repository>,
}

impl GitContext {
    fn get_or_open(&mut self, root: &Path) -> Option<&Repository> {
        if self.repo.is_none() {
            self.repo = Repository::discover(root).ok();
        }
        self.repo.as_ref()
    }
}
```

This is an optional enhancement if benchmarks show repeated `Repository::discover()` calls are significant.

### Subprocess Fallback

Keep subprocess functions available as fallback (renamed with `_subprocess` suffix) for:
- Debugging/comparison
- Environments where libgit2 has issues

```rust
/// Get commits since base (subprocess fallback).
#[allow(dead_code)]
fn get_commits_since_subprocess(root: &Path, base: &str) -> anyhow::Result<Vec<Commit>> {
    // Original implementation
}
```

### get_changed_files and get_staged_files

These functions use `git diff` and are called for file-scope checks, not the git check itself. They can be migrated in a follow-up if needed, but are lower priority since:
1. They're not called during `--git` check
2. The diff API in git2 is more complex
3. Subprocess overhead is acceptable for one-off calls

## Verification Plan

1. **Phase 1:** `cargo test -p quench -- is_git_repo` - Repository detection works
2. **Phase 2:** `cargo test -p quench -- detect_base` - Branch detection works
3. **Phase 3:** `cargo test -p quench -- get_commits_since` - Commit retrieval works
4. **Phase 4:** `cargo test -p quench -- get_all_branch` - CI mode works
5. **Phase 5:** `cargo test -p quench -- template` - Template configuration works
6. **Phase 6:** `cargo bench --bench git` - Performance targets met

**Final verification:**
```bash
make check                           # All tests pass
cargo bench --bench git              # Performance improved
quench check --git --ci              # Works on quench repo itself
```

## Checklist

- [ ] Add `git2 = "0.19"` to `Cargo.toml`
- [ ] Replace `is_git_repo()` with `Repository::discover()`
- [ ] Replace `detect_base_branch()` with `repo.find_branch()`
- [ ] Replace `get_commits_since()` with `repo.revwalk()`
- [ ] Replace `get_all_branch_commits()` with git2
- [ ] Replace `is_template_configured()` with `repo.config()`
- [ ] Replace `configure_git_template()` with `config.set_str()`
- [ ] Run benchmarks and verify performance improvement
- [ ] Bump `CACHE_VERSION` if check behavior changed
- [ ] Run `make check`
