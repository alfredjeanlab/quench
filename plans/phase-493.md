# Phase 493: JavaScript Adapter - Detection

## Overview

Implement JavaScript/TypeScript project detection for quench. This enables automatic language detection when `package.json`, `tsconfig.json`, or `jsconfig.json` is present, along with default source/test/ignore patterns and workspace detection for npm/yarn/pnpm monorepos.

This phase builds the foundation for JS/TS support. Later phases (495-496) will add escape patterns and suppress directive handling.

## Project Structure

```
crates/cli/src/adapter/
├── mod.rs                  # Add JavaScript to ProjectLanguage, detect_language()
├── javascript/
│   ├── mod.rs             # JavaScriptAdapter implementation
│   ├── workspace.rs       # npm/yarn/pnpm workspace detection
│   └── workspace_tests.rs # Unit tests for workspace parsing
└── javascript_tests.rs    # Unit tests for adapter
```

Test fixtures (already exist from phase-491):
```
tests/fixtures/javascript/
├── auto-detect/           # package.json detection
├── tsconfig-detect/       # tsconfig.json detection
├── jsconfig-detect/       # jsconfig.json detection
├── default-patterns/      # Source/test pattern matching
├── node-modules-ignore/   # Ignore pattern testing
├── workspace-npm/         # npm workspaces
└── workspace-pnpm/        # pnpm workspaces
```

## Dependencies

**Crate dependencies** (already present):
- `globset` - Pattern matching for source/test classification
- `serde_json` - Parsing package.json
- `serde_yaml` - Parsing pnpm-workspace.yaml (may need to add)

**Internal dependencies**:
- `adapter::Adapter` trait (Phase 201 - already complete)
- `adapter::glob::build_glob_set()` - Shared pattern builder

## Implementation Phases

### Phase 1: Core Adapter Structure

Create the JavaScript adapter module with basic detection.

**Files to create:**
- `crates/cli/src/adapter/javascript/mod.rs`
- `crates/cli/src/adapter/javascript_tests.rs`

**Implementation:**

```rust
// crates/cli/src/adapter/javascript/mod.rs
use std::path::Path;
use globset::GlobSet;
use super::{Adapter, FileKind, EscapePattern};
use super::glob::build_glob_set;

mod workspace;
pub use workspace::JsWorkspace;

/// JavaScript/TypeScript language adapter.
pub struct JavaScriptAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl JavaScriptAdapter {
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&[
                "**/*.js".to_string(),
                "**/*.jsx".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
                "**/*.mjs".to_string(),
                "**/*.mts".to_string(),
            ]),
            test_patterns: build_glob_set(&[
                "**/*.test.js".to_string(),
                "**/*.test.ts".to_string(),
                "**/*.test.jsx".to_string(),
                "**/*.test.tsx".to_string(),
                "**/*.spec.js".to_string(),
                "**/*.spec.ts".to_string(),
                "**/*.spec.jsx".to_string(),
                "**/*.spec.tsx".to_string(),
                "**/__tests__/**".to_string(),
                "test/**".to_string(),
                "tests/**".to_string(),
            ]),
            ignore_patterns: build_glob_set(&[
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".next/**".to_string(),
                "coverage/**".to_string(),
            ]),
        }
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Adapter for JavaScriptAdapter {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "ts", "tsx", "mjs", "mts"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        if self.should_ignore(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }

        FileKind::Other
    }

    // Note: default_escapes() will be added in Phase 495
}
```

**Verification:**
- Unit tests for file classification pass
- Test patterns correctly identify test files

### Phase 2: Project Detection Integration

Add JavaScript to the project language detection system.

**Files to modify:**
- `crates/cli/src/adapter/mod.rs`

**Changes:**

