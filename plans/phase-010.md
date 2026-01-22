# Phase 010: Test Fixtures - Implementation

**Root Feature:** `quench-057c`

## Overview

Create a comprehensive set of test fixtures for quench behavioral specs. Each fixture is a self-contained, realistic mini-project that tests can run quench against. Fixtures cover the spectrum from minimal empty projects to complex multi-language workspaces with intentional violations of every check type.

**Current State**: No `tests/fixtures/` directory exists. Behavioral specs in `tests/specs.rs` reference fixtures via `fixture("name")` helper but have no fixtures to work with.

**End State**: Eight distinct fixtures covering all project types and check scenarios, plus a README documenting each fixture's purpose.

## Project Structure

```
tests/fixtures/
├── README.md                    # Documentation for all fixtures
├── minimal/                     # Bare project, no config, no source
│   └── .gitkeep
├── rust-simple/                 # Small Rust project
│   ├── quench.toml
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   └── tests/
│       └── lib_tests.rs
├── rust-workspace/              # Multi-package Rust workspace
│   ├── quench.toml
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── core/
│   │   │   ├── Cargo.toml
│   │   │   ├── src/lib.rs
│   │   │   └── src/lib_tests.rs
│   │   └── cli/
│   │       ├── Cargo.toml
│   │       ├── src/main.rs
│   │       └── src/main_tests.rs
│   └── tests/
│       └── integration.rs
├── shell-scripts/               # Shell scripts with bats tests
│   ├── quench.toml
│   ├── scripts/
│   │   ├── build.sh
│   │   └── deploy.sh
│   └── tests/
│       └── scripts.bats
├── mixed/                       # Rust CLI + shell scripts
│   ├── quench.toml
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs
│   ├── scripts/
│   │   └── install.sh
│   └── tests/
│       ├── cli.bats
│       └── unit_tests.rs
├── violations/                  # Intentional violations for each check
│   ├── quench.toml              # Enables all checks
│   ├── Cargo.toml
│   ├── src/
│   │   ├── oversized.rs         # cloc: too many lines
│   │   ├── escapes.rs           # escapes: unwrap, unsafe without SAFETY
│   │   ├── missing_tests.rs     # tests: no corresponding test file
│   │   └── no_license.rs        # license: missing header
│   ├── scripts/
│   │   └── bad.sh               # escapes: shellcheck disable, set +e
│   ├── CLAUDE.md                # agents: has tables, missing sections
│   └── docs/
│       └── specs/
│           ├── CLAUDE.md        # docs: broken TOC
│           └── broken-link.md   # docs: broken markdown link
├── docs-project/                # Full docs structure
│   ├── quench.toml
│   ├── CLAUDE.md
│   ├── docs/
│   │   ├── CLAUDE.md            # Serves as specs index
│   │   └── specs/
│   │       ├── 00-overview.md
│   │       ├── 01-api.md
│   │       └── 02-config.md
│   └── src/
│       └── lib.rs
└── agents-project/              # Agent context files
    ├── quench.toml
    ├── CLAUDE.md                # Root agent file
    ├── .cursorrules             # Cursor config (synced)
    ├── src/
    │   └── lib.rs
    └── crates/
        └── api/
            ├── CLAUDE.md        # Package-level agent file
            └── src/lib.rs
```

## Dependencies

No external dependencies needed. Fixtures are static files.

## Implementation Phases

### Phase 10.1: Minimal Fixture

**Goal**: Create the simplest possible fixture - an empty project with no config.

**Tasks**:
1. Create `tests/fixtures/minimal/` directory
2. Add `.gitkeep` to preserve empty directory
3. No `quench.toml` - tests default behavior

**Files**:

```
tests/fixtures/minimal/
└── .gitkeep
```

**Contents**: Empty file (`.gitkeep` is just a marker)

**Verification**:
```bash
# Quench should work with defaults
cd tests/fixtures/minimal
quench check  # No config, should use defaults
```

