# Phase 001: Project Foundation - Setup

## Overview

Establish the project foundation for quench: a fast quality linting CLI for AI agents. This phase sets up the Cargo workspace, error handling, and comprehensive test infrastructure including unit tests, integration tests, snapshot tests, and benchmarks.

**Current State**: The workspace skeleton exists with `crates/cli`, basic lint configuration, and some dev-dependencies. The main binary is a stub.

**End State**: Complete test infrastructure ready for TDD development of CLI features.

## Project Structure

```
quench/
├── Cargo.toml                    # Workspace (exists)
├── crates/
│   └── cli/
│       ├── Cargo.toml            # Binary crate (update dependencies)
│       ├── src/
│       │   ├── main.rs           # Entry point
│       │   ├── lib.rs            # Library exports for tests
│       │   ├── error.rs          # Error types
│       │   └── error_tests.rs    # Error unit tests
│       └── benches/
│           └── baseline.rs       # Benchmark skeleton
├── tests/
│   ├── specs/
│   │   ├── CLAUDE.md             # Spec conventions (exists)
│   │   ├── mod.rs                # Spec test module
│   │   └── prelude.rs            # Test helpers
│   └── fixtures/                 # (created in Phase 010)
└── scripts/
    └── bootstrap                 # Quality checks (exists)
```

## Dependencies

### Runtime Dependencies (crates/cli/Cargo.toml)

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### Dev Dependencies (crates/cli/Cargo.toml)

```toml
[dev-dependencies]
assert_cmd = "2"           # (exists) CLI testing
insta = { version = "1", features = ["filters", "redactions"] }  # (update) Snapshots
predicates = "3"           # (exists) Assertions
serde_json = "1"           # (exists) JSON testing
tempfile = "3"             # (exists) Temp dirs
yare = "3"                 # Parameterized tests
proptest = "1"             # Property-based testing
criterion = { version = "0.5", features = ["html_reports"] }
```

### Workspace Configuration

```toml
# Cargo.toml (root) - add benchmark configuration
[[bench]]
name = "baseline"
harness = false
path = "crates/cli/benches/baseline.rs"
```

## Implementation Phases

### Phase 1.1: Dependency Setup

**Goal**: Add all required dependencies and verify they compile.

**Tasks**:
1. Update `crates/cli/Cargo.toml` with runtime dependencies
2. Update `crates/cli/Cargo.toml` with dev dependencies
3. Add benchmark configuration to root `Cargo.toml`
4. Run `cargo check --all` to verify dependencies resolve
5. Run `cargo deny check` to verify license compliance

**Verification**:
```bash
cargo check --all
cargo deny check
```

### Phase 1.2: Error Types

**Goal**: Establish error handling patterns with `thiserror` and `anyhow`.

**Tasks**:
1. Create `crates/cli/src/lib.rs` exposing modules
2. Create `crates/cli/src/error.rs` with error types
3. Create `crates/cli/src/error_tests.rs` with unit tests
4. Update `main.rs` to use error handling

**Files**:

`crates/cli/src/lib.rs`:
```rust
pub mod error;

pub use error::{Error, Result};
```

`crates/cli/src/error.rs`:
```rust
use std::path::PathBuf;

/// Quench error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration file not found or invalid
    #[error("config error: {message}")]
    Config { message: String, path: Option<PathBuf> },

    /// Invalid command-line arguments
    #[error("argument error: {0}")]
    Argument(String),

    /// File I/O error
    #[error("io error: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Internal error (bug)
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type using quench Error
pub type Result<T> = std::result::Result<T, Error>;

/// Exit codes per CLI spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// All checks passed
    Success = 0,
    /// One or more checks failed
    CheckFailed = 1,
    /// Configuration or argument error
    ConfigError = 2,
    /// Internal error
    InternalError = 3,
}

impl From<&Error> for ExitCode {
    fn from(err: &Error) -> Self {
        match err {
            Error::Config { .. } | Error::Argument(_) => ExitCode::ConfigError,
            Error::Io { .. } => ExitCode::InternalError,
            Error::Internal(_) => ExitCode::InternalError,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
```

`crates/cli/src/error_tests.rs`:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn config_error_display() {
    let err = Error::Config {
        message: "invalid version".into(),
        path: Some(PathBuf::from("quench.toml")),
    };
    assert!(err.to_string().contains("invalid version"));
}

