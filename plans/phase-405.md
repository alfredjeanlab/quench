# Phase 405: Shell Adapter - Detection

## Overview

Implement the core Shell language adapter with file detection and classification. This phase enables quench to automatically detect Shell projects and apply appropriate source/test patterns. The adapter will:

- **Auto-detect** Shell projects via `*.sh` files in root, `bin/`, or `scripts/`
- **Classify files** using default source patterns (`**/*.sh`, `**/*.bash`)
- **Identify test code** using default test patterns (`tests/**/*.bats`, `*_test.sh`)
- **Register with AdapterRegistry** for automatic language detection

Reference:
- `docs/specs/langs/shell.md`
- `docs/specs/10-language-adapters.md`
- Behavioral specs in `tests/specs/adapters/shell.rs`

## Project Structure

```
quench/
├── crates/cli/src/
│   └── adapter/
│       ├── mod.rs              # UPDATE: Add Shell to ProjectLanguage, detection
│       ├── shell/
│       │   └── mod.rs          # NEW: ShellAdapter implementation
│       └── shell_tests.rs      # NEW: Unit tests (sibling file convention)
├── tests/
│   └── fixtures/
│       └── shell/
│           └── auto-detect/    # NEW: Detection fixture
│               └── scripts/
│                   └── build.sh
└── plans/
    └── phase-405.md
```

## Dependencies

No new external dependencies. Uses existing:
- `globset` for pattern matching (already in `Cargo.toml`)
- Adapter trait from `crates/cli/src/adapter/mod.rs`
- `build_glob_set` helper from `crates/cli/src/adapter/glob.rs`

## Implementation Phases

### Phase 1: Add ProjectLanguage::Shell and Detection

Update the adapter module to detect Shell projects.

**Update `crates/cli/src/adapter/mod.rs`:**

```rust
// Add to ProjectLanguage enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectLanguage {
    Rust,
    Shell,  // NEW
    Generic,
}

// Update detect_language function
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }

    // Check for Shell project markers: *.sh in root, bin/, or scripts/
    if has_shell_markers(root) {
        return ProjectLanguage::Shell;
    }

    ProjectLanguage::Generic
}

/// Check if project has Shell markers.
/// Detection: *.sh files in root, bin/, or scripts/
fn has_shell_markers(root: &Path) -> bool {
    // Check root directory
    if has_sh_files(root) {
        return true;
    }

    // Check bin/ directory
    let bin_dir = root.join("bin");
    if bin_dir.is_dir() && has_sh_files(&bin_dir) {
        return true;
    }

    // Check scripts/ directory
    let scripts_dir = root.join("scripts");
    if scripts_dir.is_dir() && has_sh_files(&scripts_dir) {
        return true;
    }

    false
}

/// Check if a directory contains *.sh files.
fn has_sh_files(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sh")
            })
        })
        .unwrap_or(false)
}
```

**Milestone:** Detection compiles, Shell projects identified.

**Verification:**
```bash
cargo build --all
cargo test --lib adapter -- detect_language
```

---

### Phase 2: Create ShellAdapter Module

Create the Shell adapter with default patterns.

**Create `crates/cli/src/adapter/shell/mod.rs`:**

```rust
//! Shell language adapter.
//!
//! Provides Shell-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for shell scripts
//!
//! See docs/specs/langs/shell.md for specification.

use std::path::Path;

use globset::GlobSet;

use super::glob::build_glob_set;
use super::{Adapter, FileKind};

/// Shell language adapter.
pub struct ShellAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
}

impl ShellAdapter {
    /// Create a new Shell adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&[
                "**/*.sh".to_string(),
                "**/*.bash".to_string(),
            ]),
            test_patterns: build_glob_set(&[
                "tests/**/*.bats".to_string(),
                "test/**/*.bats".to_string(),
                "*_test.sh".to_string(),
                "**/*_test.sh".to_string(),
            ]),
        }
    }
}

impl Default for ShellAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for ShellAdapter {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sh", "bash", "bats"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Source patterns
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }

        FileKind::Other
    }
}

#[cfg(test)]
#[path = "../shell_tests.rs"]
mod tests;
```

**Update `crates/cli/src/adapter/mod.rs` imports:**

```rust
pub mod generic;
pub mod glob;
pub mod rust;
pub mod shell;  // NEW

pub use shell::ShellAdapter;  // NEW
```

**Milestone:** ShellAdapter compiles with Adapter trait.

**Verification:**
```bash
cargo build --all
```

---

### Phase 3: Register Shell Adapter

Wire up Shell adapter registration in AdapterRegistry.

**Update `crates/cli/src/adapter/mod.rs` (for_project method):**

```rust
impl AdapterRegistry {
    /// Create a registry pre-populated with detected adapters.
    pub fn for_project(root: &Path) -> Self {
        let mut registry = Self::new(Arc::new(GenericAdapter::with_defaults()));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                registry.register(Arc::new(RustAdapter::new()));
            }
            ProjectLanguage::Shell => {
                registry.register(Arc::new(ShellAdapter::new()));
            }
            ProjectLanguage::Generic => {}
        }

        registry
    }
}
```

**Milestone:** Shell adapter auto-registered for Shell projects.

**Verification:**
```bash
cargo build --all
cargo test --lib adapter
```

---

### Phase 4: Add Unit Tests

Create unit tests following the sibling `_tests.rs` convention.

**Create `crates/cli/src/adapter/shell_tests.rs`:**