```rust
// Add to ProjectLanguage enum
pub enum ProjectLanguage {
    Rust,
    Go,
    Shell,
    JavaScript,  // NEW
    Generic,
}

// Update detect_language()
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }

    if root.join("go.mod").exists() {
        return ProjectLanguage::Go;
    }

    // NEW: JavaScript detection (before Shell check)
    if root.join("package.json").exists()
        || root.join("tsconfig.json").exists()
        || root.join("jsconfig.json").exists()
    {
        return ProjectLanguage::JavaScript;
    }

    if has_shell_markers(root) {
        return ProjectLanguage::Shell;
    }

    ProjectLanguage::Generic
}

// Update AdapterRegistry::for_project()
impl AdapterRegistry {
    pub fn for_project(root: &Path) -> Self {
        let mut registry = Self::new(Arc::new(GenericAdapter::with_defaults()));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                registry.register(Arc::new(RustAdapter::new()));
            }
            ProjectLanguage::Go => {
                registry.register(Arc::new(GoAdapter::new()));
            }
            ProjectLanguage::JavaScript => {  // NEW
                registry.register(Arc::new(JavaScriptAdapter::new()));
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

**Verification:**
- `detect_language()` returns `ProjectLanguage::JavaScript` for fixtures
- Specs `auto_detected_when_*` pass (remove `#[ignore]`)

### Phase 3: Package Name Extraction

Extract package name from `package.json` for metrics reporting.

**Files to modify:**
- `crates/cli/src/adapter/javascript/mod.rs`

**Implementation:**

```rust
use std::fs;
use serde_json::Value;

impl JavaScriptAdapter {
    /// Read package name from package.json at the given root.
    pub fn package_name(root: &Path) -> Option<String> {
        let pkg_json = root.join("package.json");
        let content = fs::read_to_string(&pkg_json).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;
        value.get("name")?.as_str().map(String::from)
    }
}
```

**Verification:**
- Unit tests for package name extraction
- Returns `None` for missing/invalid package.json

### Phase 4: Workspace Detection

Detect and enumerate packages in npm/yarn/pnpm workspaces.

**Files to create:**
- `crates/cli/src/adapter/javascript/workspace.rs`
- `crates/cli/src/adapter/javascript/workspace_tests.rs`

**Implementation:**

```rust
// crates/cli/src/adapter/javascript/workspace.rs
use std::fs;
use std::path::Path;
use serde_json::Value;

/// JavaScript workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct JsWorkspace {
    /// Is this a workspace root?
    pub is_workspace: bool,
    /// Package names in the workspace.
    pub packages: Vec<String>,
    /// Workspace patterns (e.g., ["packages/*"]).
    pub patterns: Vec<String>,
}

impl JsWorkspace {
    /// Parse workspace info from project root.
    ///
    /// Checks in order:
    /// 1. pnpm-workspace.yaml (pnpm)
    /// 2. package.json workspaces field (npm/yarn)
    pub fn from_root(root: &Path) -> Self {
        // Check pnpm-workspace.yaml first
        if let Some(ws) = Self::from_pnpm_workspace(root) {
            return ws;
        }

        // Fall back to package.json workspaces
        Self::from_package_json(root).unwrap_or_default()
    }

    fn from_pnpm_workspace(root: &Path) -> Option<Self> {
        let path = root.join("pnpm-workspace.yaml");
        let content = fs::read_to_string(&path).ok()?;
        let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;

        let patterns = value.get("packages")?
            .as_sequence()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>();

        if patterns.is_empty() {
            return None;
        }

        let packages = expand_workspace_patterns(&patterns, root);
        Some(Self {
            is_workspace: true,
            packages,
            patterns,
        })
    }

    fn from_package_json(root: &Path) -> Option<Self> {
        let path = root.join("package.json");
        let content = fs::read_to_string(&path).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;

        let workspaces = value.get("workspaces")?;

        // Handle both array and object forms
        let patterns = match workspaces {
            Value::Array(arr) => arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Value::Object(obj) => {
                // { "packages": ["..."] } form
                obj.get("packages")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }
            _ => return None,
        };

        if patterns.is_empty() {
            return None;
        }

        let packages = expand_workspace_patterns(&patterns, root);
        Some(Self {
            is_workspace: true,
            packages,
            patterns,
        })
    }
}

/// Expand workspace patterns to find package names.
fn expand_workspace_patterns(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if let Some(base) = pattern.strip_suffix("/*") {
            // Single-level glob: packages/*
            expand_single_level(root, base, &mut packages);
        } else if let Some(base) = pattern.strip_suffix("/**") {
            // Recursive glob: packages/** (treat as single level)
            expand_single_level(root, base, &mut packages);
        } else if !pattern.contains('*') {
            // Direct path: packages/core
            let pkg_dir = root.join(pattern);
            if let Some(name) = read_package_name(&pkg_dir) {
                packages.push(name);
            }
        }
    }

    packages.sort();
    packages
}

fn expand_single_level(root: &Path, base: &str, packages: &mut Vec<String>) {
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

fn read_package_name(dir: &Path) -> Option<String> {
    let pkg_json = dir.join("package.json");
    let content = fs::read_to_string(&pkg_json).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    value.get("name")?.as_str().map(String::from)
}
```

