# Phase 455: Go Adapter - Detection

**Root Feature:** `quench-333a`

## Overview

Implement the Go language adapter with detection and default patterns. This enables quench to automatically detect Go projects via `go.mod`, apply Go-specific file patterns (source, test, ignore), and provide Go-specific escape patterns. This phase focuses on the core adapter infrastructure; suppress directive parsing and policy checking will follow the patterns established by Rust/Shell adapters.

## Project Structure

```
crates/cli/src/adapter/
├── mod.rs                    # MODIFY: Add go module, GoAdapter export, detect_language update
└── go/                       # NEW: Go adapter module
    ├── mod.rs                # GoAdapter implementation
    ├── suppress.rs           # //nolint directive parsing
    ├── suppress_tests.rs     # Unit tests for suppress parsing
    ├── policy.rs             # Lint policy checking wrapper
    └── policy_tests.rs       # Unit tests for policy checking

crates/cli/src/config/
├── mod.rs                    # MODIFY: Add GoPolicyConfig, GoSuppressConfig
├── go.rs                     # NEW: Go-specific config types
└── parse.rs                  # MODIFY: Add parse_go_config function
```

## Dependencies

No new external dependencies. Uses existing crates:
- `globset` - Pattern matching (already in workspace)
- `regex` - Pattern matching for escape hatches (already in workspace)

## Implementation Phases

### Phase 1: Go Adapter Core Structure

**Goal**: Create the GoAdapter with default patterns and integrate into the registry.

**Tasks**:
1. Create `crates/cli/src/adapter/go/mod.rs` with `GoAdapter` struct
2. Add `pub mod go;` to `crates/cli/src/adapter/mod.rs`
3. Add `ProjectLanguage::Go` variant to `detect_language()`
4. Register `GoAdapter` in `AdapterRegistry::for_project()`
5. Export `GoAdapter` from adapter module

**Key Code**:

```rust
// crates/cli/src/adapter/go/mod.rs

use std::path::Path;
use globset::GlobSet;
use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Default escape patterns for Go.
const GO_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe_pointer",
        pattern: r"unsafe\.Pointer",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining pointer validity.",
    },
    EscapePattern {
        name: "go_linkname",
        pattern: r"//go:linkname",
        action: EscapeAction::Comment,
        comment: Some("// LINKNAME:"),
        advice: "Add a // LINKNAME: comment explaining the external symbol dependency.",
    },
    EscapePattern {
        name: "go_noescape",
        pattern: r"//go:noescape",
        action: EscapeAction::Comment,
        comment: Some("// NOESCAPE:"),
        advice: "Add a // NOESCAPE: comment explaining why escape analysis should be bypassed.",
    },
];

pub struct GoAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl GoAdapter {
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.go".to_string()]),
            test_patterns: build_glob_set(&["**/*_test.go".to_string()]),
            ignore_patterns: build_glob_set(&["vendor/**".to_string()]),
        }
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Adapter for GoAdapter {
    fn name(&self) -> &'static str { "go" }
    fn extensions(&self) -> &'static [&'static str] { &["go"] }
    fn classify(&self, path: &Path) -> FileKind { ... }
    fn default_escapes(&self) -> &'static [EscapePattern] { GO_ESCAPE_PATTERNS }
}
```

**Registry Integration** (in `adapter/mod.rs`):

```rust
// Add to ProjectLanguage enum
pub enum ProjectLanguage {
    Rust,
    Go,      // NEW
    Shell,
    Generic,
}

// Update detect_language()
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }
    if root.join("go.mod").exists() {   // NEW
        return ProjectLanguage::Go;
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
            ProjectLanguage::Rust => registry.register(Arc::new(RustAdapter::new())),
            ProjectLanguage::Go => registry.register(Arc::new(GoAdapter::new())),  // NEW
            ProjectLanguage::Shell => registry.register(Arc::new(ShellAdapter::new())),
            ProjectLanguage::Generic => {}
        }
        registry
    }
}
```

**Verification**:
- `cargo build --all` succeeds
- `cargo test --test specs -- golang::auto_detected` passes (remove `#[ignore]`)

---

### Phase 2: Default Pattern Implementation

**Goal**: Implement file classification with Go's default patterns.

**Tasks**:
1. Implement `classify()` method in GoAdapter
2. Ensure vendor/ files are properly ignored
3. Ensure *_test.go files are classified as Test
4. All other *.go files classified as Source

**Key Code**:

```rust
impl Adapter for GoAdapter {
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
```

**Verification**:
- Remove `#[ignore]` from:
  - `default_source_pattern_matches_go_files`
  - `default_test_pattern_matches_test_files`
  - `default_ignores_vendor_directory`
- `cargo test --test specs -- golang::default` passes

---

### Phase 3: Escape Pattern Integration

**Goal**: Wire Go escape patterns into the escapes check.