### Phase 10.2: Rust Simple Fixture

**Goal**: Create a small, well-structured Rust project that passes all checks.

**Tasks**:
1. Create `quench.toml` with version 1
2. Create minimal `Cargo.toml`
3. Create `src/lib.rs` with simple function
4. Create `tests/lib_tests.rs` with test

**Files**:

```toml
# tests/fixtures/rust-simple/quench.toml
version = 1

[project]
name = "rust-simple"
```

```toml
# tests/fixtures/rust-simple/Cargo.toml
[package]
name = "rust-simple"
version = "0.1.0"
edition = "2021"
```

```rust
// tests/fixtures/rust-simple/src/lib.rs
//! A simple library for testing quench.

/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

```rust
// tests/fixtures/rust-simple/src/lib_tests.rs
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}
```

**Verification**:
```bash
cd tests/fixtures/rust-simple
cargo test           # Tests pass
quench check         # All checks pass
```

### Phase 10.3: Rust Workspace Fixture

**Goal**: Create a multi-package Rust workspace for testing package-level metrics.

**Tasks**:
1. Create workspace `Cargo.toml`
2. Create `crates/core/` with library code
3. Create `crates/cli/` with binary code
4. Add integration tests at workspace level

**Files**:

```toml
# tests/fixtures/rust-workspace/quench.toml
version = 1

[project]
name = "rust-workspace"
packages = ["crates/core", "crates/cli"]
```

```toml
# tests/fixtures/rust-workspace/Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"
```

```toml
# tests/fixtures/rust-workspace/crates/core/Cargo.toml
[package]
name = "workspace-core"
version = "0.1.0"
edition = "2021"
```

```rust
// tests/fixtures/rust-workspace/crates/core/src/lib.rs
//! Core library functionality.

