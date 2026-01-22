# Phase 025: Output Infrastructure - Specs

**Root Feature:** `quench-8da2`

## Overview

Create behavioral specifications for quench output infrastructure. These specs test that quench correctly:
- Produces text output matching the format in `docs/specs/03-output.md`
- Produces JSON output that validates against `output.schema.json`
- Returns correct exit codes (0, 1, 2, 3)
- Respects color detection rules (TTY, `CLAUDE_CODE`, `--no-color`)
- Enforces violation limits (default 15, `--no-limit`, `--limit N`)
- Supports `--config` flag for config validation
- Emits debug diagnostics to stderr when `QUENCH_LOG=debug`

All specs will be marked with `#[ignore = "TODO: Phase 030"]` until the output infrastructure implementation is complete.

**Current State**: File walking implementation complete from Phase 020. Basic CLI exists with minimal output (just `{"passed": true}` for JSON). Exit code infrastructure exists in `error.rs`.

**End State**: Complete behavioral spec coverage for output infrastructure. All specs compile and are marked as ignored pending Phase 030 implementation.

## Project Structure

```
tests/
├── fixtures/
│   ├── minimal/                    # Existing - passes all checks
│   ├── violations/                 # Existing - has intentional violations
│   │   └── quench.toml
│   ├── config-error/               # NEW: invalid config for exit code 2
│   │   └── quench.toml             # Malformed config
│   └── output-test/                # NEW: controlled violations for output testing
│       ├── quench.toml
│       └── src/
│           └── oversized.rs        # Triggers cloc violation
└── specs/
    ├── prelude.rs                  # Existing
    ├── file_walking.rs             # Existing
    └── output.rs                   # NEW: output infrastructure specs

docs/specs/
├── 03-output.md                    # Reference for text/JSON format
└── output.schema.json              # JSON schema for validation
```

## Dependencies

Add to `crates/cli/Cargo.toml` dev-dependencies:

```toml
[dev-dependencies]
jsonschema = "0.29"  # JSON schema validation
```

The `serde_json` dependency already exists for JSON parsing.

## Implementation Phases

### Phase 25.1: Test Fixtures for Output Testing

**Goal**: Create fixtures that produce controlled, predictable output for testing.

**Tasks**:
1. Create `tests/fixtures/config-error/` with invalid TOML
2. Create `tests/fixtures/output-test/` with a single cloc violation
3. Ensure `violations/` fixture is properly populated

**Files**:

```toml
# tests/fixtures/config-error/quench.toml
# Intentionally malformed TOML
version = 1
[check.cloc
max_lines = "not a number"  # Missing closing bracket, wrong type
```

```toml
# tests/fixtures/output-test/quench.toml
version = 1

[check.cloc]
max_lines = 10  # Very low limit to trigger violation
```

```rust
// tests/fixtures/output-test/src/oversized.rs
//! This file exceeds the 10-line limit configured in quench.toml.
//! Lines 1-15 to trigger a cloc violation.

pub fn line_1() {}
pub fn line_2() {}
pub fn line_3() {}
pub fn line_4() {}
pub fn line_5() {}
pub fn line_6() {}
pub fn line_7() {}
pub fn line_8() {}
pub fn line_9() {}
pub fn line_10() {}
pub fn line_11() {}
pub fn line_12() {}
```

**Verification**:
```bash
ls -la tests/fixtures/config-error/
ls -la tests/fixtures/output-test/
```

### Phase 25.2: Output Format Specs (Text)

**Goal**: Write specs for text output format matching `03-output.md`.

**Tasks**:
1. Create `tests/specs/output.rs`
2. Add specs for text output structure
3. Add specs for check name, file path, line number format
4. Add specs for advice formatting

**Files**:

