# Test Specification Architecture

This document describes the behavioral specification (spec) testing architecture for quench.

## Design Goals

1. **Specs before implementation**: Behavioral specs derived from docs/specs/ drive development
2. **Black-box testing**: Specs test CLI behavior, never internal modules
3. **Fast feedback**: Full spec suite runs in < 5 seconds
4. **Incremental progress**: Unimplemented specs are marked, not deleted

## Architecture Overview

```
tests/
├── fixtures/              # Static test projects
│   ├── minimal/           # Empty project, no config
│   ├── rust-simple/       # Small Rust project
│   ├── rust-workspace/    # Multi-package workspace
│   ├── shell-scripts/     # Shell + bats
│   ├── mixed/             # Rust + shell
│   ├── violations/        # Intentional violations
│   ├── docs-project/      # docs/, TOC, links
│   └── agents-project/    # CLAUDE.md, .cursorrules
│
├── specs/                 # Behavioral specifications
│   ├── mod.rs             # Harness, helpers, re-exports
│   ├── cli/               # CLI behavior
│   ├── config/            # Configuration
│   ├── checks/            # Per-check specs
│   ├── adapters/          # Language adapters
│   ├── output/            # Output formats
│   └── modes/             # Operating modes
│
└── snapshots/             # insta snapshot files
```

## Black-Box Constraint

Specs invoke the CLI binary and check outputs. Internal modules are never imported.

```rust
// ✓ Black-box: invoke binary, check output
check("cloc").on("rust-simple").passes();

// ✗ White-box: import internals
use quench::checks::cloc::count_lines;  // FORBIDDEN
```

This ensures specs remain valid documentation of external behavior regardless of implementation changes.

## Helper API

The spec harness provides a fluent API for concise, readable tests.

### Core Helpers

```rust
use crate::prelude::*;

/// Get path to a fixture directory
fn fixture(name: &str) -> PathBuf;

/// Create a quench command (low-level)
fn quench() -> Command;

/// Start a check spec (high-level, preferred)
fn check(name: &str) -> CheckBuilder;
```

### CheckBuilder

The `CheckBuilder` provides a fluent interface for the common case: running a single check against a fixture and asserting the result.

```rust
/// Spec: docs/specs/checks/cloc.md#counting-rules
#[test]
fn counts_non_blank_lines() {
    check("cloc")
        .on("rust-simple")
        .passes();
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
#[test]
fn fails_on_oversized_file() {
    check("cloc")
        .on("violations")
        .fails()
        .with_violation("oversized.rs");
}
```

### Builder Methods

```rust
impl CheckBuilder {
    /// Set fixture directory
    fn on(self, fixture: &str) -> Self;

    /// Set working directory (alternative to fixture)
    fn in_dir(self, path: impl AsRef<Path>) -> Self;

    /// Add CLI arguments
    fn args(self, args: &[&str]) -> Self;

    /// Request JSON output
    fn json(self) -> Self;

    /// Assert success (exit 0)
    fn passes(self) -> AssertResult;

    /// Assert failure (exit non-zero)
    fn fails(self) -> FailureAssert;

    /// Get raw command for complex cases
    fn command(self) -> Command;
}

impl FailureAssert {
    /// Assert stdout contains violation for file
    fn with_violation(self, file: &str) -> Self;

    /// Assert stdout contains text
    fn with_output(self, text: &str) -> Self;

    /// Assert stderr contains text
    fn with_error(self, text: &str) -> Self;

    /// Get underlying assertion for custom checks
    fn assert(self) -> Assert;
}

impl AssertResult {
    /// Parse JSON output and run assertions
    fn json<F>(self, f: F) where F: FnOnce(&serde_json::Value);

    /// Snapshot test the output
    fn snapshot(self);
}
```

### JSON Assertions

For specs that need to verify structured output:

```rust
/// Spec: docs/specs/checks/cloc.md#json-output
#[test]
fn json_includes_ratio() {
    check("cloc")
        .json()
        .on("rust-simple")
        .passes()
        .json(|output| {
            let ratio = output["checks"][0]["metrics"]["ratio"].as_f64();
            assert!(ratio.is_some(), "ratio should be a number");
        });
}
```

### Snapshot Testing

For verifying exact output format:

```rust
/// Spec: docs/specs/03-output.md#text-format
#[test]
fn cloc_text_output_format() {
    check("cloc")
        .on("violations")
        .fails()
        .snapshot();
}
```

Snapshots use insta and require explicit `cargo insta review` to change.

### Multi-Check Specs

For specs testing multiple checks or complex scenarios, use the low-level `quench()` helper:

```rust
/// Spec: docs/specs/01-cli.md#check-selection
#[test]
fn can_run_multiple_checks() {
    quench()
        .args(["check", "--cloc", "--escapes"])
        .current_dir(fixture("rust-simple"))
        .assert()
        .success();
}
```

### Temporary Directories

For config parsing or error case specs:

```rust
/// Spec: docs/specs/02-config.md#validation
#[test]
fn rejects_invalid_version() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        "version = 999\n"
    ).unwrap();

    check("cloc")
        .in_dir(dir.path())
        .fails()
        .with_error("unsupported config version");
}
```

## Spec Documentation

Every spec references the docs/specs/ section it tests:

```rust
/// Spec: docs/specs/checks/escapes.md#comment-detection
///
/// > For `comment` action, quench searches **upward** for the required comment.
#[test]
fn unsafe_requires_safety_comment() {
    check("escapes")
        .on("violations")
        .fails()
        .with_violation("no_safety_comment.rs");
}
```

## Unimplemented Specs

