# Phase 201: Generic Language Adapter

**Root Feature:** `quench-lang-adapters`

## Overview

Implement a language adapter abstraction that provides language-specific behavior for checks. This phase focuses on the `GenericAdapter` (fallback adapter) which uses patterns from `[project]` config for source/test file detection and has no default escape patterns.

The adapter system enables:
- **Source detection**: Pattern-based classification from `[project].source`
- **Test detection**: Pattern-based classification from `[project].tests`
- **Escape patterns**: Language-specific defaults (none for generic)
- **Extension-based selection**: Route files to appropriate adapters

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── mod.rs           # Adapter trait + registry
│   │   ├── mod_tests.rs     # Adapter tests
│   │   ├── generic.rs       # GenericAdapter implementation
│   │   └── generic_tests.rs # Generic adapter tests
│   ├── config.rs            # Add [project].source, [project].tests patterns
│   ├── lib.rs               # Export adapter module
│   └── checks/
│       └── cloc.rs          # Integrate with adapter for file classification
└── plans/
    └── phase-201.md
```

## Dependencies

No new external dependencies. Uses existing:
- `globset` (pattern matching, already in use)
- `serde` (config parsing, already in use)

## Implementation Phases

### Phase 1: Extend Config for Project Source/Test Patterns

Add `source` and `tests` patterns to `ProjectConfig` in `config.rs`.

**Changes to `config.rs`:**

```rust
/// Project-level configuration.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: Option<String>,

    /// Source file patterns (default: language-specific or none).
    #[serde(default)]
    pub source: Vec<String>,

    /// Test file patterns (default: common test patterns).
    #[serde(default = "ProjectConfig::default_test_patterns")]
    pub tests: Vec<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,
}

impl ProjectConfig {
    fn default_test_patterns() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
        ]
    }
}
```

Update `parse_with_warnings` to handle the new fields.

**Milestone:** Config parses `[project].source` and `[project].tests` patterns.

**Verification:**
```bash
cargo test -p quench -- config
cargo clippy -p quench -- -D warnings
```

---

### Phase 2: Define Adapter Trait

Create the adapter abstraction in a new `adapter` module.

**New file `crates/cli/src/adapter/mod.rs`:**

```rust
//! Language adapters provide language-specific behavior for checks.
//!
//! See docs/specs/10-language-adapters.md for specification.

use std::path::Path;

pub mod generic;

pub use generic::GenericAdapter;

/// File classification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Production source code.
    Source,
    /// Test code (unit tests, integration tests).
    Test,
    /// Not a source file (config, data, etc.).
    Other,
}

/// A language adapter provides language-specific behavior for checks.
///
/// Adapters are responsible for:
/// - Classifying files as source, test, or other
/// - Providing default escape patterns
/// - (Future) Inline test detection, lint suppression patterns
pub trait Adapter: Send + Sync {
    /// Adapter identifier (e.g., "rust", "shell", "generic").
    fn name(&self) -> &'static str;

    /// File extensions this adapter handles (e.g., ["rs"] for Rust).
    /// Empty slice means this adapter doesn't match by extension (generic fallback).
    fn extensions(&self) -> &'static [&'static str];

    /// Classify a file by its path relative to the project root.
    fn classify(&self, path: &Path) -> FileKind;

    /// Default escape patterns for this language.
    /// Returns empty slice for languages with no default escapes (generic).
    fn default_escapes(&self) -> &'static [EscapePattern] {
        &[]
    }
}

/// An escape pattern with its action.
#[derive(Debug, Clone)]
pub struct EscapePattern {
    /// Pattern name for reporting (e.g., "unsafe", "unwrap").
    pub name: &'static str,
    /// Regex pattern to match.
    pub pattern: &'static str,
    /// Required action for this escape.
    pub action: EscapeAction,
    /// Required comment pattern (for Comment action).
    pub comment: Option<&'static str>,
}

/// Action required for an escape pattern match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeAction {
    /// Just count occurrences.
    Count,
    /// Require a justification comment.
    Comment,
    /// Never allowed.
    Forbid,
}
```

**Milestone:** `Adapter` trait defined with `FileKind` and escape pattern types.

**Verification:**
```bash
cargo check -p quench
```

---

### Phase 3: Implement GenericAdapter

Create the generic/fallback adapter that uses `[project]` config patterns.

**New file `crates/cli/src/adapter/generic.rs`:**

```rust
//! Generic language adapter (fallback).
//!
//! Uses patterns from [project] config for file classification.
//! Has no default escape patterns.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::{Adapter, FileKind};

