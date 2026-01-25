# Checkpoint 9C: Refactor - Git Check

**Plan:** `checkpoint-9c-refactor`
**Root Feature:** `quench-git`
**Depends On:** Checkpoint 9B (Git Check Validation)

## Overview

Refactor the git check implementation based on patterns discovered during validation. The git check is complete and passing all tests (69 unit tests, 23 behavioral specs). This checkpoint focuses on:

1. **Performance optimization** - Pre-compile regex pattern in docs.rs
2. **Test helper extraction** - Move git test utilities to shared prelude
3. **Code cleanup** - Clarify dead_code annotations and simplify patterns

## Project Structure

```
crates/cli/src/checks/git/
├── mod.rs              # GitCheck implementation (263 lines)
├── mod_tests.rs        # Unit tests (327 lines)
├── parse.rs            # Conventional commit parsing (121 lines)
├── parse_tests.rs      # Parser unit tests (221 lines)
├── docs.rs             # Agent documentation checking (103 lines) [REFACTOR]
├── docs_tests.rs       # Documentation check unit tests (175 lines)
├── template.rs         # .gitmessage template generation (94 lines)
└── template_tests.rs   # Template unit tests (107 lines)

tests/specs/
├── prelude.rs          # Test helpers [ADD GIT HELPERS]
└── checks/git.rs       # Behavioral specs (680 lines) [REFACTOR HELPERS]

crates/cli/src/git.rs   # Git utilities (167 lines)
```

## Dependencies

No external dependencies needed. Uses existing:
- `regex` crate (already in use)
- `std::sync::LazyLock` (already used in parse.rs)

## Implementation Phases

### Phase 1: Pre-compile Regex in docs.rs

**Goal:** Eliminate repeated regex compilation in `has_type_prefix()`.

**Problem:** The `has_type_prefix` function (docs.rs:73-81) builds and compiles a new regex pattern every time it's called:

```rust
// Current (inefficient)
fn has_type_prefix(content: &str) -> bool {
    let types_pattern = COMMIT_TYPES.join("|");
    let pattern = format!(r"(?i)\b({})[:(\(]", types_pattern);
    Regex::new(&pattern)
        .map(|re| re.is_match(content))
        .unwrap_or(false)
}
```

**Solution:** Pre-compile using `LazyLock`, following the pattern from parse.rs:

```rust
use std::sync::LazyLock;

/// Compiled regex for commit type prefixes.
static TYPE_PREFIX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let types_pattern = COMMIT_TYPES.join("|");
    let pattern = format!(r"(?i)\b({})[:(\(]", types_pattern);
    Regex::new(&pattern).expect("valid regex")
});

fn has_type_prefix(content: &str) -> bool {
    TYPE_PREFIX_REGEX.is_match(content)
}
```

**Verification:**
```bash
cargo test -p quench checks::git::docs
# Expected: All docs_tests.rs tests pass
```

---

### Phase 2: Extract Git Test Helpers to Prelude

**Goal:** Move git test utilities from git.rs specs to shared prelude for reuse.

**Current location:** `tests/specs/checks/git.rs` (lines 21-91) has these helpers:
- `init_git_repo(project)` - Initialize git repo with user config
- `create_main_branch(project)` - Add initial commit
- `create_branch(project, name)` - Create feature branch
- `add_commit(project, message)` - Add commit with unique file
- `rand_id()` - Generate unique IDs

**Target location:** `tests/specs/prelude.rs`

**New module structure:**
```rust
// tests/specs/prelude.rs

// ... existing code ...

// =============================================================================
// GIT TEST HELPERS
// =============================================================================

/// Initialize a git repo with minimal config
pub fn git_init(project: &Project) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(project.path())
        .output()
        .expect("git init should succeed");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(project.path())
        .output()
        .expect("git config email should succeed");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(project.path())
        .output()
        .expect("git config name should succeed");
}

/// Create main branch with initial commit (requires files to exist)
pub fn git_initial_commit(project: &Project) {
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .expect("git add should succeed");

    std::process::Command::new("git")
        .args(["commit", "-m", "feat: initial commit"])
        .current_dir(project.path())
        .output()
        .expect("git commit should succeed");
}

/// Create a feature branch
pub fn git_branch(project: &Project, name: &str) {
    std::process::Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(project.path())
        .output()
        .expect("git checkout -b should succeed");
}

/// Add a commit with the given message
pub fn git_commit(project: &Project, message: &str) {
    // Touch a file to make a change
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should work")
        .as_nanos();
    let dummy_file = project.path().join(format!("dummy_{}.txt", id));
    std::fs::write(&dummy_file, "dummy").expect("write should succeed");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .expect("git add should succeed");

    std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project.path())
        .output()
        .expect("git commit should succeed");
}
```

