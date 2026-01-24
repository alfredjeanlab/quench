# Phase 1545: Per-Language Policy Check Level Specs

**Root Feature:** `quench-eb8d`

## Overview

Write behavioral specifications for per-language policy check levels. Each language adapter (`rust`, `golang`, `javascript`, `shell`) should support an independent `check` field in `[lang.policy]` that controls whether policy violations (e.g., lint config standalone requirement) are errors, warnings, or disabled.

This phase creates test fixtures and behavioral specs only. Implementation is Phase 1547.

## Project Structure

Files to create:
```
tests/
├── fixtures/policy-lang/
│   ├── rust-off/           # [rust.policy] check = "off"
│   │   ├── quench.toml
│   │   ├── Cargo.toml
│   │   ├── rustfmt.toml    # Lint config file (changed)
│   │   └── src/lib.rs      # Source file (changed)
│   ├── rust-warn/          # [rust.policy] check = "warn"
│   │   ├── quench.toml
│   │   ├── Cargo.toml
│   │   ├── rustfmt.toml
│   │   └── src/lib.rs
│   ├── golang-off/         # [golang.policy] check = "off"
│   │   ├── quench.toml
│   │   ├── go.mod
│   │   ├── .golangci.yml
│   │   └── main.go
│   ├── golang-warn/        # [golang.policy] check = "warn"
│   │   ├── quench.toml
│   │   ├── go.mod
│   │   ├── .golangci.yml
│   │   └── main.go
│   ├── javascript-off/     # [javascript.policy] check = "off"
│   │   ├── quench.toml
│   │   ├── package.json
│   │   ├── eslint.config.js
│   │   └── src/index.ts
│   ├── shell-off/          # [shell.policy] check = "off"
│   │   ├── quench.toml
│   │   ├── .shellcheckrc
│   │   └── scripts/deploy.sh
│   ├── mixed-levels/       # Multiple languages with different levels
│   │   ├── quench.toml
│   │   ├── Cargo.toml
│   │   ├── rustfmt.toml
│   │   ├── go.mod
│   │   ├── .golangci.yml
│   │   ├── src/lib.rs
│   │   └── main.go
│   └── inherits/           # Unset inherits from global (future)
│       ├── quench.toml
│       ├── Cargo.toml
│       ├── rustfmt.toml
│       └── src/lib.rs
└── specs/checks/
    └── policy_lang.rs      # Behavioral specs
```

## Dependencies

No new external dependencies required. Uses existing:
- `assert_cmd` for CLI testing
- `serde_json` for JSON output parsing
- `tempfile` for temporary directories

## Implementation Phases

### Phase 1: Create Test Fixtures

Create fixture directories that simulate policy violations (lint config + source files changed together) for each language.

Each fixture needs:
1. `quench.toml` with `[lang.policy]` config
2. Language marker file (`Cargo.toml`, `go.mod`, `package.json`, or `*.sh`)
3. Lint config file for that language
4. Source file for that language

**Fixture: `policy-lang/rust-off/quench.toml`**
```toml
version = 1

[rust.policy]
check = "off"
lint_changes = "standalone"
```

**Fixture: `policy-lang/rust-warn/quench.toml`**
```toml
version = 1

[rust.policy]
check = "warn"
lint_changes = "standalone"
```

**Fixture: `policy-lang/mixed-levels/quench.toml`**
```toml
version = 1

[rust.policy]
check = "error"
lint_changes = "standalone"

[golang.policy]
check = "warn"
lint_changes = "standalone"

[javascript.policy]
check = "off"
lint_changes = "standalone"
```

**Verification**: Fixtures parse without errors with `cargo run -- check --cloc --escapes` (avoiding policy check until implemented).

### Phase 2: Write Rust Policy Specs

Create `tests/specs/checks/policy_lang.rs` with specs for Rust:

```rust
//! Behavioral specs for per-language policy check level.
//!
//! Tests that quench correctly:
//! - Respects {lang}.policy.check = "off" to disable policy for that language
//! - Respects {lang}.policy.check = "warn" to report without failing
//! - Allows independent check levels per language
//!
//! Reference: docs/specs/langs/{rust,golang,javascript,shell}.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// RUST POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/rust.md#policy
///
/// > [rust.policy]
/// > check = "off" disables policy for Rust files
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn rust_policy_check_off_disables_policy() {
    // Fixture has lint config + source changes but rust.policy.check = "off"
    // Should pass even though standalone policy would normally fail
    check("escapes").on("policy-lang/rust-off").passes();
}

/// Spec: docs/specs/langs/rust.md#policy
///
/// > [rust.policy]
/// > check = "warn" reports but doesn't fail
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn rust_policy_check_warn_reports_without_failing() {
    check("escapes")
        .on("policy-lang/rust-warn")
        .passes()
        .stdout_has("lint config changes must be standalone");
}
```

**Verification**: Specs compile with `cargo test --test specs -- --ignored`.

### Phase 3: Write Go, JavaScript, Shell Specs

Add specs for remaining languages following the same pattern:

