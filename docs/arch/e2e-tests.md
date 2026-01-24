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
└── specs/                 # Behavioral specifications
    ├── prelude.rs         # Harness, helpers, re-exports
    ├── cli/               # CLI behavior (flags, toggles)
    ├── config/            # Configuration parsing
    ├── checks/            # Per-check specs (cloc, escapes, etc.)
    ├── adapters/          # Language adapters
    ├── output/            # Output formats
    └── modes/             # Operating modes (cache, file walking)
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
fn quench_cmd() -> Command;

/// Single-check builder (high-level, preferred)
fn check(name: &str) -> CheckBuilder<Text, Single>;

/// All-checks builder
fn cli() -> CheckBuilder<Text, All>;

/// Create temp project with quench.toml and CLAUDE.md defaults
fn default_project() -> Project;
```

### CheckBuilder

The `CheckBuilder` provides a fluent interface for the common case: running a single check against a fixture and asserting the result.

```rust
/// Spec: docs/specs/checks/cloc.md#counting-rules
#[test]
fn counts_non_blank_lines() {
    check("cloc").on("rust-simple").passes();
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
#[test]
fn fails_on_oversized_file() {
    check("cloc").on("violations").fails().stdout_has("oversized.rs");
}
```

### Builder Methods

The builder uses typestate for both output mode (Text/Json) and scope (Single/All).

```rust
// Common methods for all builders
impl<Mode, Scope> CheckBuilder<Mode, Scope> {
    fn on(self, fixture: &str) -> Self;
    fn pwd(self, path: impl AsRef<Path>) -> Self;
    fn args(self, args: &[&str]) -> Self;
    fn env(self, key: &str, value: &str) -> Self;
}

// Text mode -> RunAssert
impl CheckBuilder<Text, Single> {
    fn json(self) -> CheckBuilder<Json, Single>;
    fn passes(self) -> RunAssert;
    fn fails(self) -> RunAssert;
    fn exits(self, code: i32) -> RunAssert;
}

impl CheckBuilder<Text, All> {
    fn json(self) -> CheckBuilder<Json, All>;
    fn passes(self) -> RunAssert;
    fn fails(self) -> RunAssert;
    fn exits(self, code: i32) -> RunAssert;
}

// JSON + Single -> CheckJson
impl CheckBuilder<Json, Single> {
    fn passes(self) -> CheckJson;
    fn fails(self) -> CheckJson;
}

// JSON + All -> ChecksJson
impl CheckBuilder<Json, All> {
    fn passes(self) -> ChecksJson;
    fn fails(self) -> ChecksJson;
}

// Single check JSON wrapper
impl CheckJson {
    fn value(&self) -> &Value;      // root JSON
    fn check(&self) -> &Value;      // the check object
    fn get(&self, key: &str) -> Option<&Value>;
    fn require(&self, key: &str) -> &Value;
}

// All checks JSON wrapper
impl ChecksJson {
    fn value(&self) -> &Value;      // root JSON
    fn checks(&self) -> &Vec<Value>; // all check objects
}

// Run result for chaining assertions
impl RunAssert {
    fn stdout(&self) -> String;
    fn stderr(&self) -> String;
    fn stdout_has(self, pred: impl IntoStrPredicate) -> Self;
    fn stdout_lacks(self, pred: impl IntoStrPredicate) -> Self;
    fn stderr_has(self, pred: impl IntoStrPredicate) -> Self;
    fn stderr_lacks(self, pred: impl IntoStrPredicate) -> Self;
    fn stdout_eq(self, expected: &str) -> Self;
    fn stderr_eq(self, expected: &str) -> Self;
}
```

### JSON Assertions

For single-check specs, `.json()` returns `CheckJson`:

```rust
/// Spec: docs/specs/checks/cloc.md#json-output
#[test]
fn json_includes_ratio() {
    let cloc = check("cloc").on("cloc/basic").json().passes();
    let metrics = cloc.require("metrics");
    assert!(metrics.get("ratio").is_some());
}

/// Spec: docs/specs/checks/cloc.md#file-size-limits
#[test]
fn violations_have_file_path() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v.get("file").and_then(|f| f.as_str()).unwrap().ends_with("big.rs")
    }));
}
```

### All-Checks Mode

For all-checks specs, `.json()` returns `ChecksJson`:

```rust
/// Spec: docs/specs/03-output.md#exit-codes
#[test]
fn exit_code_0_all_checks_pass() {
    let dir = temp_project();
    cli().pwd(dir.path()).args(&["--no-git"]).passes();
}

/// Spec: docs/specs/03-output.md#json-format
#[test]
fn json_has_all_checks() {
    let result = cli().on("output-test").json().fails();
    assert!(result.checks().len() > 0);
}
```

### Exact Match Testing

For verifying exact output format, use `assert_eq!` with raw strings.
This ensures any output change requires explicit test update (no auto-accept):

```rust
/// Spec: docs/specs/03-output.md#text-format
#[test]
fn cloc_text_output_format() {
    assert_eq!(
        check("cloc").on("violations").fails().stdout(),
        r#"cloc: FAIL
  src/oversized.rs: file_too_large (14 vs 10)
    Split into smaller modules.
7 checks passed, 1 failed
"#,
        "output format must match exactly"
    );
}
```

Prefer exact matching over snapshots - it prevents regressions from slipping through.

### Multi-Check JSON Specs

For specs testing multiple checks with JSON output:

```rust
/// Spec: docs/specs/01-cli.md#check-selection
#[test]
fn enable_flag_runs_only_that_check() {
    let dir = temp_project();
    let result = cli().pwd(dir.path()).args(&["--cloc"]).json().passes();
    assert_eq!(result.checks().len(), 1);
    assert_eq!(result.checks()[0]["name"], "cloc");
}
```

### Temporary Directories

For config parsing or error case specs:

```rust
/// Spec: docs/specs/02-config.md#validation
#[test]
fn rejects_invalid_version() {
    let temp =default_project();
    temp.config("version = 999");

    check("cloc")
        .pwd(temp.path())
        .fails()
        .with_error("unsupported config version");
}
```

Project methods:
- `Project::empty()` - No default files
- `Project::with_defaults()` - Has quench.toml + CLAUDE.md
- `temp.config(content)` - Write quench.toml (auto-adds `version = 1` if missing)
- `temp.file(path, content)` - Write file (auto-creates parent dirs)
- `temp.path()` - Get the temp directory path

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

## CI Integration

```yaml
jobs:
  specs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Specs
        run: cargo test --test specs
```
