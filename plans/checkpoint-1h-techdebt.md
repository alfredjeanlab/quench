# Checkpoint 1H: Tech Debt - DRY Test Patterns

## Overview

Reduce test boilerplate and improve maintainability by extracting common test utilities and converting repetitive tests to `yare` parameterized tests. The `yare` crate is already a dev-dependency but currently unused.

## Project Structure

```
crates/cli/
├── src/
│   └── test_utils.rs         # NEW: Shared unit test utilities
└── Cargo.toml                # yare already present

tests/
└── specs/
    └── prelude.rs            # Extended with new helpers
```

## Dependencies

Already present in `crates/cli/Cargo.toml`:
- `yare = "3"` (dev-dependency, currently unused)
- `tempfile` (dev-dependency)
- `serde_json` (dev-dependency)

## Implementation Phases

### Phase 1: Create Unit Test Utilities Module

Create `crates/cli/src/test_utils.rs` for shared unit test helpers.

**Key utilities to add:**

```rust
// crates/cli/src/test_utils.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a temp directory with a minimal quench.toml
pub fn temp_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    dir
}

/// Creates a temp directory with custom config content
pub fn temp_project_with_config(config: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), config).unwrap();
    dir
}

/// Creates a directory tree from a list of (path, content) pairs
pub fn create_tree(root: &Path, files: &[(&str, &str)]) {
    for (path, content) in files {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }
}
```

**Wire up the module:**

```rust
// crates/cli/src/lib.rs (add near other test modules)
#[cfg(test)]
pub mod test_utils;
```

**Milestone:** `cargo test --lib` passes, utilities available for unit tests.

---

### Phase 2: Extend Spec Prelude with High-Level Helpers

Extend `tests/specs/prelude.rs` with helpers for common spec patterns.

**Key additions:**

```rust
// tests/specs/prelude.rs

/// Creates a temp directory with quench.toml (version = 1)
pub fn temp_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    dir
}

/// Run quench check with JSON output, return parsed JSON
pub fn check_json(dir: &std::path::Path) -> serde_json::Value {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Run quench check with args, return parsed JSON
pub fn check_json_with_args(dir: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut cmd_args = vec!["check", "-o", "json"];
    cmd_args.extend(args);
    let output = quench_cmd()
        .args(&cmd_args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Extract check names from JSON output
pub fn check_names(json: &serde_json::Value) -> Vec<&str> {
    json.get("checks")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect()
}
```

**Milestone:** Helpers compile and can be used in specs.

---

### Phase 3: Apply yare to Spec Flag Tests

Convert repetitive single-check enable/disable tests in `tests/specs/checks.rs` to yare parameterized tests.

**Before (8 similar tests):**
```rust
#[test]
fn cloc_flag_enables_only_cloc_check() { ... }
#[test]
fn escapes_flag_enables_only_escapes_check() { ... }
// ... 6 more identical patterns
```

**After (1 parameterized test):**
```rust
use yare::parameterized;

#[parameterized(
    cloc = { "cloc" },
    escapes = { "escapes" },
    agents = { "agents" },
    docs = { "docs" },
    tests = { "tests" },
    git = { "git" },
    build = { "build" },
    license = { "license" },
)]
fn enable_flag_runs_only_that_check(check_name: &str) {
    let dir = temp_project();
    let json = check_json_with_args(dir.path(), &[&format!("--{}", check_name)]);
    let names = check_names(&json);

    assert_eq!(names.len(), 1, "only one check should run");
    assert_eq!(names[0], check_name);
}
```

**Similarly for disable flags:**
```rust
#[parameterized(
    cloc = { "cloc" },
    escapes = { "escapes" },
    docs = { "docs" },
    tests = { "tests" },
)]
fn disable_flag_skips_that_check(check_name: &str) {
    let dir = temp_project();
    let json = check_json_with_args(dir.path(), &[&format!("--no-{}", check_name)]);
    let names = check_names(&json);

    assert!(!names.contains(&check_name), "{} should not be present", check_name);
    assert_eq!(names.len(), 7, "7 checks should run");
}
```

**Milestone:** `cargo test --test specs checks` passes with fewer lines of code.

---

### Phase 4: Simplify walker_tests.rs with Utilities

Refactor `crates/cli/src/walker_tests.rs` to use shared utilities.

**Before:**
```rust
#[test]
fn walks_simple_directory() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("src/test.rs"), "fn test() {}").unwrap();
    // ...
}
```

**After:**
```rust
use crate::test_utils::{temp_project, create_tree};

#[test]
fn walks_simple_directory() {
    let tmp = temp_project();
    create_tree(tmp.path(), &[
        ("src/lib.rs", "fn main() {}"),
        ("src/test.rs", "fn test() {}"),
    ]);
    // ...
}
```

**Milestone:** `cargo test walker` passes, tests are shorter and clearer.

---

### Phase 5: Apply yare to Walker Threshold Tests

Convert repetitive walker tests to parameterized tests where appropriate.

**Candidate tests in `walker_tests.rs`:**
```rust
#[parameterized(
    force_parallel = { true, false },
    force_sequential = { false, true },
)]
fn force_flags_override_heuristic(force_parallel: bool, force_sequential: bool) {
    let tmp = temp_project();
    create_tree(tmp.path(), &[("file.txt", "content")]);

    let walker = FileWalker::new(WalkerConfig {
        force_parallel,
        force_sequential,
        ..Default::default()
    });

    assert_eq!(walker.should_use_parallel(tmp.path()), force_parallel);
}
```

**Milestone:** `cargo test walker` passes with reduced test count but same coverage.

---

### Phase 6: Final Cleanup and Documentation

1. **Remove dead helper code** from `prelude.rs` if now unused
2. **Add doc comments** to new test utilities explaining their purpose
3. **Update any tests** still using old patterns to use new helpers
4. **Run full test suite** to verify no regressions

**Milestone:** `make check` passes, test files are measurably shorter.

---

## Key Implementation Details

### yare Parameterized Test Syntax

```rust
use yare::parameterized;

#[parameterized(
    case_name_1 = { arg1_value, arg2_value },
    case_name_2 = { arg1_value, arg2_value },
)]
fn test_function(arg1: Type1, arg2: Type2) {
    // Test body using arg1, arg2
}
```

### Test Utility Module Visibility

The `test_utils` module uses `#[cfg(test)]` gating:

```rust
// In lib.rs
#[cfg(test)]
pub mod test_utils;

// In any _tests.rs file
use crate::test_utils::*;
```

### Prelude Import Pattern for Specs

```rust
// tests/specs/checks.rs
use crate::prelude::*;
use yare::parameterized;
```

## Verification Plan

1. **Phase verification:** After each phase, run the relevant subset:
   - Phase 1: `cargo test --lib test_utils`
   - Phase 2: `cargo test --test specs`
   - Phase 3: `cargo test --test specs checks`
   - Phase 4: `cargo test walker`
   - Phase 5: `cargo test walker`
   - Phase 6: `make check`

2. **Line count comparison:** Before and after LOC for key files:
   - `tests/specs/checks.rs` (target: -100 lines)
   - `crates/cli/src/walker_tests.rs` (target: -50 lines)

3. **Coverage check:** Run tests with `--nocapture` to verify all cases execute

4. **Final validation:**
   ```bash
   make check  # Full CI validation
   ```
