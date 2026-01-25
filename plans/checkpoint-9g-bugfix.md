# Checkpoint 9G: Bug Fixes - Git Check

**Root Feature:** `quench-971g`
**Follows:** checkpoint-9f-quickwins (git2 migration completion)

## Overview

Fix bugs and edge cases in the git check and git utilities discovered after the git2 migration. The primary issue is that deleted files are not properly detected when using git2's diff operations, as the code only checks `new_file().path()` which is empty for deleted files.

**Goals:**
- Fix deleted file detection in `get_staged_files()` and `get_changed_files()`
- Add comprehensive tests for edge cases (deleted files, renamed files)
- Handle initial commit edge case in `get_changed_files()`
- Improve error messages for common failure scenarios

## Project Structure

```
quench/
├── crates/cli/
│   └── src/
│       ├── git.rs                    # MODIFY: fix deleted file detection
│       ├── git_tests.rs              # MODIFY: add deleted/renamed file tests
│       └── checks/git/
│           ├── mod.rs                # (no changes expected)
│           └── mod_tests.rs          # (no changes expected)
├── tests/
│   └── specs/checks/
│       └── git.rs                    # MODIFY: add behavioral tests for edge cases
└── plans/
    └── checkpoint-9g-bugfix.md       # THIS FILE
```

## Dependencies

**Existing:**
- `git2 = "0.19"` - Already added in 9e

No new dependencies required.

## Implementation Phases

### Phase 1: Fix Deleted File Detection in `get_staged_files()`

The current implementation only uses `delta.new_file().path()`, which returns `None` for deleted files. For deleted files, we need to use `delta.old_file().path()`.

**File:** `crates/cli/src/git.rs`

**Current (buggy):**
```rust
for delta in diff.deltas() {
    if let Some(path) = delta.new_file().path() {
        files.push(root.join(path));
    }
}
```

**Fixed:**
```rust
for delta in diff.deltas() {
    // For deleted files, new_file().path() is None; use old_file() instead
    let path = delta.new_file().path().or_else(|| delta.old_file().path());
    if let Some(path) = path {
        files.push(root.join(path));
    }
}
```

**Add test:** `get_staged_files_includes_deleted`
```rust
#[test]
fn get_staged_files_includes_deleted() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    // Create and commit a file
    std::fs::write(temp.path().join("to_delete.txt"), "content").unwrap();
    git_add(&temp, "to_delete.txt");
    git_commit(&temp, "feat: add file");

    // Delete and stage the deletion
    std::fs::remove_file(temp.path().join("to_delete.txt")).unwrap();
    git_add(&temp, "to_delete.txt");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("to_delete.txt"));
}
```

**Verification:**
- `cargo test -p quench -- get_staged_files_includes_deleted` passes

---

### Phase 2: Fix Deleted File Detection in `get_changed_files()`

The same issue exists in three places within `get_changed_files()`:
1. Committed changes (HEAD vs base)
2. Staged changes (index vs base)
3. Unstaged changes (workdir vs index)

**File:** `crates/cli/src/git.rs`

**Fixed approach:** Create a helper function to extract path from delta:

```rust
/// Extract file path from a diff delta.
/// For deleted files, new_file().path() is None, so fall back to old_file().
fn extract_path(delta: &git2::DiffDelta) -> Option<&Path> {
    delta.new_file().path().or_else(|| delta.old_file().path())
}
```

Then use it consistently:
```rust
for delta in head_diff.deltas() {
    if let Some(path) = extract_path(&delta) {
        files.insert(root.join(path));
    }
}

for delta in index_diff.deltas() {
    if let Some(path) = extract_path(&delta) {
        files.insert(root.join(path));
    }
}

for delta in workdir_diff.deltas() {
    if let Some(path) = extract_path(&delta) {
        files.insert(root.join(path));
    }
}
```

**Add tests:**

```rust
#[test]
fn get_changed_files_includes_deleted_committed() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Add a file on main
    std::fs::write(temp.path().join("to_delete.txt"), "content").unwrap();
    git_add(&temp, "to_delete.txt");
    git_commit(&temp, "feat: add file");

    // Create branch and delete the file
    git_checkout_b(&temp, "feature");
    std::fs::remove_file(temp.path().join("to_delete.txt")).unwrap();
    git_add(&temp, "to_delete.txt");
    git_commit(&temp, "chore: delete file");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("to_delete.txt"));
}

#[test]
fn get_changed_files_includes_deleted_staged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    // Add and commit a file
    std::fs::write(temp.path().join("to_delete.txt"), "content").unwrap();
    git_add(&temp, "to_delete.txt");
    git_commit(&temp, "feat: add file");

    // Create branch and stage deletion
    git_checkout_b(&temp, "feature");
    std::fs::remove_file(temp.path().join("to_delete.txt")).unwrap();
    git_add(&temp, "to_delete.txt");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert!(files.iter().any(|f| f.ends_with("to_delete.txt")));
}

#[test]
fn get_changed_files_includes_deleted_unstaged() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    git_checkout_b(&temp, "feature");

    // Delete README.md (tracked file) without staging
    std::fs::remove_file(temp.path().join("README.md")).unwrap();

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert!(files.iter().any(|f| f.ends_with("README.md")));
}
```