/// Generic adapter that uses project config patterns.
///
/// This is the fallback adapter for files that don't match
/// any language-specific adapter.
pub struct GenericAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
}

impl GenericAdapter {
    /// Create a new generic adapter from config patterns.
    pub fn new(source_patterns: &[String], test_patterns: &[String]) -> Self {
        Self {
            source_patterns: build_glob_set(source_patterns),
            test_patterns: build_glob_set(test_patterns),
        }
    }

    /// Create with default patterns (no source filter, common test patterns).
    pub fn with_defaults() -> Self {
        Self::new(
            &[],
            &[
                "**/tests/**".to_string(),
                "**/test/**".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.test.*".to_string(),
                "**/*.spec.*".to_string(),
            ],
        )
    }
}

impl Adapter for GenericAdapter {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn extensions(&self) -> &'static [&'static str] {
        // Generic adapter doesn't match by extension
        // It's selected as fallback when no other adapter matches
        &[]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // If source patterns are configured, file must match
        if !self.source_patterns.is_empty() {
            if self.source_patterns.is_match(path) {
                return FileKind::Source;
            }
            return FileKind::Other;
        }

        // No source patterns configured = all non-test files are source
        FileKind::Source
    }

    // No default escapes for generic adapter
    // fn default_escapes() uses trait default: &[]
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        } else {
            tracing::warn!("invalid glob pattern: {}", pattern);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

#[cfg(test)]
#[path = "generic_tests.rs"]
mod tests;
```

**Milestone:** `GenericAdapter` classifies files based on patterns.

**Verification:**
```bash
cargo test -p quench -- generic
```

---

### Phase 4: Create Adapter Registry

Add an adapter registry that selects adapters based on file extension.

**Add to `crates/cli/src/adapter/mod.rs`:**

```rust
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available adapters.
pub struct AdapterRegistry {
    /// Adapters by extension (e.g., "rs" -> RustAdapter).
    by_extension: HashMap<&'static str, Arc<dyn Adapter>>,
    /// Fallback adapter for unrecognized extensions.
    fallback: Arc<dyn Adapter>,
}

impl AdapterRegistry {
    /// Create a new registry with the given fallback adapter.
    pub fn new(fallback: Arc<dyn Adapter>) -> Self {
        Self {
            by_extension: HashMap::new(),
            fallback,
        }
    }

    /// Register an adapter for its declared extensions.
    pub fn register(&mut self, adapter: Arc<dyn Adapter>) {
        for ext in adapter.extensions() {
            self.by_extension.insert(ext, Arc::clone(&adapter));
        }
    }

    /// Get the adapter for a file path based on extension.
    pub fn adapter_for(&self, path: &Path) -> &dyn Adapter {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        self.by_extension
            .get(ext)
            .map(|a| a.as_ref())
            .unwrap_or(self.fallback.as_ref())
    }

    /// Classify a file using the appropriate adapter.
    pub fn classify(&self, path: &Path) -> FileKind {
        self.adapter_for(path).classify(path)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new(Arc::new(GenericAdapter::with_defaults()))
    }
}
```

**Milestone:** Registry routes files to adapters by extension.

**Verification:**
```bash
cargo test -p quench -- adapter
```

---

### Phase 5: Integrate with CLOC Check

Refactor `cloc.rs` to use the adapter system for file classification instead of its internal `PatternMatcher`.

**Changes to `crates/cli/src/checks/cloc.rs`:**

1. Accept `AdapterRegistry` in `CheckContext` or construct from config
2. Replace `PatternMatcher::is_test_file()` with `registry.classify()`
3. Keep `PatternMatcher` for exclude patterns only (or move exclude logic to adapter)

```rust
// In run():
let registry = AdapterRegistry::from_config(&ctx.config);

for file in ctx.files {
    // Use adapter for classification
    let kind = registry.classify(&file.path);
    let is_test = kind == FileKind::Test;

    // ... rest of logic unchanged
}
```

**Option A: Keep PatternMatcher for excludes only**
- Simpler migration
- PatternMatcher handles exclude patterns
- Adapter handles source/test classification

**Option B: Move exclude to adapter**
- Cleaner long-term
- Adapter.classify() returns Other for excluded files
- Requires exclude patterns in adapter config

Recommend **Option A** for this phase to minimize changes.

**Milestone:** CLOC check uses adapter for source/test classification.

**Verification:**
```bash
cargo test -p quench -- cloc
# Ensure existing CLOC behavior unchanged
```

---

### Phase 6: Unit Tests and Documentation

Add comprehensive tests for the adapter system.

**Test cases for `generic_tests.rs`:**

```rust
use super::*;
use yare::parameterized;

#[parameterized(
    // Test directory patterns
    tests_dir = { "tests/foo.rs", FileKind::Test },
    test_dir = { "test/bar.py", FileKind::Test },
    nested_tests = { "crate/tests/unit.rs", FileKind::Test },

    // Test file patterns
    suffix_test = { "src/foo_test.rs", FileKind::Test },
    suffix_tests = { "src/bar_tests.rs", FileKind::Test },
    dot_test = { "src/baz.test.js", FileKind::Test },
    dot_spec = { "src/qux.spec.ts", FileKind::Test },

    // Source files
    src_file = { "src/lib.rs", FileKind::Source },
    root_file = { "main.py", FileKind::Source },
    nested_src = { "pkg/internal/util.go", FileKind::Source },
)]
fn classify_with_defaults(path: &str, expected: FileKind) {
    let adapter = GenericAdapter::with_defaults();
    assert_eq!(adapter.classify(Path::new(path)), expected);
}