**Tasks**:
1. Verify escape patterns are returned by `default_escapes()`
2. Ensure escapes check uses adapter's patterns for .go files
3. Implement comment detection for Go escape patterns

**Escape Pattern Details**:

| Pattern | Regex | Required Comment | Purpose |
|---------|-------|------------------|---------|
| `unsafe.Pointer` | `unsafe\.Pointer` | `// SAFETY:` | Bypasses type safety |
| `//go:linkname` | `//go:linkname` | `// LINKNAME:` | Links to unexported symbols |
| `//go:noescape` | `//go:noescape` | `// NOESCAPE:` | Lies to compiler about escape analysis |

**Comment Detection**: The existing `check_justification_comment()` utility supports Go's `//` comment style. Create a Go-specific `CommentStyle`:

```rust
// In crates/cli/src/adapter/common/suppress.rs
impl CommentStyle {
    pub const GO: Self = Self {
        prefix: "//",
        directive_patterns: &["//go:"],
    };
}
```

**Verification**:
- Remove `#[ignore]` from:
  - `unsafe_pointer_without_safety_comment_fails`
  - `unsafe_pointer_with_safety_comment_passes`
  - `go_linkname_without_linkname_comment_fails`
  - `go_linkname_with_linkname_comment_passes`
  - `go_noescape_without_noescape_comment_fails`
  - `go_noescape_with_noescape_comment_passes`
- `cargo test --test specs -- golang::escape` passes

---

### Phase 4: Suppress Directive Parsing

**Goal**: Parse `//nolint` directives with justification comment checking.

**Tasks**:
1. Create `crates/cli/src/adapter/go/suppress.rs`
2. Create `crates/cli/src/adapter/go/suppress_tests.rs`
3. Parse `//nolint` and `//nolint:code1,code2` formats
4. Support inline reasons: `//nolint:errcheck // reason here`
5. Use `check_justification_comment()` for comment detection

**Key Code**:

```rust
// crates/cli/src/adapter/go/suppress.rs

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// Suppress directive found in Go source code.
#[derive(Debug, Clone)]
pub struct NolintDirective {
    pub line: usize,
    pub codes: Vec<String>,         // Empty = all linters
    pub has_comment: bool,
    pub comment_text: Option<String>,
}

/// Parse //nolint directives from Go source.
pub fn parse_nolint_directives(content: &str, comment_pattern: Option<&str>) -> Vec<NolintDirective> {
    let lines: Vec<&str> = content.lines().collect();
    let mut directives = Vec::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match //nolint or //nolint:codes
        if let Some(directive) = parse_nolint_line(trimmed) {
            let (has_comment, comment_text) = if directive.has_inline_comment {
                // Inline comment counts as justification
                (true, directive.inline_comment.clone())
            } else {
                check_justification_comment(&lines, line_idx, comment_pattern, &CommentStyle::GO)
            };

            directives.push(NolintDirective {
                line: line_idx,
                codes: directive.codes,
                has_comment,
                comment_text,
            });
        }
    }

    directives
}

fn parse_nolint_line(line: &str) -> Option<ParsedNolint> {
    // Match: //nolint, //nolint:code, //nolint:code1,code2
    // Optional trailing: // reason
    if !line.contains("//nolint") {
        return None;
    }
    // ... parsing logic
}
```

**Verification**:
- Remove `#[ignore]` from:
  - `nolint_without_comment_fails_when_comment_required`
  - `nolint_with_comment_passes`
- `cargo test --test specs -- golang::nolint` passes
- Unit tests in `suppress_tests.rs` pass

---

### Phase 5: Config and Policy Integration

**Goal**: Add Go-specific configuration and policy checking.

**Tasks**:
1. Create `crates/cli/src/config/go.rs` with `GoConfig`, `GoPolicyConfig`, `GoSuppressConfig`
2. Add `go: Option<toml::Value>` to `FlexibleConfig`
3. Add `pub go: GoConfig` to `Config`
4. Implement `parse_go_config()` in parse.rs
5. Create policy wrapper in `crates/cli/src/adapter/go/policy.rs`
6. Implement `PolicyConfig` trait for `GoPolicyConfig`

**Config Structure**:

```rust
// crates/cli/src/config/go.rs

/// Go language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GoConfig {
    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: GoSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: GoPolicyConfig,
}

/// Go lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GoPolicyConfig {
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    #[serde(default = "GoPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl GoPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            ".golangci.yml".to_string(),
            ".golangci.yaml".to_string(),
            ".golangci.toml".to_string(),
        ]
    }
}

impl PolicyConfig for GoPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy { self.lint_changes }
    fn lint_config(&self) -> &[String] { &self.lint_config }
}
```

**Verification**:
- Remove `#[ignore]` from:
  - `lint_config_changes_with_source_fails_standalone_policy`
- `cargo test --test specs -- golang::lint_config` passes
- Unit tests in `policy_tests.rs` pass