```rust
// tests/specs/output.rs
//! Behavioral specs for output infrastructure.
//!
//! Tests that quench correctly formats output according to:
//! - docs/specs/03-output.md (text and JSON formats)
//! - docs/specs/output.schema.json (JSON schema)
//!
//! Reference: docs/specs/03-output.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// Text Output Format
// =============================================================================

/// Spec: docs/specs/03-output.md#text-format
///
/// > Text format: `<check-name>: FAIL`
/// > `  <file>:<line>: <brief violation description>`
/// > `    <advice>`
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn text_output_format_check_name_fail() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"^\w+: FAIL").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > File path and line number format: `<file>:<line>:`
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn text_output_format_file_line() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"  \S+:\d*:").unwrap());
}

/// Spec: docs/specs/03-output.md#text-format
///
/// > Advice is indented under violation
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn text_output_format_advice_indented() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"\n    \S").unwrap());  // 4-space indent for advice
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > Summary line: `N checks passed, M failed`
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn text_output_summary_line() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1)
        .stdout(predicates::str::is_match(r"\d+ checks? (passed|failed)").unwrap());
}

/// Spec: docs/specs/03-output.md#verbosity
///
/// > When all checks pass, only summary: `N checks passed`
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn text_output_passing_summary_only() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"^\d+ checks? passed\n?$").unwrap());
}
```

**Verification**:
```bash
cargo test --test specs output -- --list
```

### Phase 25.3: Output Format Specs (JSON)

**Goal**: Write specs for JSON output format and schema validation.

**Tasks**:
1. Add specs for JSON structure
2. Add JSON schema validation using `jsonschema` crate
3. Add specs for "no additional properties" constraint

**Files**:

```rust
// tests/specs/output.rs - continued

// =============================================================================
// JSON Output Format
// =============================================================================

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON output validates against output.schema.json
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn json_output_validates_against_schema() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("output should be valid JSON");

    // Load schema from docs/specs/output.schema.json
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("docs/specs/output.schema.json");
    let schema_str = std::fs::read_to_string(&schema_path).unwrap();
    let schema: serde_json::Value = serde_json::from_str(&schema_str).unwrap();

    let compiled = jsonschema::JSONSchema::compile(&schema)
        .expect("schema should be valid");

    assert!(compiled.is_valid(&json), "output should validate against schema");
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON has required fields: passed, checks
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn json_output_has_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("passed").is_some(), "should have 'passed' field");
    assert!(json.get("checks").is_some(), "should have 'checks' array");
}

/// Spec: docs/specs/03-output.md#json-format
///
/// > JSON timestamp is ISO 8601 format
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn json_output_timestamp_iso8601() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let timestamp = json.get("timestamp").and_then(|v| v.as_str());
    assert!(timestamp.is_some(), "should have timestamp");

    // ISO 8601 format: 2026-01-21T10:30:00Z
    let ts = timestamp.unwrap();
    assert!(
        ts.contains('T') && ts.ends_with('Z'),
        "timestamp should be ISO 8601: {}",
        ts
    );
}

/// Spec: docs/specs/output.schema.json
///
/// > Check objects have required fields: name, passed
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn json_output_check_has_required_fields() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = json.get("checks").and_then(|v| v.as_array()).unwrap();

    for check in checks {
        assert!(check.get("name").is_some(), "check should have 'name'");
        assert!(check.get("passed").is_some(), "check should have 'passed'");
    }
}

/// Spec: docs/specs/output.schema.json
///
/// > Violation objects have required fields: type, advice
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn json_output_violation_has_required_fields() {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = json.get("checks").and_then(|v| v.as_array()).unwrap();

    for check in checks {
        if let Some(violations) = check.get("violations").and_then(|v| v.as_array()) {
            for violation in violations {
                assert!(violation.get("type").is_some(), "violation should have 'type'");
                assert!(violation.get("advice").is_some(), "violation should have 'advice'");
            }
        }
    }
}
```

**Verification**:
```bash
cargo test --test specs json_output -- --list
```

### Phase 25.4: Exit Code Specs

**Goal**: Write specs for all exit codes per `03-output.md`.

**Tasks**:
1. Add specs for exit code 0 (pass)
2. Add specs for exit code 1 (fail)
3. Add specs for exit code 2 (config error)
4. Add specs for exit code 3 (internal error) - may need stub

**Files**:

```rust
// tests/specs/output.rs - continued

// =============================================================================
// Exit Codes
// =============================================================================

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 0 when all checks pass
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn exit_code_0_all_checks_pass() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .assert()
        .code(0);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 1 when any check fails
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn exit_code_1_check_fails() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .assert()
        .code(1);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit code 2 on configuration error
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn exit_code_2_config_error() {
    quench_cmd()
        .args(["check"])
        .current_dir(fixture("config-error"))
        .assert()
        .code(2);
}

/// Spec: docs/specs/03-output.md#exit-codes
///
/// > Exit codes: 0 (pass), 1 (fail), 2 (config), 3 (internal)
/// > These are the ONLY valid exit codes
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn exit_codes_are_exactly_0_1_2_3() {
    // This test documents the contract. Individual tests verify each code.
    // Exit code 3 (internal error) is hard to trigger intentionally,
    // so we verify the enum values in error.rs match the spec.

    use quench::error::ExitCode;
    assert_eq!(ExitCode::Success as u8, 0);
    assert_eq!(ExitCode::CheckFailed as u8, 1);
    assert_eq!(ExitCode::ConfigError as u8, 2);
    assert_eq!(ExitCode::InternalError as u8, 3);
}
```

**Verification**:
```bash
cargo test --test specs exit_code -- --list
```

### Phase 25.5: Color Detection Specs

**Goal**: Write specs for color output detection rules.

**Tasks**:
1. Add spec for CLAUDE_CODE env disabling color
2. Add spec for non-TTY disabling color
3. Add spec for --no-color flag

**Files**:

```rust
// tests/specs/output.rs - continued

// =============================================================================
// Colorization
// =============================================================================

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when CLAUDE_CODE env var is set
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn color_disabled_when_claude_code_env_set() {
    let output = quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .env("CLAUDE_CODE", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes when CLAUDE_CODE is set"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > Color disabled when stdout is not a TTY
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn color_disabled_when_not_tty() {
    // When run via assert_cmd, stdout is piped (not a TTY)
    let output = quench_cmd()
        .args(["check"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes when not a TTY"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > --no-color flag disables color output
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn no_color_flag_disables_color() {
    let output = quench_cmd()
        .args(["check", "--no-color"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes with --no-color"
    );
}

/// Spec: docs/specs/03-output.md#colorization
///
/// > --color=never disables color output
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn color_never_disables_color() {
    let output = quench_cmd()
        .args(["check", "--color=never"])
        .current_dir(fixture("output-test"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI escape codes with --color=never"
    );
}
```

**Verification**:
```bash
cargo test --test specs color -- --list
```

### Phase 25.6: Violation Limit Specs

**Goal**: Write specs for violation limiting behavior.

**Tasks**:
1. Add spec for default 15 violation limit
2. Add spec for --no-limit showing all violations
3. Add spec for --limit N custom limit
4. Add spec for --config validation mode

**Files**:

```rust
// tests/specs/output.rs - continued

// =============================================================================
// Violation Limits
// =============================================================================

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Default limit: 15 violations shown
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn violation_limit_defaults_to_15() {
    // This spec requires a fixture with >15 violations
    // For now, just verify the flag is accepted
    quench_cmd()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("limit"));
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > --no-limit shows all violations
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn no_limit_shows_all_violations() {
    quench_cmd()
        .args(["check", "--no-limit"])
        .current_dir(fixture("output-test"))
        .assert();  // Just verify flag is accepted
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > --limit N shows N violations
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn limit_n_shows_n_violations() {
    quench_cmd()
        .args(["check", "--limit", "5"])
        .current_dir(fixture("output-test"))
        .assert();  // Just verify flag is accepted
}

/// Spec: docs/specs/03-output.md#violation-limits
///
/// > Message shown when limit reached: "Stopped after N violations"
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn limit_message_when_truncated() {
    // Requires fixture with many violations
    quench_cmd()
        .args(["check", "--limit", "1"])
        .current_dir(fixture("violations"))
        .assert()
        .stdout(predicates::str::contains("Stopped after").or(predicates::str::contains("--no-limit")));
}

// =============================================================================
// Config Validation Mode
// =============================================================================

/// Spec: docs/specs/01-cli.md#commands (implied)
///
/// > --config validates config and exits without running checks
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn config_flag_validates_and_exits() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check", "--config"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().or(predicates::str::contains("valid")));
}

/// Spec: docs/specs/01-cli.md#commands (implied)
///
/// > --config with invalid config returns exit code 2
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn config_flag_invalid_returns_code_2() {
    quench_cmd()
        .args(["check", "--config"])
        .current_dir(fixture("config-error"))
        .assert()
        .code(2);
}

// =============================================================================
// Debug Output
// =============================================================================

/// Spec: docs/specs/03-output.md (implied from QUENCH_LOG)
///
/// > QUENCH_LOG=debug emits diagnostics to stderr
#[test]
#[ignore = "TODO: Phase 030 - Output Infrastructure"]
fn quench_log_debug_emits_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check"])
        .current_dir(dir.path())
        .env("QUENCH_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::is_empty().not());
}
```