pub fn process(input: &str) -> String {
    input.to_uppercase()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

```rust
// tests/fixtures/rust-workspace/crates/core/src/lib_tests.rs
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn test_process() {
    assert_eq!(process("hello"), "HELLO");
}
```

```toml
# tests/fixtures/rust-workspace/crates/cli/Cargo.toml
[package]
name = "workspace-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
workspace-core = { path = "../core" }
```

```rust
// tests/fixtures/rust-workspace/crates/cli/src/main.rs
//! CLI entry point.

use workspace_core::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    for arg in args {
        println!("{}", process(&arg));
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
```

```rust
// tests/fixtures/rust-workspace/crates/cli/src/main_tests.rs
#![allow(clippy::unwrap_used)]

#[test]
fn test_placeholder() {
    // CLI tests would typically be integration tests
    assert!(true);
}
```

```rust
// tests/fixtures/rust-workspace/tests/integration.rs
#![allow(clippy::unwrap_used)]

use workspace_core::process;

#[test]
fn test_integration() {
    assert_eq!(process("test"), "TEST");
}
```

**Verification**:
```bash
cd tests/fixtures/rust-workspace
cargo test --all     # All tests pass
quench check         # Package metrics collected
```

### Phase 10.4: Shell Scripts Fixture

**Goal**: Create a shell-only project with bats tests.

**Tasks**:
1. Create `quench.toml` for shell project
2. Create shell scripts in `scripts/`
3. Create bats test file

**Files**:

```toml
# tests/fixtures/shell-scripts/quench.toml
version = 1

[project]
name = "shell-scripts"
```

```bash
#!/bin/bash
# tests/fixtures/shell-scripts/scripts/build.sh
# Build script for the project

set -euo pipefail

echo "Building project..."
echo "Build complete"
```

```bash
#!/bin/bash
# tests/fixtures/shell-scripts/scripts/deploy.sh
# Deploy script for the project

set -euo pipefail

TARGET="${1:-production}"
echo "Deploying to $TARGET..."
echo "Deploy complete"
```

```bash
#!/usr/bin/env bats
# tests/fixtures/shell-scripts/tests/scripts.bats

@test "build script runs successfully" {
    run ./scripts/build.sh
    [ "$status" -eq 0 ]
    [[ "$output" == *"Build complete"* ]]
}

@test "deploy script accepts target argument" {
    run ./scripts/deploy.sh staging
    [ "$status" -eq 0 ]
    [[ "$output" == *"staging"* ]]
}
```

**Verification**:
```bash
cd tests/fixtures/shell-scripts
bats tests/          # Bats tests pass (if bats installed)
quench check         # Shell checks pass
```

### Phase 10.5: Mixed Project Fixture

**Goal**: Create a Rust CLI with shell helper scripts.

**Tasks**:
1. Create combined `quench.toml`
2. Create Rust CLI binary
3. Create install script
4. Create both bats and Rust tests

**Files**:

```toml
# tests/fixtures/mixed/quench.toml
version = 1

[project]
name = "mixed-project"
```

```toml
# tests/fixtures/mixed/Cargo.toml
[package]
name = "mixed-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "mixed"
path = "src/main.rs"
```

```rust
// tests/fixtures/mixed/src/main.rs
//! A CLI tool with shell script helpers.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        println!("Usage: mixed <command>");
        return;
    }
    match args[0].as_str() {
        "hello" => println!("Hello, world!"),
        "version" => println!("mixed 0.1.0"),
        _ => eprintln!("Unknown command: {}", args[0]),
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
```

```rust
// tests/fixtures/mixed/src/main_tests.rs
#![allow(clippy::unwrap_used)]

#[test]
fn test_placeholder() {
    assert!(true);
}
```

```bash
#!/bin/bash
# tests/fixtures/mixed/scripts/install.sh
# Install the mixed CLI tool

set -euo pipefail

cargo build --release
cp target/release/mixed /usr/local/bin/
echo "Installed mixed to /usr/local/bin/"
```

```bash
#!/usr/bin/env bats
# tests/fixtures/mixed/tests/cli.bats

setup() {
    cargo build --quiet 2>/dev/null || true
}

@test "mixed shows usage without args" {
    run cargo run --quiet --
    [ "$status" -eq 0 ]
    [[ "$output" == *"Usage"* ]]
}

@test "mixed hello prints greeting" {
    run cargo run --quiet -- hello
    [ "$status" -eq 0 ]
    [[ "$output" == *"Hello"* ]]
}
```

**Verification**:
```bash
cd tests/fixtures/mixed
cargo test           # Rust tests pass
bats tests/cli.bats  # Bats tests pass
quench check         # All checks pass
```

### Phase 10.6: Violations Fixture

**Goal**: Create a project with intentional violations for every check type.

This is the most important fixture - it tests that quench correctly detects problems.

**Tasks**:
1. Create `quench.toml` enabling all checks with strict settings
2. Create files with specific violations for each check:
   - **cloc**: oversized file (>750 lines)
   - **escapes**: `.unwrap()`, `unsafe` without `// SAFETY:`, `#[allow(...)]` without comment
   - **tests**: source file with no corresponding test
   - **license**: file missing SPDX header
   - **agents**: CLAUDE.md with tables, missing "Landing the Plane" section
   - **docs**: broken TOC paths, broken markdown links
   - **git**: (tested via commit messages, not fixture files)
   - **build**: (tested via CI mode, not fixture files)

**Files**:

```toml
# tests/fixtures/violations/quench.toml
version = 1

[project]
name = "violations"

# Enable all checks with strict settings
[check.cloc]
check = "error"
max_lines = 750
max_lines_test = 1100

[check.escapes]
check = "error"

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"

[check.agents]
check = "error"
required = ["CLAUDE.md"]
tables = "forbid"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before completing work"

[check.docs]
check = "error"

[check.docs.toc]
check = "error"

[check.docs.links]
check = "error"

[check.tests]
check = "error"

[check.tests.commit]
check = "error"

[check.license]
check = "error"
license = "MIT"
copyright = "Test Organization"
```

```toml
# tests/fixtures/violations/Cargo.toml
[package]
name = "violations"
version = "0.1.0"
edition = "2021"
```

```rust
// tests/fixtures/violations/src/oversized.rs
//! This file is intentionally oversized to trigger cloc violations.
//!
//! It contains more than 750 lines to exceed the default max_lines limit.

// Lines 1-100: Module documentation and imports
pub mod oversized {
    //! Oversized module with too many lines.

    use std::collections::HashMap;
    use std::sync::Arc;

    // ... repeat pattern to reach 800+ lines
    pub fn func_001() -> i32 { 1 }
    pub fn func_002() -> i32 { 2 }
    pub fn func_003() -> i32 { 3 }
    pub fn func_004() -> i32 { 4 }
    pub fn func_005() -> i32 { 5 }
    pub fn func_006() -> i32 { 6 }
    pub fn func_007() -> i32 { 7 }
    pub fn func_008() -> i32 { 8 }
    pub fn func_009() -> i32 { 9 }
    pub fn func_010() -> i32 { 10 }
    // ... continue to line 800+
}

// NOTE: This file will be generated during fixture creation
// to have exactly 800 lines
```

```rust
// tests/fixtures/violations/src/escapes.rs
//! File with escape hatch violations.

/// Function using unwrap (forbidden in production code).
pub fn risky_parse(input: &str) -> i32 {
    input.parse().unwrap()  // VIOLATION: .unwrap() forbidden
}

/// Function using expect (also forbidden).
pub fn risky_get(map: &std::collections::HashMap<String, i32>, key: &str) -> i32 {
    *map.get(key).expect("key must exist")  // VIOLATION: .expect() forbidden
}

/// Unsafe block without SAFETY comment.
pub fn unsafe_op(ptr: *const i32) -> i32 {
    unsafe { *ptr }  // VIOLATION: unsafe without // SAFETY: comment
}

/// Proper unsafe with SAFETY comment (should pass).
pub fn safe_unsafe_op(ptr: *const i32) -> i32 {
    // SAFETY: Caller guarantees ptr is valid and aligned.
    unsafe { *ptr }
}

/// Suppressed lint without justification.
#[allow(dead_code)]  // VIOLATION: #[allow] without comment
fn unused_function() {}

/// Properly justified lint suppression (should pass).
// JUSTIFIED: This function is used via FFI, not detected by Rust analysis.
#[allow(dead_code)]
fn ffi_callback() {}
```

```rust
// tests/fixtures/violations/src/missing_tests.rs
//! Source file with no corresponding test file.
//!
//! This triggers the tests check because there's no
//! tests/missing_tests_tests.rs or similar.

/// A function that should have tests but doesn't.
pub fn untested_logic(x: i32) -> i32 {
    if x > 0 { x * 2 } else { x * -1 }
}

/// Another untested function.
pub fn more_untested_code(s: &str) -> String {
    s.chars().rev().collect()
}
```

```rust
// tests/fixtures/violations/src/no_license.rs
// This file is missing the required license header.
// It should have SPDX-License-Identifier and Copyright lines.

/// A function in a file without license headers.
pub fn unlicensed_function() -> &'static str {
    "This code has no license header"
}
```

```rust
// tests/fixtures/violations/src/lib.rs
//! Library root for violations fixture.

pub mod oversized;
pub mod escapes;
pub mod missing_tests;
pub mod no_license;
```

```bash
#!/bin/bash
# tests/fixtures/violations/scripts/bad.sh
# Script with shell escape hatch violations

# VIOLATION: shellcheck disable without justification
# shellcheck disable=SC2086
echo $UNQUOTED_VAR

# VIOLATION: set +e without OK comment
set +e
risky_command_that_might_fail
set -e

# Proper set +e with comment (should pass)
# OK: We intentionally ignore errors here to collect all results
set +e
optional_command || true
set -e
```

```markdown
<!-- tests/fixtures/violations/CLAUDE.md -->
# Violations Project

This CLAUDE.md has intentional violations.

## Directory Structure

<!-- This is correct -->
```
src/
├── lib.rs
└── escapes.rs
```

## Some Section

Content here.

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Value A  | Value B  | Value C  |
| Value D  | Value E  | Value F  |

VIOLATION: The table above is forbidden in agent files.

VIOLATION: Missing required "Landing the Plane" section.
```

```markdown
<!-- tests/fixtures/violations/docs/specs/CLAUDE.md -->
# Violations Specs

Spec index with broken TOC.

## File Structure

```
docs/specs/
├── 00-overview.md
├── 01-existing.md
└── 99-nonexistent.md    # VIOLATION: This file doesn't exist
```
```

```markdown
<!-- tests/fixtures/violations/docs/specs/broken-link.md -->
# Broken Link Spec

This spec has a broken markdown link.

See [the missing file](../missing-doc.md) for details.

VIOLATION: The link above points to a non-existent file.
```

**Verification**:
```bash
cd tests/fixtures/violations
quench check --cloc     # FAIL: oversized.rs
quench check --escapes  # FAIL: unwrap, unsafe, allow
quench check --tests    # FAIL: missing_tests.rs
quench check --license  # FAIL: no_license.rs (CI only)
quench check --agents   # FAIL: table, missing section
quench check --docs     # FAIL: broken TOC, broken link
```

### Phase 10.7: Docs Project Fixture

**Goal**: Create a project with proper documentation structure.

**Tasks**:
1. Create `docs/specs/` with index and spec files
2. Create proper TOC in index
3. Create markdown links that resolve correctly
4. Create CLAUDE.md with proper sections

**Files**:

```toml
# tests/fixtures/docs-project/quench.toml
version = 1

[project]
name = "docs-project"

[check.docs]
check = "error"
path = "docs/specs"
index = "toc"
```

```markdown
<!-- tests/fixtures/docs-project/CLAUDE.md -->
# Docs Project

A project demonstrating proper documentation structure.

## Directory Structure

```
docs-project/
├── CLAUDE.md
├── docs/
│   ├── CLAUDE.md
│   └── specs/
│       ├── 00-overview.md
│       ├── 01-api.md
│       └── 02-config.md
└── src/
    └── lib.rs
```

## Development

Run `cargo test` to verify everything works.

## Landing the Plane

Before completing work:

- [ ] Run `quench check`
- [ ] Run `cargo test`
- [ ] Update docs if needed
```

```markdown
<!-- tests/fixtures/docs-project/docs/CLAUDE.md -->
# Documentation

This directory contains all project documentation.

## Specs

See [specs/00-overview.md](specs/00-overview.md) for the overview.

## File Structure

```
docs/
├── CLAUDE.md           # This file
└── specs/
    ├── 00-overview.md  # Problem and philosophy
    ├── 01-api.md       # API documentation
    └── 02-config.md    # Configuration guide
```
```

```markdown
<!-- tests/fixtures/docs-project/docs/specs/00-overview.md -->
# Overview

## Purpose

This document describes the project's purpose and philosophy.

## Goals

- Goal 1
- Goal 2
- Goal 3

## Configuration

See [02-config.md](02-config.md) for configuration details.
```

```markdown
<!-- tests/fixtures/docs-project/docs/specs/01-api.md -->
# API Documentation

## Purpose

Documents the public API.

## Endpoints

### GET /health

Returns health status.

### GET /version

Returns version info.
```

```markdown
<!-- tests/fixtures/docs-project/docs/specs/02-config.md -->
# Configuration Guide

## Purpose

Explains how to configure the project.

## Options

- `option_a`: Description
- `option_b`: Description
```

```rust
// tests/fixtures/docs-project/src/lib.rs
//! A library with proper documentation.

/// Returns health status.
pub fn health() -> &'static str {
    "ok"
}

/// Returns version.
pub fn version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health() {
        assert_eq!(health(), "ok");
    }
}
```

**Verification**:
```bash
cd tests/fixtures/docs-project
quench check --docs    # All docs checks pass
```

### Phase 10.8: Agents Project Fixture

**Goal**: Create a project with agent context files at multiple levels.

**Tasks**:
1. Create root CLAUDE.md with required sections
2. Create .cursorrules synced with CLAUDE.md
3. Create package-level CLAUDE.md in crates/api/
4. Ensure all agent checks pass

**Files**:

```toml
# tests/fixtures/agents-project/quench.toml
version = 1

[project]
name = "agents-project"
packages = ["crates/api"]

[check.agents]
check = "error"
files = ["CLAUDE.md", ".cursorrules"]
required = ["CLAUDE.md"]
optional = [".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
tables = "forbid"

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Overview of project layout"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents"

[check.agents.package]
optional = ["CLAUDE.md"]
max_lines = 200
```

```markdown
<!-- tests/fixtures/agents-project/CLAUDE.md -->
# Agents Project

A project demonstrating proper agent context files.

## Directory Structure

```
agents-project/
├── CLAUDE.md
├── .cursorrules
├── src/
│   └── lib.rs
└── crates/
    └── api/
        ├── CLAUDE.md
        └── src/lib.rs
```

## Development

Standard Rust development workflow.

## Landing the Plane

Before completing work:

- [ ] Run `quench check`
- [ ] Run `cargo test`
- [ ] Run `cargo clippy`
```

```markdown
<!-- tests/fixtures/agents-project/.cursorrules -->
# Agents Project

A project demonstrating proper agent context files.

## Directory Structure

```
agents-project/
├── CLAUDE.md
├── .cursorrules
├── src/
│   └── lib.rs
└── crates/
    └── api/
        ├── CLAUDE.md
        └── src/lib.rs
```

## Development

Standard Rust development workflow.

## Landing the Plane

Before completing work:

- [ ] Run `quench check`
- [ ] Run `cargo test`
- [ ] Run `cargo clippy`
```

```markdown
<!-- tests/fixtures/agents-project/crates/api/CLAUDE.md -->
# API Package

Package-specific context for the API crate.

## Purpose

Provides the public API interface.

## Key Files

- `src/lib.rs` - Main API exports
```

```rust
// tests/fixtures/agents-project/src/lib.rs
//! Root library for agents project.

pub fn hello() -> &'static str {
    "Hello from agents-project"
}
```

```toml
# tests/fixtures/agents-project/Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"

[package]
name = "agents-project"
version = "0.1.0"
edition = "2021"
```

```toml
# tests/fixtures/agents-project/crates/api/Cargo.toml
[package]
name = "agents-api"
version = "0.1.0"
edition = "2021"
```

```rust
// tests/fixtures/agents-project/crates/api/src/lib.rs
//! API package library.

/// Get API version.
pub fn api_version() -> &'static str {
    "v1"
}
```

**Verification**:
```bash
cd tests/fixtures/agents-project
quench check --agents  # All agent checks pass
```

### Phase 10.9: Fixture README and Generation Script

**Goal**: Document all fixtures and create oversized file generator.

**Tasks**:
1. Write comprehensive README.md for fixtures
2. Create script to generate the 800-line oversized.rs file

**Files**:

```markdown
<!-- tests/fixtures/README.md -->
# Test Fixtures

Test fixtures for quench behavioral specs. Each fixture is a self-contained mini-project.

## Fixture Index

| Fixture | Description | Primary Checks |
|---------|-------------|----------------|
| `minimal/` | Empty project, no config | Default behavior |
| `rust-simple/` | Small Rust library | cloc, tests |
| `rust-workspace/` | Multi-package workspace | Package metrics |
| `shell-scripts/` | Shell scripts with bats | Shell escapes |
| `mixed/` | Rust CLI + shell scripts | Multi-language |
| `violations/` | Intentional violations | All checks |
| `docs-project/` | Proper docs structure | docs |
| `agents-project/` | Agent context files | agents |

## Usage in Specs

```rust
use crate::prelude::*;

#[test]
fn cloc_passes_on_simple_project() {
    check("cloc").on("rust-simple").passes();
}

#[test]
fn escapes_fails_on_unwrap() {
    check("escapes")
        .on("violations")
        .fails()
        .with_violation("escapes.rs");
}
```

## Fixture Details

### minimal/

Bare project with no configuration. Tests that quench works with defaults and doesn't fail on empty projects.

- No `quench.toml`
- No source files
- Just `.gitkeep` to preserve directory

### rust-simple/

A minimal Rust library that passes all checks. Good baseline for testing default behavior.

- `quench.toml` with version 1
- `src/lib.rs` with simple function
- `src/lib_tests.rs` with unit test
- Under 750 lines (passes cloc)
- Proper test coverage (passes tests)

### rust-workspace/

Multi-package Rust workspace for testing package-level metrics and breakdown.

- Workspace with `crates/core/` and `crates/cli/`
- Integration tests at workspace root
- Package-specific metrics collection

### shell-scripts/

Shell-only project for testing shell-specific checks.

- Shell scripts in `scripts/`
- Bats tests in `tests/`
- No Rust code

### mixed/

Combined Rust and shell project for testing multi-language detection.

- Rust CLI binary
- Shell install script
- Both bats and Rust tests

### violations/

Project with intentional violations for every check type. Essential for testing failure detection.

**Violations included:**

| Check | File | Violation |
|-------|------|-----------|
| cloc | `src/oversized.rs` | 800+ lines (max: 750) |
| escapes | `src/escapes.rs` | `.unwrap()`, `unsafe` without SAFETY |
| escapes | `scripts/bad.sh` | `shellcheck disable`, `set +e` |
| tests | `src/missing_tests.rs` | No corresponding test file |
| license | `src/no_license.rs` | Missing SPDX header |
| agents | `CLAUDE.md` | Table, missing "Landing the Plane" |
| docs | `docs/specs/CLAUDE.md` | Broken TOC path |
| docs | `docs/specs/broken-link.md` | Broken markdown link |

### docs-project/

Project with proper documentation structure for testing docs checks.

- `docs/specs/` with index and spec files
- Proper TOC with valid paths
- Working markdown links between files
- Required sections present

### agents-project/

Project with agent context files at multiple scopes.

- Root `CLAUDE.md` and `.cursorrules` (synced)
- Package-level `crates/api/CLAUDE.md`
- All required sections present
- No tables (forbidden)

## Regenerating Fixtures

Most fixtures are static. The oversized file is generated:

```bash
./scripts/generate-oversized.sh > tests/fixtures/violations/src/oversized.rs
```

## Adding New Fixtures

1. Create directory under `tests/fixtures/`
2. Add minimal `quench.toml` (or none for default behavior test)
3. Add source files appropriate for the test scenario
4. Document in this README
5. Add specs that use the fixture
```

```bash
#!/bin/bash
# scripts/generate-oversized.sh
# Generates an 800-line Rust file for testing cloc violations

cat << 'HEADER'
//! This file is intentionally oversized to trigger cloc violations.
//!
//! It contains more than 750 lines to exceed the default max_lines limit.
//! Generated by scripts/generate-oversized.sh

#![allow(dead_code)]

HEADER

# Generate 790 function definitions (one per line plus overhead = ~800 lines)
for i in $(seq -w 1 790); do
    echo "pub fn func_$i() -> i32 { $i }"
done

echo ""
echo "// End of generated file"
```

**Verification**:
```bash
# Generate oversized file
./scripts/generate-oversized.sh > tests/fixtures/violations/src/oversized.rs
wc -l tests/fixtures/violations/src/oversized.rs  # Should be ~800 lines

# Verify all fixtures are valid
for dir in tests/fixtures/*/; do
    if [ -f "$dir/Cargo.toml" ]; then
        (cd "$dir" && cargo check --quiet)
    fi
done
```

## Key Implementation Details

### Fixture Isolation

Each fixture must be completely self-contained:
- No relative imports to parent directories
- No shared dependencies between fixtures
- Each fixture can run quench independently

### Git Handling

Fixtures are not git repositories themselves. When quench runs on them:
- Config discovery stops at fixture root (no parent quench.toml)
- Git-related checks (commit format) need special handling in tests

### Oversized File Generation

The `violations/src/oversized.rs` file must be generated, not committed:
- Keeps repo small
- Makes line count easily adjustable
- Generated during test setup or CI

Pattern for the generated file:
```rust
pub fn func_001() -> i32 { 1 }
pub fn func_002() -> i32 { 2 }
// ... 790 more functions
```

This creates exactly one meaningful line per function, making line counting predictable.

### Violation Specificity

Each violation in `violations/` should trigger exactly one check:
- `oversized.rs` - only cloc (no escapes or other issues)
- `escapes.rs` - only escapes (proper size, has tests)
- `missing_tests.rs` - only tests (no escapes, proper size)

This allows tests to verify individual checks in isolation.

### Agent File Sync

In `agents-project/`, CLAUDE.md and .cursorrules must be identical:
- Tests sync detection
- Tests `--fix` behavior
- Any difference triggers failure

## Verification Plan

### Phase Completion Checklist

- [ ] `tests/fixtures/minimal/` exists with `.gitkeep`
- [ ] `tests/fixtures/rust-simple/` compiles and tests pass
- [ ] `tests/fixtures/rust-workspace/` compiles and tests pass
- [ ] `tests/fixtures/shell-scripts/` has valid shell scripts
- [ ] `tests/fixtures/mixed/` compiles and has both test types
- [ ] `tests/fixtures/violations/` has all violation types
- [ ] `tests/fixtures/docs-project/` has proper doc structure
- [ ] `tests/fixtures/agents-project/` has synced agent files
- [ ] `tests/fixtures/README.md` documents all fixtures
- [ ] `scripts/generate-oversized.sh` creates 800-line file
- [ ] All Rust fixtures pass `cargo check`
- [ ] Violations fixture triggers expected failures

### File Counts

| Fixture | Files | Lines (approx) |
|---------|-------|----------------|
| minimal | 1 | 0 |
| rust-simple | 4 | 30 |
| rust-workspace | 10 | 80 |
| shell-scripts | 4 | 40 |
| mixed | 7 | 60 |
| violations | 12 | 900+ |
| docs-project | 8 | 100 |
| agents-project | 9 | 80 |
| **Total** | **~55** | **~1300** |

### Running Verification

```bash
# Verify all fixtures exist
ls -la tests/fixtures/

# Verify Rust fixtures compile
for dir in tests/fixtures/{rust-simple,rust-workspace,mixed,violations,docs-project,agents-project}; do
    echo "Checking $dir..."
    (cd "$dir" && cargo check --quiet 2>/dev/null && echo "  OK" || echo "  SKIP (no Cargo.toml)")
done

# Verify shell scripts are valid
shellcheck tests/fixtures/*/scripts/*.sh 2>/dev/null || echo "shellcheck not installed"

# Generate and verify oversized file
./scripts/generate-oversized.sh > tests/fixtures/violations/src/oversized.rs
lines=$(wc -l < tests/fixtures/violations/src/oversized.rs)
echo "Generated oversized.rs: $lines lines"
[ "$lines" -gt 750 ] && echo "OK: exceeds 750 line limit" || echo "FAIL: needs more lines"
```

## Summary

Phase 010 creates the test fixture infrastructure:

1. **8 fixtures** covering all project types and check scenarios
2. **violations/** fixture with examples for every check
3. **README** documenting fixture purposes and usage
4. **Generation script** for oversized file

These fixtures enable behavioral specs to test quench as a black box, verifying that checks correctly detect passing and failing conditions across diverse project types.
