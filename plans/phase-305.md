# Phase 305: Rust Adapter - Detection

**Root Feature:** `quench-a0ea`

## Overview

Implement the Rust language adapter detection system. This enables quench to automatically detect Rust projects and apply language-specific defaults without explicit configuration. The adapter provides:

- **Project detection** via `Cargo.toml` presence
- **Default source patterns** for `.rs` files
- **Default test patterns** for test directories and file suffixes
- **Default ignore patterns** for build artifacts
- **Workspace awareness** for multi-package projects

This phase implements the foundation; subsequent phases add escape patterns (306) and `#[cfg(test)]` inline detection (307).

Reference docs:
- `docs/specs/langs/rust.md`
- `docs/specs/10-language-adapters.md`

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── mod.rs          # Update: detection trait, registry
│   │   ├── rust.rs         # NEW: Rust adapter implementation
│   │   └── rust_tests.rs   # NEW: Rust adapter unit tests
│   ├── config.rs           # Update: [rust] config section
│   └── discovery.rs        # Update: Cargo.toml detection
├── tests/
│   ├── specs/
│   │   └── adapters/rust.rs  # Remove #[ignore] for detection specs
│   └── fixtures/
│       ├── rust/auto-detect/   # EXISTING
│       └── rust-workspace/     # EXISTING
└── plans/
    └── phase-305.md
```

## Dependencies

No new external dependencies. Uses existing:
- `toml` for Cargo.toml parsing
- `globset` for pattern matching (already used by generic adapter)

## Implementation Phases

### Phase 1: Rust Adapter Skeleton

Create the basic Rust adapter module that registers for `.rs` extension.

**Create `crates/cli/src/adapter/rust.rs`:**

```rust
//! Rust language adapter.
//!
//! Provides Rust-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Rust projects
//! - (Future) Inline test detection via #[cfg(test)]
//!
//! See docs/specs/langs/rust.md for specification.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::{Adapter, FileKind};

/// Rust language adapter.
pub struct RustAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RustAdapter {
    /// Create a new Rust adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.rs".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**".to_string(),
                "test/**/*.rs".to_string(),
                "*_test.rs".to_string(),
                "*_tests.rs".to_string(),
            ]),
            ignore_patterns: build_glob_set(&["target/**".to_string()]),
        }
    }

    /// Check if a path should be ignored (e.g., target/).
    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Default for RustAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for RustAdapter {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
        if self.should_ignore(path) {
            return FileKind::Other;
        }

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

fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod tests;
```

**Update `crates/cli/src/adapter/mod.rs`:**

```rust
pub mod generic;
pub mod rust;

pub use generic::GenericAdapter;
pub use rust::RustAdapter;
```

**Milestone:** Rust adapter compiles and classifies `.rs` files.

**Verification:**
```bash
cargo build
cargo test adapter::rust
```

---

### Phase 2: Cargo.toml Detection

Add project detection via `Cargo.toml` presence to inform adapter selection.

**Add to `crates/cli/src/adapter/mod.rs`:**

```rust
/// Detect project language from marker files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectLanguage {
    Rust,
    Generic,
}

/// Detect project language by checking for marker files.
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }
    ProjectLanguage::Generic
}
```

**Update `AdapterRegistry` to select by project language:**

```rust
impl AdapterRegistry {
    /// Create a registry pre-populated with detected adapters.
    pub fn for_project(root: &Path) -> Self {
        let mut registry = Self::new(Arc::new(GenericAdapter::with_defaults()));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                registry.register(Arc::new(RustAdapter::new()));
            }
            ProjectLanguage::Generic => {}
        }

        registry
    }
}
```

**Milestone:** Projects with `Cargo.toml` get Rust adapter by default.

**Verification:**
```bash
cargo test adapter -- detect_language
```

---

### Phase 3: Default Patterns Implementation

Ensure default patterns match the spec:

| Pattern Type | Patterns |
|-------------|----------|
| source | `**/*.rs` |
| tests | `tests/**`, `test/**/*.rs`, `*_test.rs`, `*_tests.rs` |
| ignore | `target/` |

**Create `crates/cli/src/adapter/rust_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::Path;

mod classification {
    use super::*;