---

### Phase 6: Module and Package Detection (Optional)

**Goal**: Extract module name and enumerate packages from directory structure.

**Tasks**:
1. Add `parse_go_mod()` function to extract module name
2. Add `enumerate_packages()` to discover package directories
3. Expose module/package info in check context

**Key Code**:

```rust
// crates/cli/src/adapter/go/mod.rs

/// Parse go.mod to extract module name.
pub fn parse_go_mod(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            return Some(trimmed.strip_prefix("module ")?.trim().to_string());
        }
    }
    None
}

/// Enumerate packages from directory structure.
/// Returns paths relative to the module root that contain .go files.
pub fn enumerate_packages(root: &Path) -> Vec<String> {
    // Walk directories, find those containing *.go files (excluding *_test.go only dirs)
    // Return relative paths like ".", "internal/config", "pkg/api"
    ...
}
```

**Verification**:
- Remove `#[ignore]` from:
  - `detects_module_name_from_go_mod`
  - `detects_packages_from_directory_structure`
- `cargo test --test specs -- golang::module` passes

---

## Key Implementation Details

### Pattern Precedence

File classification follows this order:
1. **Ignore** - `vendor/**` paths return `FileKind::Other`
2. **Test** - `**/*_test.go` paths return `FileKind::Test`
3. **Source** - `**/*.go` paths return `FileKind::Source`
4. **Other** - Non-Go files return `FileKind::Other`

### Escape Pattern Comment Detection

Go uses `//` comments. The comment must appear on the line immediately before the escape pattern:

```go
// SAFETY: Converting pointer to access underlying memory layout
ptr := unsafe.Pointer(uintptr(0x1234))
```

For directives like `//go:linkname`, the comment must precede the directive:

```go
// LINKNAME: Accessing runtime internal for profiling
//go:linkname runtimeNano runtime.nanotime
```

### Nolint Directive Formats

Support all golangci-lint formats:
- `//nolint` - suppress all linters (discouraged)
- `//nolint:errcheck` - suppress specific linter
- `//nolint:errcheck,gosec` - suppress multiple linters
- `//nolint:errcheck // reason here` - inline justification

### Adapter Selection Priority

After this phase, language detection order is:
1. `rust` - `Cargo.toml` exists
2. `go` - `go.mod` exists
3. `shell` - `*.sh` in root, bin/, or scripts/
4. `generic` - fallback

## Verification Plan

### Unit Tests

Each module has a corresponding `*_tests.rs` file:
- `go/suppress_tests.rs` - nolint parsing edge cases
- `go/policy_tests.rs` - policy checking scenarios

Run with: `cargo test adapter::go`

### Behavioral Specs

Remove `#[ignore = "TODO: Phase 455+"]` from tests in `tests/specs/adapters/golang.rs` as features are implemented.

Run with: `cargo test --test specs -- golang`

### Full Verification

```bash
# All checks pass
make check

# List enabled golang specs
cargo test --test specs -- golang --list

# Confirm all golang specs are enabled (not ignored)
cargo test --test specs -- golang 2>&1 | grep -c "ignored" # Should be 0
```

### Expected Final State

All 15 Go adapter specs should pass:
```
test specs::adapters::golang::auto_detected_when_go_mod_present ... ok
test specs::adapters::golang::default_source_pattern_matches_go_files ... ok
test specs::adapters::golang::default_test_pattern_matches_test_files ... ok
test specs::adapters::golang::default_ignores_vendor_directory ... ok
test specs::adapters::golang::detects_module_name_from_go_mod ... ok
test specs::adapters::golang::detects_packages_from_directory_structure ... ok
test specs::adapters::golang::unsafe_pointer_without_safety_comment_fails ... ok
test specs::adapters::golang::unsafe_pointer_with_safety_comment_passes ... ok
test specs::adapters::golang::go_linkname_without_linkname_comment_fails ... ok
test specs::adapters::golang::go_linkname_with_linkname_comment_passes ... ok
test specs::adapters::golang::go_noescape_without_noescape_comment_fails ... ok
test specs::adapters::golang::go_noescape_with_noescape_comment_passes ... ok
test specs::adapters::golang::nolint_without_comment_fails_when_comment_required ... ok
test specs::adapters::golang::nolint_with_comment_passes ... ok
test specs::adapters::golang::lint_config_changes_with_source_fails_standalone_policy ... ok
```

## Checklist

- [ ] Phase 1: GoAdapter core structure + registry integration
- [ ] Phase 2: Default pattern implementation (source, test, ignore)
- [ ] Phase 3: Escape pattern integration (3 patterns)
- [ ] Phase 4: Suppress directive parsing (//nolint)
- [ ] Phase 5: Config and policy integration
- [ ] Phase 6: Module and package detection (optional)
- [ ] Final: `make check` passes, all golang specs pass