```rust
//! Unit tests for the Shell adapter.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use yare::parameterized;

use super::shell::ShellAdapter;
use super::{Adapter, FileKind};

// =============================================================================
// FILE CLASSIFICATION
// =============================================================================

#[parameterized(
    // Source files
    src_root_sh = { "build.sh", FileKind::Source },
    src_scripts_sh = { "scripts/deploy.sh", FileKind::Source },
    src_nested_sh = { "scripts/ci/build.sh", FileKind::Source },
    src_bash = { "scripts/setup.bash", FileKind::Source },
    src_bin_sh = { "bin/run.sh", FileKind::Source },
    // Test files
    test_bats = { "tests/integration.bats", FileKind::Test },
    test_dir_bats = { "test/unit.bats", FileKind::Test },
    test_nested_bats = { "tests/e2e/smoke.bats", FileKind::Test },
    test_suffix = { "build_test.sh", FileKind::Test },
    test_suffix_nested = { "scripts/build_test.sh", FileKind::Test },
    // Other files
    other_toml = { "quench.toml", FileKind::Other },
    other_md = { "README.md", FileKind::Other },
    other_rs = { "src/lib.rs", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = ShellAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}

#[test]
fn name_returns_shell() {
    let adapter = ShellAdapter::new();
    assert_eq!(adapter.name(), "shell");
}

#[test]
fn extensions_include_sh_bash_bats() {
    let adapter = ShellAdapter::new();
    let exts = adapter.extensions();
    assert!(exts.contains(&"sh"), "should include sh");
    assert!(exts.contains(&"bash"), "should include bash");
    assert!(exts.contains(&"bats"), "should include bats");
}
```

**Milestone:** Unit tests pass.

**Verification:**
```bash
cargo test --lib adapter::shell
```

---

### Phase 5: Create Test Fixtures

Create fixtures required for behavioral specs.

**Create `tests/fixtures/shell/auto-detect/scripts/build.sh`:**

```bash
#!/bin/bash
echo 'building'
```

This minimal fixture enables detection via `scripts/*.sh`.

**Milestone:** Fixture exists and is valid.

**Verification:**
```bash
ls tests/fixtures/shell/auto-detect/scripts/build.sh
file tests/fixtures/shell/auto-detect/scripts/build.sh
```

---

### Phase 6: Enable Behavioral Specs

Update behavioral specs to remove `#[ignore]` for detection-related tests.

**Specs to enable in `tests/specs/adapters/shell.rs`:**

1. `shell_adapter_auto_detected_when_sh_files_in_scripts`
2. `shell_adapter_auto_detected_when_sh_files_in_bin`
3. `shell_adapter_auto_detected_when_sh_files_in_root`
4. `shell_adapter_default_source_pattern_matches_sh_files`
5. `shell_adapter_default_source_pattern_matches_bash_files`
6. `shell_adapter_default_test_pattern_matches_bats_files`
7. `shell_adapter_default_test_pattern_matches_test_sh_files`

**Note:** Escape pattern and shellcheck suppress specs remain ignored until Phase 406/407.

**Milestone:** Detection and pattern specs pass.

**Verification:**
```bash
cargo test --test specs shell_adapter_auto
cargo test --test specs shell_adapter_default
```

---

## Key Implementation Details

### Detection Priority

Detection checks in order:
1. `Cargo.toml` → Rust (existing behavior)
2. `*.sh` in root, `bin/`, or `scripts/` → Shell
3. Otherwise → Generic

This ensures Rust projects with shell scripts (like `scripts/bootstrap.sh`) still detect as Rust.

### Default Patterns

| Type | Patterns |
|------|----------|
| Source | `**/*.sh`, `**/*.bash` |
| Test | `tests/**/*.bats`, `test/**/*.bats`, `*_test.sh`, `**/*_test.sh` |

The `**/*_test.sh` pattern ensures nested test files like `scripts/build_test.sh` are classified as tests.

### Extension Registration

The Shell adapter registers for extensions: `sh`, `bash`, `bats`

This enables `AdapterRegistry::adapter_for(path)` to return `ShellAdapter` for any of these extensions.

### No Inline Test Convention

Unlike Rust's `#[cfg(test)]`, Shell has no inline test code convention. File-level classification is sufficient:
- `*.bats` files → always test
- `*_test.sh` files → always test
- Files in `tests/` or `test/` → always test

---

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build --all

# Run unit tests
cargo test --lib adapter::shell

# Check clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run enabled shell adapter specs
cargo test --test specs shell_adapter

# Full quality gates
make check
```

### Test Matrix

| Category | Spec Count | Phase |
|----------|------------|-------|
| Auto-detection | 3 | This phase |
| Default patterns | 4 | This phase |
| Escape patterns | 5 | Phase 406 |
| Shellcheck suppress | 4 | Phase 407 |

**This phase enables: 7 specs**

---

## Summary

| Phase | Task | Key Files |
|-------|------|-----------|
| 1 | Add Shell to ProjectLanguage, detection logic | `adapter/mod.rs` |
| 2 | Create ShellAdapter module | `adapter/shell/mod.rs` |
| 3 | Register Shell adapter in AdapterRegistry | `adapter/mod.rs` |
| 4 | Add unit tests | `adapter/shell_tests.rs` |
| 5 | Create test fixtures | `tests/fixtures/shell/auto-detect/` |
| 6 | Enable behavioral specs | `tests/specs/adapters/shell.rs` |

## Future Phases

- **Phase 406**: Shell Escape Patterns (`set +e`, `eval`)
- **Phase 407**: Shellcheck Suppress Handling
- **Phase 408**: Shell Policy Integration (lint_changes, .shellcheckrc)