    #[test]
    fn source_file_in_src() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("src/lib.rs")), FileKind::Source);
        assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
        assert_eq!(adapter.classify(Path::new("src/foo/bar.rs")), FileKind::Source);
    }

    #[test]
    fn test_file_in_tests_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("tests/integration.rs")), FileKind::Test);
        assert_eq!(adapter.classify(Path::new("tests/foo/bar.rs")), FileKind::Test);
    }

    #[test]
    fn test_file_with_suffix() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("src/lib_test.rs")), FileKind::Test);
        assert_eq!(adapter.classify(Path::new("src/lib_tests.rs")), FileKind::Test);
    }

    #[test]
    fn ignored_target_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("target/debug/deps/foo.rs")), FileKind::Other);
        assert_eq!(adapter.classify(Path::new("target/release/build/bar.rs")), FileKind::Other);
    }

    #[test]
    fn non_rust_file() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("Cargo.toml")), FileKind::Other);
        assert_eq!(adapter.classify(Path::new("README.md")), FileKind::Other);
    }
}

mod ignore_patterns {
    use super::*;

    #[test]
    fn target_dir_ignored() {
        let adapter = RustAdapter::new();
        assert!(adapter.should_ignore(Path::new("target/debug/foo.rs")));
        assert!(adapter.should_ignore(Path::new("target/release/bar.rs")));
    }

    #[test]
    fn src_not_ignored() {
        let adapter = RustAdapter::new();
        assert!(!adapter.should_ignore(Path::new("src/lib.rs")));
        assert!(!adapter.should_ignore(Path::new("tests/test.rs")));
    }
}
```

**Milestone:** All default pattern unit tests pass.

**Verification:**
```bash
cargo test adapter::rust -- --nocapture
```

---

### Phase 4: Workspace Package Enumeration

Parse `Cargo.toml` to detect workspace members for package-level metrics.

**Add workspace parsing to `crates/cli/src/adapter/rust.rs`:**

```rust
use std::fs;
use toml::Value;

/// Cargo workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct CargoWorkspace {
    /// Is this a workspace root?
    pub is_workspace: bool,
    /// Package names in the workspace.
    pub packages: Vec<String>,
    /// Member glob patterns (e.g., "crates/*").
    pub member_patterns: Vec<String>,
}

impl CargoWorkspace {
    /// Parse workspace info from Cargo.toml at the given root.
    pub fn from_root(root: &Path) -> Self {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(&cargo_toml) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let value: Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        Self::from_toml(&value, root)
    }

    fn from_toml(value: &Value, root: &Path) -> Self {
        let workspace = value.get("workspace");

        if workspace.is_none() {
            // Single package, not a workspace
            if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
                return Self {
                    is_workspace: false,
                    packages: vec![pkg.as_str().unwrap_or("").to_string()],
                    member_patterns: vec![],
                };
            }
            return Self::default();
        }

        let workspace = workspace.unwrap();
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Expand member patterns to find actual packages
        let packages = expand_workspace_members(&members, root);

        Self {
            is_workspace: true,
            packages,
            member_patterns: members,
        }
    }
}

/// Expand workspace member patterns to package names.
fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        // Handle glob patterns like "crates/*"
        if pattern.contains('*') {
            if let Some(base) = pattern.strip_suffix("/*") {
                let dir = root.join(base);
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            if let Some(name) = read_package_name(&entry.path()) {
                                packages.push(name);
                            }
                        }
                    }
                }
            }
        } else {
            // Direct path to package
            let pkg_dir = root.join(pattern);
            if let Some(name) = read_package_name(&pkg_dir) {
                packages.push(name);
            }
        }
    }

    packages.sort();
    packages
}

/// Read package name from a directory's Cargo.toml.
fn read_package_name(dir: &Path) -> Option<String> {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).ok()?;
    let value: Value = toml::from_str(&content).ok()?;
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(String::from)
}
```

**Milestone:** Workspace packages are enumerated from Cargo.toml.

**Verification:**
```bash
cargo test adapter::rust -- workspace
```

---

### Phase 5: Integration with Walker/Config

Wire the Rust adapter into the file walker to apply ignore patterns automatically.

**Update `crates/cli/src/walker.rs`** (if needed) to use adapter ignore patterns:

```rust
use crate::adapter::{detect_language, ProjectLanguage, RustAdapter};

impl WalkerBuilder {
    /// Build walker with language-specific ignore patterns.
    pub fn with_detected_ignores(mut self, root: &Path) -> Self {
        match detect_language(root) {
            ProjectLanguage::Rust => {
                // Add target/ to ignore patterns
                self.ignore_patterns.push("target/**".to_string());
            }
            ProjectLanguage::Generic => {}
        }
        self
    }
}
```

**Update config loading to detect Rust and apply defaults:**

```rust
// In config.rs or runner.rs
impl Config {
    pub fn with_detected_defaults(mut self, root: &Path) -> Self {
        if detect_language(root) == ProjectLanguage::Rust {
            // Apply Rust defaults if not explicitly configured
            if self.project.ignore.is_empty() {
                self.project.ignore.push("target/".to_string());
            }
        }
        self
    }
}
```

**Milestone:** Rust projects automatically ignore `target/` during file walking.

**Verification:**
```bash
cargo test --test specs rust_adapter_default_ignores
```

---

### Phase 6: Enable Detection Specs

Remove `#[ignore]` from detection-related specs in `tests/specs/adapters/rust.rs`.