**Migration in git.rs specs:**
```rust
// tests/specs/checks/git.rs

// REMOVE local helper functions

// UPDATE test usage:
// Old: init_git_repo(&temp);
// New: git_init(&temp);

// Old: create_main_branch(&temp);
// New: git_initial_commit(&temp);

// Old: create_branch(&temp, "feature");
// New: git_branch(&temp, "feature");

// Old: add_commit(&temp, "feat: add feature");
// New: git_commit(&temp, "feat: add feature");
```

**Verification:**
```bash
cargo test --test specs git
# Expected: All 23 behavioral specs pass
```

---

### Phase 3: Clarify Dead Code Annotations

**Goal:** Improve documentation for `#[allow(dead_code)]` annotations.

**File:** `crates/cli/src/checks/git/docs.rs`

**Current:**
```rust
/// Result of searching for commit format documentation.
#[derive(Debug)]
pub enum DocsResult {
    /// Documentation found in the specified file.
    // NOTE(lifetime): Used in tests to verify which file matched
    #[allow(dead_code)]
    Found(String),
    // ...
}
```

**Improved:**
```rust
/// Result of searching for commit format documentation.
#[derive(Debug)]
pub enum DocsResult {
    /// Documentation found in the specified file.
    ///
    /// The contained `String` is the filename where documentation was found.
    /// Note: Field is accessed via pattern matching in tests (docs_tests.rs).
    #[allow(dead_code)] // Variant is matched in tests, not directly constructed
    Found(String),
    // ...
}
```

**Verification:**
```bash
cargo clippy --all-targets --all-features -- -D warnings
# Expected: No dead_code warnings for DocsResult::Found
```

---

### Phase 4: Verify No Regressions

**Goal:** Ensure all refactoring maintains identical behavior.

**Run full git check test suite:**
```bash
# Unit tests
cargo test -p quench checks::git
# Expected: 69 tests pass

# Behavioral specs
cargo test --test specs git
# Expected: 23 tests pass

# No ignored specs
grep -r "#\[ignore" tests/specs/checks/git.rs
# Expected: No output
```

---

### Phase 5: Run Full Verification

**Goal:** Confirm all checks pass after refactoring.

**Actions:**
```bash
make check
```

**Expected results:**
- `cargo fmt --all -- --check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test --all` passes (including 69 git unit tests + 23 git specs)
- `cargo build --all` passes
- `cargo audit` passes
- `cargo deny check` passes

## Key Implementation Details

### Regex Pre-compilation Pattern

Following the established pattern from `parse.rs`:

```rust
// parse.rs (existing pattern to follow)
use std::sync::LazyLock;

#[allow(clippy::expect_used)]
static CONVENTIONAL_COMMIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-z]+)(\(([^)]+)\))?:\s*(.+)$").expect("valid regex"));
```

Apply same pattern to `docs.rs` for `TYPE_PREFIX_REGEX`.

### Test Helper Naming Convention

Prefix git helpers with `git_` to namespace them clearly:
- `git_init()` - Initialize repo
- `git_initial_commit()` - First commit on main
- `git_branch()` - Create branch
- `git_commit()` - Add commit

This matches the existing `fixture()` and `default_project()` helper patterns.

### Backward Compatibility

All changes are internal refactoring:
- No public API changes
- No configuration changes
- No output format changes
- All existing tests continue to pass

## Verification Plan

### Phase 1 Verification
```bash
cargo test -p quench checks::git::docs
# Confirm docs.rs tests pass with pre-compiled regex
```

### Phase 2 Verification
```bash
cargo test --test specs git
# Confirm specs pass with extracted helpers
```

### Phase 3 Verification
```bash
cargo clippy --all-targets --all-features -- -D warnings
# No warnings related to dead_code
```

### Phase 4 Verification
```bash
cargo test -p quench checks::git
cargo test --test specs git
# All tests pass (69 unit + 23 behavioral)
```

### Phase 5 Verification
```bash
make check
# Full CI suite passes
```

## Checklist

- [ ] Phase 1: Pre-compile TYPE_PREFIX_REGEX in docs.rs
- [ ] Phase 2: Extract git helpers to tests/specs/prelude.rs
- [ ] Phase 2: Update git.rs specs to use shared helpers
- [ ] Phase 3: Improve DocsResult::Found documentation
- [ ] Phase 4: All git tests pass (69 unit + 23 behavioral)
- [ ] Phase 5: `make check` passes
- [ ] No `#[ignore]` in git specs