```rust
// =============================================================================
// GOLANG POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#policy
///
/// > [golang.policy]
/// > check = "off" disables policy for Go files
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn golang_policy_check_off_disables_policy() {
    check("escapes").on("policy-lang/golang-off").passes();
}

/// Spec: docs/specs/langs/golang.md#policy
///
/// > [golang.policy]
/// > check = "warn" reports but doesn't fail
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn golang_policy_check_warn_reports_without_failing() {
    check("escapes")
        .on("policy-lang/golang-warn")
        .passes()
        .stdout_has("lint config changes must be standalone");
}

// =============================================================================
// JAVASCRIPT POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/javascript.md#policy
///
/// > [javascript.policy]
/// > check = "off" disables policy for JS/TS files
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn javascript_policy_check_off_disables_policy() {
    check("escapes").on("policy-lang/javascript-off").passes();
}

// =============================================================================
// SHELL POLICY CONFIG SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#policy
///
/// > [shell.policy]
/// > check = "off" disables policy for shell scripts
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn shell_policy_check_off_disables_policy() {
    check("escapes").on("policy-lang/shell-off").passes();
}
```

**Verification**: All specs compile.

### Phase 4: Write Independent Check Level Specs

Add specs testing that languages can have independent policy check levels:

```rust
// =============================================================================
// INDEPENDENT CHECK LEVEL SPECS
// =============================================================================

/// Spec: docs/specs/10-language-adapters.md
///
/// > Each language can have independent policy check level
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn each_language_can_have_independent_policy_check_level() {
    // Fixture has: rust=error (fails), golang=warn (reports), javascript=off (skipped)
    let result = check("escapes").on("policy-lang/mixed-levels").json().fails();

    // Rust policy violation should cause failure
    let violations = result.require("violations").as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v.get("type")
            .and_then(|t| t.as_str())
            .map(|t| t.contains("lint_config"))
            .unwrap_or(false)
    }));
}

/// Spec: docs/specs/10-language-adapters.md
///
/// > Mixed project: Go policy warns, Rust policy errors
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn mixed_levels_go_warn_rust_error() {
    // When golang.policy.check = "warn" and rust.policy.check = "error"
    // Go violations should be reported but not cause failure
    // Rust violations should cause failure
    check("escapes")
        .on("policy-lang/mixed-levels")
        .fails()
        .stdout_has("lint config changes must be standalone");
}
```

**Verification**: All specs compile and show as ignored.

### Phase 5: Register Spec Module and Final Verification

1. Add module declaration to `tests/specs/checks/mod.rs`:
   ```rust
   mod policy_lang;
   ```

2. Run full verification:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --test specs
   cargo test --test specs -- --ignored 2>&1 | grep "policy"
   ```

3. Verify ignored test count increased appropriately.

**Verification**: `make check` passes, ignored specs listed correctly.

## Key Implementation Details

### Policy Check vs Escapes Check

The policy check is currently part of the escapes check flow (via `check_lint_policy`). The specs use `check("escapes")` because that's where policy violations surface. When Phase 1547 implements the per-language check level, the policy violations will respect `[lang.policy].check`.

### Fixture Design

Each fixture must trigger a policy violation scenario:
- Lint config file present AND would be "changed" (simulated by git diff in actual runs)
- Source file present AND would be "changed"
- `lint_changes = "standalone"` policy enabled

For behavioral tests without actual git, the fixtures represent the "would fail" state.

### Check Level Semantics

Following the pattern from cloc:
- `check = "error"` (default): Policy violations fail the check
- `check = "warn"`: Policy violations are reported but don't fail
- `check = "off"`: Policy check is skipped entirely for that language

### Future: Inheritance from Global

Phase 1545 does not implement inheritance behavior (unset `{lang}.policy.check` inheriting from a global policy check level). This may be added in a future phase if a global `[check.policy]` section is introduced.

## Verification Plan

### Compilation

All specs must compile:
```bash
cargo test --test specs -- --no-run
```

### Ignored Specs

Verify new specs appear as ignored:
```bash
cargo test --test specs -- --ignored 2>&1 | grep "policy_lang"
```

Expected output shows 8-10 ignored tests for policy_lang module.

### Fixture Validity

Each fixture should parse without errors:
```bash
for fixture in tests/fixtures/policy-lang/*/; do
  echo "Checking $fixture"
  cargo run -- check --cloc --no-cache "$fixture" || true
done
```

### Full Suite

Run `make check` to ensure no regressions:
```bash
make check
```

## Spec Summary

| Language | Check = Off | Check = Warn | Independent Levels |
|----------|-------------|--------------|-------------------|
| Rust | `rust_policy_check_off_disables_policy` | `rust_policy_check_warn_reports_without_failing` | ✓ |
| Go | `golang_policy_check_off_disables_policy` | `golang_policy_check_warn_reports_without_failing` | ✓ |
| JavaScript | `javascript_policy_check_off_disables_policy` | (future) | ✓ |
| Shell | `shell_policy_check_off_disables_policy` | (future) | ✓ |
| Mixed | - | - | `each_language_can_have_independent_policy_check_level`, `mixed_levels_go_warn_rust_error` |

All specs marked with `#[ignore = "TODO: Phase 1547 - Per-language policy config"]`.