#[test]
fn classify_with_source_patterns() {
    let adapter = GenericAdapter::new(
        &["src/**/*".to_string(), "lib/**/*".to_string()],
        &["**/tests/**".to_string()],
    );

    assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("lib/util.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("bin/cli.rs")), FileKind::Other);
    assert_eq!(adapter.classify(Path::new("src/tests/unit.rs")), FileKind::Test);
}

#[test]
fn no_default_escapes() {
    let adapter = GenericAdapter::with_defaults();
    assert!(adapter.default_escapes().is_empty());
}
```

**Test cases for `mod_tests.rs` (registry):**

```rust
#[test]
fn registry_fallback_to_generic() {
    let registry = AdapterRegistry::default();
    let adapter = registry.adapter_for(Path::new("unknown.xyz"));
    assert_eq!(adapter.name(), "generic");
}

#[test]
fn registry_extension_lookup() {
    // Will test this when Rust adapter is added
    // For now, verify fallback works for all extensions
    let registry = AdapterRegistry::default();
    assert_eq!(registry.adapter_for(Path::new("foo.rs")).name(), "generic");
    assert_eq!(registry.adapter_for(Path::new("bar.py")).name(), "generic");
}
```

**Milestone:** Full test coverage for adapter system.

**Verification:**
```bash
cargo test -p quench -- adapter
cargo test -p quench -- generic
make check
```

## Key Implementation Details

### Why Test Patterns Take Precedence

A file like `tests/test_utils.rs` could match both source and test patterns. Test patterns take precedence because:
1. Test files should use higher line limits
2. Test-specific escape handling (more lenient)
3. Matches developer intent (file in test directory = test code)

### Empty Source Patterns = All Files Are Source

When `[project].source` is not configured:
- All files not matching test patterns are classified as Source
- This matches the "convention over configuration" principle
- Users who want to restrict source files can add explicit patterns

### Pattern Matching Uses Relative Paths

The adapter receives paths relative to project root:
- `src/lib.rs` not `/home/user/project/src/lib.rs`
- Consistent with gitignore-style patterns
- Matches user expectations from config

### Escape Patterns Are Static

Default escape patterns are compile-time constants (`&'static [EscapePattern]`):
- No allocation overhead
- Language adapters define their escapes at compile time
- User-configured escapes merge with defaults (future phase)

## Verification Plan

### Phase-by-Phase Verification

Each phase includes specific verification commands. Run after completing each phase.

### Full Quality Gates

Before committing any changes:

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `./scripts/bootstrap`
- `cargo audit`
- `cargo deny check`

### Behavioral Verification

Ensure CLOC check behavior is unchanged after integration:

```bash
# Run existing behavioral tests
cargo test --test cloc

# Manual verification on quench itself
cargo run -- check --cloc
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Extend config for source/test patterns | `config.rs` | [ ] Pending |
| 2 | Define Adapter trait | `adapter/mod.rs` | [ ] Pending |
| 3 | Implement GenericAdapter | `adapter/generic.rs` | [ ] Pending |
| 4 | Create Adapter Registry | `adapter/mod.rs` | [ ] Pending |
| 5 | Integrate with CLOC check | `checks/cloc.rs` | [ ] Pending |
| 6 | Unit tests and documentation | `*_tests.rs` | [ ] Pending |

## Future Work (Not This Phase)

- **Phase 202**: Rust adapter with `#[cfg(test)]` inline detection
- **Phase 203**: Shell adapter with `.bats` test detection
- **Phase 204**: Escape pattern merging (defaults + user config)
- **Phase 205**: Language auto-detection from project files