**Verification:**
- Unit tests for npm workspace detection
- Unit tests for pnpm workspace detection
- Specs `detects_npm_workspaces_from_package_json` and `detects_pnpm_workspaces` pass

### Phase 5: Integration and Spec Verification

Wire everything together and verify all Phase 493 specs pass.

**Files to modify:**
- `crates/cli/src/adapter/mod.rs` - Export JavaScriptAdapter
- `tests/specs/adapters/javascript.rs` - Remove `#[ignore]` from Phase 493 specs

**Specs to un-ignore:**
```rust
// AUTO-DETECTION SPECS
auto_detected_when_package_json_present()
auto_detected_when_tsconfig_json_present()
auto_detected_when_jsconfig_json_present()

// DEFAULT PATTERN SPECS
default_source_pattern_matches_js_ts_files()
default_test_pattern_matches_test_files()
default_ignores_node_modules_directory()

// WORKSPACE DETECTION SPECS
detects_npm_workspaces_from_package_json()
detects_pnpm_workspaces()
```

**Verification:**
- All Phase 493 specs pass
- `make check` passes
- Integration test on `fixtures/javascript/*` works

## Key Implementation Details

### Detection Priority

JavaScript detection happens after Rust and Go but before Shell:

```
Cargo.toml → Rust
go.mod → Go
package.json/tsconfig.json/jsconfig.json → JavaScript
*.sh in root/bin/scripts → Shell
(fallback) → Generic
```

This prevents false positives - a Rust project with JS tooling (common for web frontends) should detect as Rust.

### Test File Identification

Test files are identified by pattern matching, not content analysis (unlike Rust's `#[cfg(test)]`):

| Pattern | Match |
|---------|-------|
| `*.test.{js,ts,jsx,tsx}` | Jest/Vitest naming |
| `*.spec.{js,ts,jsx,tsx}` | Jest/Vitest alt naming |
| `__tests__/**` | Jest convention |
| `test/**`, `tests/**` | Common directories |

### Workspace Pattern Expansion

Both npm/yarn and pnpm use similar glob patterns:
- `packages/*` - Single-level glob
- `packages/**` - Recursive (treated as single-level)
- `apps/web` - Direct path

The implementation mirrors the Rust workspace expansion in `adapter/rust/workspace.rs`.

### File Extensions

All JS/TS extensions are registered with the adapter:
- `.js`, `.jsx` - JavaScript
- `.ts`, `.tsx` - TypeScript
- `.mjs`, `.mts` - ES modules (explicit)

Note: `.cjs`, `.cts` (CommonJS) could be added if needed.

## Verification Plan

### Unit Tests

1. **File classification tests** (`javascript_tests.rs`):
   - Source files correctly classified
   - Test files (all patterns) correctly classified
   - Ignored directories (node_modules, dist) excluded

2. **Package name tests** (`javascript_tests.rs`):
   - Extracts name from valid package.json
   - Returns None for missing/invalid files

3. **Workspace tests** (`workspace_tests.rs`):
   - npm workspaces detected from package.json
   - pnpm workspaces detected from pnpm-workspace.yaml
   - Yarn workspaces (object form) detected
   - Package enumeration from glob patterns

### Behavioral Specs

Run specs to verify end-to-end behavior:

```bash
# Run Phase 493 specs specifically
cargo test --test specs adapters::javascript

# Run all specs
cargo test --test specs
```

Expected: All 8 Phase 493 specs pass (those with `#[ignore = "TODO: Phase 493"]`)

### Integration Test

Manual verification on fixture projects:

```bash
# Test auto-detection
cargo run -- check tests/fixtures/javascript/auto-detect

# Test workspace detection
cargo run -- check tests/fixtures/javascript/workspace-npm
cargo run -- check tests/fixtures/javascript/workspace-pnpm

# Test ignore patterns
cargo run -- check tests/fixtures/javascript/node-modules-ignore
```

### Full Check

```bash
make check
```

All checks must pass:
- `cargo fmt`
- `cargo clippy`
- `cargo test`
- `cargo build`
- `./scripts/bootstrap`
- `cargo audit`
- `cargo deny check`