#[test]
fn exit_code_from_config_error() {
    let err = Error::Config {
        message: "test".into(),
        path: None,
    };
    assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);
}

#[test]
fn exit_code_from_argument_error() {
    let err = Error::Argument("unknown flag".into());
    assert_eq!(ExitCode::from(&err), ExitCode::ConfigError);
}

#[test]
fn exit_code_from_internal_error() {
    let err = Error::Internal("bug".into());
    assert_eq!(ExitCode::from(&err), ExitCode::InternalError);
}
```

**Verification**:
```bash
cargo test -p quench error
```

### Phase 1.3: Unit Test Setup

**Goal**: Establish unit test patterns with yare for parameterized tests.

**Tasks**:
1. Verify yare dependency works with a sample test
2. Verify proptest dependency works with a sample test
3. Document test patterns in code comments

**Add to `crates/cli/src/error_tests.rs`** (demonstrating patterns):
```rust
use yare::parameterized;

#[parameterized(
    config = { Error::Config { message: "x".into(), path: None }, ExitCode::ConfigError },
    argument = { Error::Argument("x".into()), ExitCode::ConfigError },
    internal = { Error::Internal("x".into()), ExitCode::InternalError },
)]
fn exit_code_mapping(err: Error, expected: ExitCode) {
    assert_eq!(ExitCode::from(&err), expected);
}
```

**Verification**:
```bash
cargo test -p quench
```

### Phase 1.4: Integration Test Harness

**Goal**: Set up CLI testing infrastructure with assert_cmd.

**Tasks**:
1. Create `tests/specs/mod.rs` as the integration test entry point
2. Create `tests/specs/prelude.rs` with test helpers per `tests/specs/CLAUDE.md`
3. Add initial smoke test (ignored until Phase 005)

**Files**:

`tests/specs/mod.rs`:
```rust
//! Behavioral specifications for quench CLI.
//!
//! These tests are black-box: they invoke the CLI binary and verify
//! stdout, stderr, and exit codes. See CLAUDE.md for conventions.

mod prelude;

use prelude::*;

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 0 when invoked with --help
#[test]
fn help_exits_successfully() {
    quench()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("quench"));
}

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 0 when invoked with --version
#[test]
fn version_exits_successfully() {
    quench()
        .arg("--version")
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench check runs quality checks
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_command_exists() {
    quench()
        .arg("check")
        .assert()
        .success();
}
```

`tests/specs/prelude.rs`:
```rust
//! Test helpers for behavioral specifications.
//!
//! Provides high-level DSL for testing quench CLI behavior.

pub use assert_cmd::prelude::*;
pub use predicates::prelude::*;
use std::process::Command;

/// Returns a Command configured to run the quench binary
pub fn quench() -> Command {
    Command::cargo_bin("quench").expect("quench binary should exist")
}

/// High-level check builder (expanded in later phases)
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub struct CheckBuilder {
    check_name: String,
    fixture: Option<String>,
    json: bool,
}

#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
impl CheckBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            check_name: name.to_string(),
            fixture: None,
            json: false,
        }
    }

    pub fn on(mut self, fixture: &str) -> Self {
        self.fixture = Some(fixture.to_string());
        self
    }

    pub fn json(mut self) -> Self {
        self.json = true;
        self
    }
}

/// Create a check builder for the named check
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub fn check(name: &str) -> CheckBuilder {
    CheckBuilder::new(name)
}

/// Get path to a test fixture directory
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub fn fixture(name: &str) -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set");
    std::path::PathBuf::from(manifest_dir)
        .parent()
        .expect("parent should exist")
        .parent()
        .expect("grandparent should exist")
        .join("tests")
        .join("fixtures")
        .join(name)
}
```

**Verification**:
```bash
cargo test --test specs
cargo test --test specs -- --ignored  # Show unimplemented count
```

### Phase 1.5: Snapshot Testing Setup

**Goal**: Configure insta for snapshot testing with appropriate filters.

**Tasks**:
1. Update insta dependency with filters feature
2. Create `.config/insta.yaml` for project-wide settings
3. Add sample snapshot test (ignored until Phase 030)

**Files**:

`.config/insta.yaml`:
```yaml
# Insta snapshot configuration
# https://insta.rs/docs/settings/

# Review mode: require explicit approval
behavior:
  review: true