Specs for unimplemented features use `#[ignore]` with a phase reference:

```rust
/// Spec: docs/specs/checks/escapes.md#comment-detection
#[test]
#[ignore = "TODO: Phase 10 - Escapes Check Actions"]
fn unsafe_allows_safety_comment() {
    check("escapes")
        .on("rust-simple")  // has proper SAFETY comments
        .passes();
}
```

This allows:
- `cargo test --test specs` - runs implemented specs
- `cargo test --test specs -- --ignored` - shows unimplemented count

## Speed Architecture

### Static Fixtures

Fixtures are pre-built, checked-in projects. No compilation during tests.

### Parallel Execution

Specs are independent and run in parallel by default.

### Tiered Execution

Slow specs (those requiring actual builds) are gated:

```rust
#[test]
#[cfg_attr(not(feature = "slow-specs"), ignore = "slow: runs actual build")]
fn build_measures_binary_size() {
    // ...
}
```

- `cargo test --test specs` - fast specs only (< 5s)
- `cargo test --test specs --features slow-specs` - all specs (CI)

## Fixture Design

Each fixture is minimal while exercising specific features:

| Fixture | Purpose |
|---------|---------|
| `minimal` | Empty project, no config - tests defaults |
| `rust-simple` | Single package with src/ and tests/ |
| `rust-workspace` | Multi-package workspace |
| `shell-scripts` | Shell scripts with bats tests |
| `mixed` | Rust + shell combined |
| `violations` | Intentional failures for each check |
| `docs-project` | Markdown files, TOC, links |
| `agents-project` | CLAUDE.md, .cursorrules files |

The `violations` fixture contains subdirectories for each check type, ensuring predictable failure scenarios.

## Implementation

### Module Structure

```rust
// tests/specs/mod.rs (harness entry point)
mod prelude;
mod helpers;
mod builders;

pub use prelude::*;

mod cli;
mod config;
mod checks;
mod adapters;
mod output;
mod modes;
```

### Prelude

```rust
// tests/specs/prelude.rs
pub use crate::helpers::{fixture, quench};
pub use crate::builders::{check, CheckBuilder};
pub use assert_cmd::prelude::*;
pub use predicates::prelude::*;
pub use tempfile::tempdir;
```

### Helpers

```rust
// tests/specs/helpers.rs
use assert_cmd::Command;
use std::path::PathBuf;
use std::sync::LazyLock;

static FIXTURES: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
});

pub fn fixture(name: &str) -> PathBuf {
    let path = FIXTURES.join(name);
    assert!(path.exists(), "Fixture not found: {}", path.display());
    path
}

pub fn quench() -> Command {
    Command::cargo_bin("quench").unwrap()
}
```

### CheckBuilder Implementation

```rust
// tests/specs/builders.rs
use crate::helpers::{fixture, quench};
use assert_cmd::assert::Assert;
use assert_cmd::Command;
use std::path::PathBuf;

pub fn check(name: &str) -> CheckBuilder {
    CheckBuilder::new(name)
}

pub struct CheckBuilder {
    check_name: String,
    dir: Option<PathBuf>,
    args: Vec<String>,
    json: bool,
}

impl CheckBuilder {
    fn new(name: &str) -> Self {
        Self {
            check_name: name.to_string(),
            dir: None,
            args: Vec::new(),
            json: false,
        }
    }

    pub fn on(mut self, fixture_name: &str) -> Self {
        self.dir = Some(fixture(fixture_name));
        self
    }

    pub fn in_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.dir = Some(path.into());
        self
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    pub fn json(mut self) -> Self {
        self.json = true;
        self
    }

    pub fn command(self) -> Command {
        let mut cmd = quench();
        cmd.arg("check");
        cmd.arg(format!("--{}", self.check_name));

        if self.json {
            cmd.args(["-o", "json"]);
        }

        cmd.args(&self.args);

        if let Some(dir) = self.dir {
            cmd.current_dir(dir);
        }

        cmd
    }

    pub fn passes(self) -> AssertResult {
        let assert = self.command().assert().success();
        AssertResult { assert }
    }

    pub fn fails(self) -> FailureAssert {
        let assert = self.command().assert().failure();
        FailureAssert { assert }
    }
}

pub struct AssertResult {
    assert: Assert,
}

impl AssertResult {
    pub fn json<F>(self, f: F)
    where
        F: FnOnce(&serde_json::Value),
    {
        let output = self.assert.get_output();
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap();
        f(&value);
    }

    pub fn snapshot(self) {
        let output = self.assert.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        insta::assert_snapshot!(stdout);
    }
}

pub struct FailureAssert {
    assert: Assert,
}

impl FailureAssert {
    pub fn with_violation(self, file: &str) -> Self {
        Self {
            assert: self.assert.stdout(predicates::str::contains(file)),
        }
    }

    pub fn with_output(self, text: &str) -> Self {
        Self {
            assert: self.assert.stdout(predicates::str::contains(text)),
        }
    }

    pub fn with_error(self, text: &str) -> Self {
        Self {
            assert: self.assert.stderr(predicates::str::contains(text)),
        }
    }

    pub fn assert(self) -> Assert {
        self.assert
    }

    pub fn snapshot(self) {
        let output = self.assert.get_output();
        let stdout = String::from_utf8_lossy(&output.stdout);
        insta::assert_snapshot!(stdout);
    }
}
```

## CI Integration

```yaml
jobs:
  specs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Fast specs
        run: cargo test --test specs

      - name: Slow specs
        run: cargo test --test specs --features slow-specs

      - name: Check snapshots
        run: cargo insta test --check
```