**Verification:**
- `cargo test -p quench -- get_changed` passes
- All deleted file tests pass

---

### Phase 3: Handle Renamed Files

Renamed files are another edge case. In git2, renamed files have:
- `old_file()` with the original path
- `new_file()` with the new path
- `delta.status()` == `Delta::Renamed`

For file walking purposes (determining which files to check), we want the **new** path. The current code happens to work correctly for renames since `new_file().path()` returns the new path.

However, we should add tests to verify this behavior is preserved:

```rust
#[test]
fn get_staged_files_includes_renamed_new_path() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    // Create and commit a file
    std::fs::write(temp.path().join("old_name.txt"), "content").unwrap();
    git_add(&temp, "old_name.txt");
    git_commit(&temp, "feat: add file");

    // Rename the file using git mv
    Command::new("git")
        .args(["mv", "old_name.txt", "new_name.txt"])
        .current_dir(temp.path())
        .output()
        .expect("git mv should succeed");

    let files = get_staged_files(temp.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("new_name.txt"), "should use new name");
    assert!(!files[0].ends_with("old_name.txt"), "should not use old name");
}

#[test]
fn get_changed_files_includes_renamed_new_path() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);
    create_initial_commit(&temp);

    git_checkout_b(&temp, "feature");

    // Add and commit a file
    std::fs::write(temp.path().join("old_name.txt"), "content").unwrap();
    git_add(&temp, "old_name.txt");
    git_commit(&temp, "feat: add file");

    // Rename using git mv and commit
    Command::new("git")
        .args(["mv", "old_name.txt", "new_name.txt"])
        .current_dir(temp.path())
        .output()
        .expect("git mv should succeed");
    git_commit(&temp, "refactor: rename file");

    let files = get_changed_files(temp.path(), "main").unwrap();
    assert!(files.iter().any(|f| f.ends_with("new_name.txt")));
}
```

**Note:** The helper function design from Phase 2 needs adjustment to prefer `new_file()` for renames:

```rust
/// Extract file path from a diff delta.
/// - For deleted files: use old_file (new_file is None)
/// - For added/modified/renamed: use new_file (the current path)
fn extract_path(delta: &git2::DiffDelta) -> Option<&Path> {
    delta.new_file().path().or_else(|| delta.old_file().path())
}
```

This order (new then old) correctly handles:
- Added: new_file has path, old_file is None → returns new
- Modified: both have same path → returns new (either works)
- Renamed: both have paths, new is current → returns new
- Deleted: new_file is None → returns old

**Verification:**
- `cargo test -p quench -- renamed` passes

---

### Phase 4: Handle Initial Commit Edge Case

When `get_changed_files()` is called with a base ref that is the initial commit itself, or when HEAD is the initial commit, the diff operations need special handling.

**Issue:** `repo.head()?.peel_to_tree()?` may fail on an empty repository.

**Current code already handles this for `get_staged_files()`:**
```rust
let head_tree = match repo.head() {
    Ok(head) => Some(head.peel_to_tree()?),
    Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
    Err(e) => return Err(e).context("Failed to get HEAD"),
};
```

**Apply same pattern to `get_changed_files()`:**

```rust
pub fn get_changed_files(root: &Path, base: &str) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::discover(root).context("Failed to open repository")?;

    // Resolve base to a tree
    let base_tree = repo
        .revparse_single(base)
        .with_context(|| format!("Failed to resolve base ref: {}", base))?
        .peel_to_tree()
        .context("Failed to get tree for base ref")?;

    // Get HEAD tree (handle case of empty repo with no commits)
    let head_tree = match repo.head() {
        Ok(head) => head.peel_to_tree().context("Failed to get HEAD tree")?,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            // No commits yet - only unstaged changes are possible
            // Return empty set (nothing to compare against)
            return Ok(Vec::new());
        }
        Err(e) => return Err(e).context("Failed to get HEAD"),
    };

    // ... rest of function
}
```

**Add test:**
```rust
#[test]
fn get_changed_files_empty_repo() {
    let temp = TempDir::new().unwrap();
    init_git_repo(&temp);

    // Try to get changed files against nonexistent ref
    let result = get_changed_files(temp.path(), "main");
    assert!(result.is_err(), "should error when base ref doesn't exist");
}
```