**Specs to enable:**
- `rust_adapter_auto_detected_when_cargo_toml_present`
- `rust_adapter_default_source_pattern_matches_rs_files`
- `rust_adapter_default_ignores_target_directory`
- `rust_adapter_detects_workspace_packages_from_cargo_toml`

**Update `#[ignore]` attributes:**

```rust
// BEFORE:
#[test]
#[ignore = "TODO: Phase 302 - Rust Adapter Implementation"]
fn rust_adapter_auto_detected_when_cargo_toml_present() {

// AFTER:
#[test]
fn rust_adapter_auto_detected_when_cargo_toml_present() {
```

**Milestone:** Detection specs pass without `#[ignore]`.

**Verification:**
```bash
cargo test --test specs rust_adapter_auto
cargo test --test specs rust_adapter_default
cargo test --test specs rust_adapter_detects_workspace
```

---

## Key Implementation Details

### Detection Priority

When determining the active adapter:
1. Explicit `[rust]` section in quench.toml takes precedence
2. Presence of `Cargo.toml` triggers auto-detection
3. Fallback to generic adapter

### Default Patterns (from spec)

```toml
[rust]
source = ["**/*.rs"]
tests = ["tests/**", "test/**/*.rs", "*_test.rs", "*_tests.rs"]
ignore = ["target/"]
```

### Workspace Member Expansion

The `[workspace] members` field supports glob patterns:

```toml
[workspace]
members = ["crates/*"]
```

This expands to find actual package directories and reads their names from nested `Cargo.toml` files.

### Test File Classification

A file is classified as `Test` if it matches any:
- Path starts with `tests/` directory
- Path matches `test/**/*.rs`
- Filename ends with `_test.rs` or `_tests.rs`

### Ignore Pattern Application

Ignore patterns are applied at two levels:
1. **File walker**: Skip entire directories (e.g., `target/`)
2. **Adapter classify**: Return `FileKind::Other` for ignored paths

This ensures ignored files are never counted in any check.

### Error Handling

Workspace parsing uses graceful fallback:
- Missing `Cargo.toml`: Empty workspace
- Parse error: Empty workspace
- Invalid member patterns: Skip that pattern

No errors are raised for malformed Cargo.toml during detection.

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test adapter::rust

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run detection specs
cargo test --test specs rust_adapter_auto_detected
cargo test --test specs rust_adapter_default_source
cargo test --test specs rust_adapter_default_ignores
cargo test --test specs rust_adapter_detects_workspace

# Full quality gates
make check
```

### Test Matrix

| Test Case | Fixture | Expected |
|-----------|---------|----------|
| Auto-detect from Cargo.toml | rust/auto-detect | Rust adapter active |
| Source files counted | rust/auto-detect | `source_loc > 0` |
| target/ ignored | rust/auto-detect | No target files in output |
| Workspace packages | rust-workspace | `core`, `cli` detected |

### Integration Test

```bash
# Run quench on the fixture
cd tests/fixtures/rust-workspace
cargo run -- check --cloc --json | jq '.checks[] | select(.name=="cloc")'

# Should show packages: ["cli", "core"]
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Rust adapter skeleton | `adapter/rust.rs` | [ ] Pending |
| 2 | Cargo.toml detection | `adapter/mod.rs` | [ ] Pending |
| 3 | Default patterns | `adapter/rust_tests.rs` | [ ] Pending |
| 4 | Workspace enumeration | `adapter/rust.rs` | [ ] Pending |
| 5 | Integration with walker | `walker.rs`, `config.rs` | [ ] Pending |
| 6 | Enable detection specs | `specs/adapters/rust.rs` | [ ] Pending |

## Future Phases

- **Phase 306**: Rust Escape Patterns (unsafe, unwrap, expect, transmute)
- **Phase 307**: Inline Test Detection (`#[cfg(test)]` block parsing)
- **Phase 308**: Suppress Attribute Checks (`#[allow(...)]` policy)
- **Phase 309**: Lint Config Policy (`lint_changes = "standalone"`)