# Glob patterns for test discovery
test_runner: auto
```

**Add to `tests/specs/mod.rs`**:
```rust
/// Spec: docs/specs/03-output.md#text-output
///
/// > Text output format snapshot
#[test]
#[ignore = "TODO: Phase 030 - Output infrastructure"]
fn check_output_format_snapshot() {
    let output = quench()
        .args(["check", "--cloc"])
        .current_dir(fixture("violations"))
        .output()
        .expect("command should run");

    insta::assert_snapshot!(
        String::from_utf8_lossy(&output.stdout),
        @"" // Inline snapshot, will be filled on first run
    );
}
```

**Verification**:
```bash
cargo insta test --accept  # Run and accept initial snapshots
cargo insta review         # Review any pending changes
```

### Phase 1.6: Benchmarking Setup

**Goal**: Configure criterion for performance benchmarks.

**Tasks**:
1. Create `crates/cli/benches/baseline.rs` benchmark skeleton
2. Add benchmark harness configuration to workspace
3. Verify benchmarks compile and run

**Files**:

`crates/cli/benches/baseline.rs`:
```rust
//! Baseline benchmarks for quench performance tracking.
//!
//! These benchmarks establish performance baselines for:
//! - CLI startup time
//! - File walking (when implemented)
//! - Check execution (when implemented)

use criterion::{criterion_group, criterion_main, Criterion};
use std::process::Command;

/// Benchmark CLI startup time (no-op execution)
fn bench_cli_startup(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    c.bench_function("cli_startup", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("--help")
                .output()
                .expect("quench should run")
        })
    });
}

/// Benchmark version check (minimal work)
fn bench_version_check(c: &mut Criterion) {
    let quench_bin = env!("CARGO_BIN_EXE_quench");

    c.bench_function("version_check", |b| {
        b.iter(|| {
            Command::new(quench_bin)
                .arg("--version")
                .output()
                .expect("quench should run")
        })
    });
}

criterion_group!(benches, bench_cli_startup, bench_version_check);
criterion_main!(benches);
```

**Update root `Cargo.toml`**:
```toml
[[bench]]
name = "baseline"
harness = false
path = "crates/cli/benches/baseline.rs"
```

**Verification**:
```bash
cargo bench --bench baseline
```

## Key Implementation Details

### Error Handling Pattern

Use `thiserror` for defining error types with structured context:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config error: {message}")]
    Config { message: String, path: Option<PathBuf> },
}
```

Use `anyhow` at the application boundary for ergonomic error propagation:

```rust
fn main() -> anyhow::Result<()> {
    // ...
}
```

### Test File Convention

Per CLAUDE.md, use sibling `_tests.rs` files:

```rust
// src/parser.rs
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

```rust
// src/parser_tests.rs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
```

### Exit Code Mapping

Exit codes per CLI spec (`docs/specs/01-cli.md#exit-codes`):

| Code | Meaning | Error Type |
|------|---------|------------|
| 0 | Success | - |
| 1 | Check failed | Check violation |
| 2 | Config/arg error | `Error::Config`, `Error::Argument` |
| 3 | Internal error | `Error::Io`, `Error::Internal` |

## Verification Plan

### Unit Tests
```bash
cargo test -p quench          # All unit tests
cargo test -p quench error    # Error module only
```

### Integration Tests
```bash
cargo test --test specs              # Implemented specs
cargo test --test specs -- --ignored # Show unimplemented count
```

### Snapshots
```bash
cargo insta test              # Run snapshot tests
cargo insta review            # Review changes
```

### Benchmarks
```bash
cargo bench --bench baseline  # Run benchmarks
```

### Full Check
```bash
make check                    # All CI checks
```

### Expected Outcomes

After Phase 001 completion:

1. `cargo test --all` passes with:
   - Error type unit tests passing
   - Help/version integration tests passing
   - Unimplemented specs shown as ignored

2. `cargo bench --bench baseline` runs:
   - CLI startup benchmark executes
   - Version check benchmark executes

3. `make check` passes:
   - fmt, clippy, test, build
   - bootstrap checks (file sizes, conventions)
   - cargo audit, cargo deny

4. Project structure matches specification:
   - `crates/cli/src/lib.rs` exports error module
   - `crates/cli/src/error.rs` has error types
   - `tests/specs/` has prelude and initial specs
   - `crates/cli/benches/baseline.rs` has benchmarks