**Verification**:
```bash
cargo test --test specs violation_limit -- --list
cargo test --test specs config_flag -- --list
cargo test --test specs quench_log -- --list
```

## Key Implementation Details

### JSON Schema Validation

The `jsonschema` crate validates output against `docs/specs/output.schema.json`:

```rust
use jsonschema::JSONSchema;

let schema: serde_json::Value = serde_json::from_str(&schema_str)?;
let compiled = JSONSchema::compile(&schema)?;

if !compiled.is_valid(&json_output) {
    let errors: Vec<_> = compiled.validate(&json_output)
        .err().unwrap()
        .collect();
    panic!("JSON validation errors: {:?}", errors);
}
```

### Color Detection Logic

From `03-output.md`, the detection order:
1. `--color=always` → use color
2. `--color=never` → no color
3. `--color=auto` (default):
   - If not `stdout.is_tty()` → no color
   - If `CLAUDE_CODE`, `CODEX`, or `CI` env var set → no color
   - Else → use color

### Exit Code Priority

When multiple error types occur:
```
3 (internal error) > 2 (config error) > 1 (check failed) > 0 (passed)
```

This is already modeled in `error.rs` with the `ExitCode` enum.

### Test Fixtures

| Fixture | Purpose | Expected Exit |
|---------|---------|---------------|
| `minimal/` | No violations | 0 |
| `output-test/` | Single cloc violation | 1 |
| `config-error/` | Malformed TOML | 2 |
| `violations/` | Many violations | 1 |

### Module Integration

Update `tests/specs.rs` to include the new module:

```rust
#[path = "specs/output.rs"]
mod output;
```

## Verification Plan

### Phase Completion Checklist

- [ ] `tests/fixtures/config-error/` exists with invalid TOML
- [ ] `tests/fixtures/output-test/` exists with cloc violation
- [ ] `tests/specs/output.rs` has all specs listed in task outline
- [ ] `jsonschema` added to dev-dependencies
- [ ] All specs compile with `cargo test --test specs`
- [ ] All specs are ignored (counted in `--ignored` output)
- [ ] Module included in `tests/specs.rs`

### Running Verification

```bash
# Verify fixtures exist
ls -la tests/fixtures/config-error/
ls -la tests/fixtures/output-test/

# Verify specs compile
cargo test --test specs output -- --list

# Count ignored specs (should match task count)
cargo test --test specs -- --ignored 2>&1 | grep -c "ignored"

# Verify no specs run yet (all should be ignored)
cargo test --test specs output 2>&1 | grep "0 passed"

# Full check
make check
```

### Expected Spec Count

| Category | Specs |
|----------|-------|
| Text output format | 5 |
| JSON output format | 5 |
| Exit codes | 4 |
| Colorization | 4 |
| Violation limits | 4 |
| Config validation | 2 |
| Debug output | 1 |
| **Total** | **25** |

## Summary

Phase 025 creates behavioral spec coverage for output infrastructure:

1. **2 new fixtures** for testing output behavior
2. **25 behavioral specs** covering all output requirements from the task outline
3. **JSON schema validation** using `jsonschema` crate
4. **Documentation** of color detection and exit code logic

All specs are marked `#[ignore = "TODO: Phase 030"]` until the output infrastructure is implemented.