**Verification:**
- `cargo test -p quench -- initial_commit` passes
- `cargo test -p quench -- empty_repo` passes

---

### Phase 5: Add Behavioral Specs

Add behavioral tests to verify the git check works correctly with deleted files.

**File:** `tests/specs/checks/git.rs`

```rust
/// Spec: docs/specs/checks/git.md#scope
///
/// > `--base <ref>`: Validates all commits on branch since base
#[test]
fn git_check_validates_branch_with_deleted_files() {
    let temp = Project::empty();
    temp.config(
        r#"[git.commit]
check = "error"
agents = false
"#,
    );
    temp.file("CLAUDE.md", "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n");
    temp.file("to_delete.txt", "content");

    git_init(&temp);
    git_initial_commit(&temp);

    // Create branch, delete file, commit
    git_branch(&temp, "feature");
    std::fs::remove_file(temp.path().join("to_delete.txt")).unwrap();
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "chore: delete unused file"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should pass - commit has valid conventional format
    check("git").pwd(temp.path()).args(&["--ci"]).passes();
}
```

**Verification:**
- `cargo test --test specs -- git_check_validates_branch_with_deleted_files` passes

---

### Phase 6: Clean Up and Documentation

1. Update `git.rs` module documentation to clarify deleted file handling
2. Ensure consistent error messages
3. Run full test suite

**File:** `crates/cli/src/git.rs`

Update module doc:
```rust
//! Git utilities for change detection.
//!
//! Uses git2 (libgit2) for all git operations to avoid subprocess overhead.
//!
//! ## File Detection
//!
//! When detecting changed files:
//! - Added files: path from `new_file()`
//! - Modified files: path from `new_file()` (same as old)
//! - Renamed files: path from `new_file()` (the new location)
//! - Deleted files: path from `old_file()` (since `new_file()` is empty)
```

**Verification:**
- `make check` passes (all lint, test, build steps)
- `cargo doc --no-deps -p quench` builds cleanly

---

## Key Implementation Details

### git2 Delta File Path Resolution

The git2 library represents diffs with `DiffDelta` objects containing:
- `old_file()` - The file state before the change
- `new_file()` - The file state after the change
- `status()` - The type of change (`Added`, `Deleted`, `Modified`, `Renamed`, etc.)

| Delta Status | old_file().path() | new_file().path() | What to Use |
|--------------|-------------------|-------------------|-------------|
| Added | None | Some(path) | new_file |
| Deleted | Some(path) | None | old_file |
| Modified | Some(path) | Some(path) | Either (same) |
| Renamed | Some(old_path) | Some(new_path) | new_file |
| Copied | Some(old_path) | Some(new_path) | new_file |

### Helper Function Pattern

```rust
fn extract_path(delta: &git2::DiffDelta) -> Option<&Path> {
    // Try new_file first (works for add, modify, rename, copy)
    // Fall back to old_file (needed for delete)
    delta.new_file().path().or_else(|| delta.old_file().path())
}
```

This single helper replaces 4 instances of the same pattern and ensures consistent behavior.

### Test Fixture Helpers

New test helpers needed in `git_tests.rs`:

```rust
/// Delete and stage a file.
fn git_rm(temp: &TempDir, file: &str) {
    std::fs::remove_file(temp.path().join(file)).unwrap();
    Command::new("git")
        .args(["add", file])
        .current_dir(temp.path())
        .output()
        .expect("Failed to stage deletion");
}

/// Rename a file using git mv.
fn git_mv(temp: &TempDir, old: &str, new: &str) {
    Command::new("git")
        .args(["mv", old, new])
        .current_dir(temp.path())
        .output()
        .expect("Failed to rename file");
}
```

## Verification Plan

1. **Phase 1:** `cargo test -p quench -- get_staged_files_includes_deleted`
2. **Phase 2:** `cargo test -p quench -- get_changed_files_includes_deleted`
3. **Phase 3:** `cargo test -p quench -- renamed`
4. **Phase 4:** `cargo test -p quench -- empty_repo`
5. **Phase 5:** `cargo test --test specs -- deleted_files`
6. **Phase 6:** `make check`

**Final verification:**
```bash
make check                           # All tests pass
quench check --staged               # Works with staged deletions
quench check --base main            # Works with committed deletions
```

## Checklist

- [ ] Fix `get_staged_files()` to handle deleted files
- [ ] Fix `get_changed_files()` to handle deleted files in all three diffs
- [ ] Add `extract_path()` helper function for consistent path extraction
- [ ] Add tests for deleted files (staged, committed, unstaged)
- [ ] Add tests for renamed files
- [ ] Handle initial commit edge case in `get_changed_files()`
- [ ] Add behavioral spec for branch with deleted files
- [ ] Update module documentation
- [ ] Bump `CACHE_VERSION` if check behavior changed
- [ ] Run `make check`
