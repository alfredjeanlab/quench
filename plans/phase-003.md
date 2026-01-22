# Phase 003: CLI Contract - Behavioral Specs

**Root Feature:** `quench-6878`

## Overview

Establish the behavioral specification test suite for quench's CLI contract. This phase creates black-box tests that verify the CLI adheres to its specification before implementation begins. All tests are marked `#[ignore = "TODO: Phase 005"]` until the CLI skeleton is implemented.

**Current State**: Basic test infrastructure exists with `tests/specs.rs` and `tests/specs/prelude.rs`. Help and version tests pass.

**End State**: Complete behavioral spec coverage for CLI commands, flags, and environment variables. Tests are ready to be un-ignored as features are implemented.

## Project Structure

```
quench/
├── tests/
│   ├── specs.rs                    # (update) Add CLI contract specs
│   └── specs/
│       ├── CLAUDE.md               # (exists) Spec conventions
│       └── prelude.rs              # (exists) Test helpers
└── docs/
    └── specs/
        ├── 01-cli.md               # (reference) CLI specification
        ├── 02-config.md            # (reference) Config specification
        └── 03-output.md            # (reference) Output specification
```

## Dependencies

No new dependencies required. Uses existing test infrastructure from Phase 001.

## Implementation Phases

### Phase 3.1: Command Specs

**Goal**: Verify CLI commands are exactly: `(none)`, `help`, `check`, `report`, `init`.

**Tasks**:
1. Add spec test verifying bare `quench` shows help
2. Add spec test verifying `quench help` shows help
3. Add spec test verifying `quench check` is recognized
4. Add spec test verifying `quench report` is recognized
5. Add spec test verifying `quench init` is recognized
6. Add spec test verifying unknown commands produce error with exit code 2

**Test Code**:

```rust
/// Spec: docs/specs/01-cli.md#commands
///
/// > quench (bare invocation) shows help
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn bare_invocation_shows_help() {
    quench_cmd()
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench help shows help
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn help_command_shows_help() {
    quench_cmd()
        .arg("help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench report generates reports
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn report_command_exists() {
    quench_cmd()
        .arg("report")
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#commands
///
/// > quench init initializes configuration
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn init_command_exists() {
    let dir = tempfile::tempdir().unwrap();
    quench_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#exit-codes
///
/// > Exit code 2 for unknown commands
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn unknown_command_fails() {
    quench_cmd()
        .arg("unknown")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("unrecognized"));
}
```

**Verification**:
```bash
cargo test --test specs -- --ignored 2>&1 | grep -c "ignore"
# Should show count of ignored tests
```

### Phase 3.2: Global Flag Specs

**Goal**: Verify global short flags are exactly: `-h`, `-V`, `-C`.

**Tasks**:
1. Add spec test verifying `-h` shows help
2. Add spec test verifying `-V` shows version
3. Add spec test verifying `-C` accepts config path
4. Add spec test verifying unknown global flags produce error

**Test Code**:

```rust
/// Spec: docs/specs/01-cli.md#global-flags
///
/// > -h shows help (short for --help)
#[test]
fn short_help_flag_works() {
    quench_cmd()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > -V shows version (short for --version)
#[test]
fn short_version_flag_works() {
    quench_cmd()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > -C <FILE> specifies config file (short for --config)
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn short_config_flag_works() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "version = 1\n").unwrap();

    quench_cmd()
        .args(["-C", config_path.to_str().unwrap(), "check"])
        .current_dir(dir.path())
        .assert()
        .success();
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > Unknown global flags produce error, not silently ignored
#[test]
fn unknown_global_flag_fails() {
    quench_cmd()
        .arg("-x")
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}

/// Spec: docs/specs/01-cli.md#global-flags
///
/// > Unknown long flags produce error
#[test]
fn unknown_long_flag_fails() {
    quench_cmd()
        .arg("--unknown-flag")
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}
```

**Verification**:
```bash
cargo test --test specs short_
cargo test --test specs unknown_
```

### Phase 3.3: Check Command Flag Specs

**Goal**: Verify `check` short flags are exactly: `-o`.

**Tasks**:
1. Add spec test verifying `-o` accepts output format
2. Add spec test verifying `-o json` produces JSON
3. Add spec test verifying `-o text` produces text
4. Add spec test verifying unknown check flags produce error

**Test Code**:

```rust
/// Spec: docs/specs/01-cli.md#output-flags
///
/// > -o <FMT> sets output format (short for --output)
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_short_output_flag_works() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("{"));
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > -o json produces JSON output
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_output_json_format() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("output should be valid JSON");
    assert!(json.get("passed").is_some(), "JSON should have 'passed' field");
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Unknown flags to check command produce error
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_unknown_flag_fails() {
    quench_cmd()
        .args(["check", "-x"])
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}

/// Spec: docs/specs/01-cli.md#output-flags
///
/// > Unknown long flags to check command produce error
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn check_unknown_long_flag_fails() {
    quench_cmd()
        .args(["check", "--unknown-option"])
        .assert()
        .code(2)
        .stderr(predicates::str::is_match(r"(?i)(unexpected|unknown|unrecognized)").unwrap());
}
```

**Verification**:
```bash
cargo test --test specs check_
```

### Phase 3.4: Config Warning Specs

**Goal**: Verify unrecognized config keys produce warning (not error).

**Tasks**:
1. Add spec test for warning on unknown top-level key
2. Add spec test for warning on unknown nested key
3. Verify valid config keys don't produce warnings
4. Verify warnings go to stderr, not stdout

**Test Code**:

```rust
/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown keys are warnings (forward compatibility)
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn unknown_config_key_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        "version = 1\nunknown_key = true\n",
    ).unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success()  // Should not fail
        .stderr(predicates::str::contains("unknown").or(predicates::str::contains("unrecognized")));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown nested keys are warnings
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn unknown_nested_config_key_warns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.unknown]
field = "value"
"#,
    ).unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("unknown").or(predicates::str::contains("unrecognized")));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Valid config produces no warnings
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn valid_config_no_warnings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicates::str::is_empty().or(
            predicates::str::contains("warning").not()
        ));
}
```

**Verification**:
```bash
cargo test --test specs config_
```

### Phase 3.5: Environment Variable Specs

**Goal**: Verify env vars are exactly: `QUENCH_NO_COLOR`, `QUENCH_CONFIG`, `QUENCH_LOG`.

**Tasks**:
1. Add spec test for `QUENCH_NO_COLOR=1` disabling color
2. Add spec test for `QUENCH_CONFIG` setting config path
3. Add spec test for `QUENCH_LOG=debug` enabling logging
4. Add spec test verifying unknown `QUENCH_*` vars are ignored (no error)

**Test Code**:

```rust
/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_NO_COLOR=1 disables color output
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn env_no_color_disables_color() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    // Create a file that would trigger a violation with colored output
    std::fs::write(dir.path().join("test.rs"), "fn main() {}\n").unwrap();

    let output = quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI escape codes start with \x1b[
    assert!(!stdout.contains("\x1b["), "output should not contain ANSI codes");
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_CONFIG sets config file location
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn env_config_sets_path() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("custom-config.toml");
    std::fs::write(&config_path, "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_CONFIG", config_path.to_str().unwrap())
        .assert()
        .success();
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG enables debug logging to stderr
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn env_log_enables_debug() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::contains("DEBUG").or(predicates::str::contains("debug")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG=trace enables trace logging
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn env_log_trace_level() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

    quench_cmd()
        .arg("check")
        .current_dir(dir.path())
        .env("QUENCH_LOG", "trace")
        .assert()
        .success()
        .stderr(predicates::str::contains("TRACE").or(predicates::str::contains("trace")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > Unknown QUENCH_* environment variables are silently ignored
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn env_unknown_vars_ignored() {
    quench_cmd()
        .arg("--help")
        .env("QUENCH_UNKNOWN_VAR", "some_value")
        .assert()
        .success();  // Should not error on unknown env vars
}
```

**Verification**:
```bash
cargo test --test specs env_
```

### Phase 3.6: Integration and Cleanup

**Goal**: Ensure all specs compile, are properly documented, and follow conventions.

**Tasks**:
1. Ensure all tests have doc comments referencing spec location
2. Verify all `#[ignore]` attributes have proper format
3. Run full test suite to verify compilation
4. Update test count expectations

**Verification**:
```bash
# All tests should compile
cargo test --test specs --no-run

# Count ignored tests (should be ~20 for Phase 005)
cargo test --test specs -- --ignored 2>&1 | grep -c "ignored"

# Passing tests should still pass
cargo test --test specs

# Full CI check
make check
```

## Key Implementation Details

### Test Organization Pattern

Group related tests in the `tests/specs.rs` file using comment headers:

```rust
// =============================================================================
// COMMAND SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#commands
#[test]
#[ignore = "TODO: Phase 005 - CLI skeleton"]
fn bare_invocation_shows_help() { ... }

// =============================================================================
// GLOBAL FLAG SPECS
// =============================================================================

/// Spec: docs/specs/01-cli.md#global-flags
#[test]
fn short_help_flag_works() { ... }
```

### Ignore Annotation Convention

Per `tests/specs/CLAUDE.md`, use this exact format:

```rust
#[ignore = "TODO: Phase N - Brief description"]
```

All specs in this phase should use:
```rust
#[ignore = "TODO: Phase 005 - CLI skeleton"]
```

### Exit Code Assertions

Per `docs/specs/01-cli.md#exit-codes`:

| Code | Meaning | Usage |
|------|---------|-------|
| 0 | Success | `.assert().success()` |
| 1 | Check failed | `.assert().code(1)` |
| 2 | Config/arg error | `.assert().code(2)` |
| 3 | Internal error | `.assert().code(3)` |

### Temp Directory Usage

For tests that need a clean filesystem:

```rust
let dir = tempfile::tempdir().unwrap();
std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();

quench_cmd()
    .arg("check")
    .current_dir(dir.path())
    .assert()
    .success();
```

### Predicates Usage

For flexible string matching:

```rust
use predicates::prelude::*;

// Contains substring
.stderr(predicates::str::contains("error"))

// Regex match (for case-insensitive or flexible matching)
.stderr(predicates::str::is_match(r"(?i)(error|failed)").unwrap())

// Combined predicates
.stderr(predicates::str::contains("unknown").or(predicates::str::contains("unrecognized")))
```

## Verification Plan

### Phase Completion Checklist

- [ ] All command specs added (bare, help, check, report, init, unknown)
- [ ] All global flag specs added (-h, -V, -C, unknown)
- [ ] All check flag specs added (-o, unknown)
- [ ] All config warning specs added (unknown keys at various levels)
- [ ] All env var specs added (QUENCH_NO_COLOR, QUENCH_CONFIG, QUENCH_LOG)
- [ ] All specs have proper doc comments with spec references
- [ ] All unimplemented specs have `#[ignore = "TODO: Phase 005 - CLI skeleton"]`
- [ ] `cargo test --test specs` passes (non-ignored tests)
- [ ] `cargo test --test specs --no-run` compiles all tests
- [ ] `make check` passes

### Test Count Summary

After Phase 003, `tests/specs.rs` should contain approximately:

| Category | Tests | Passing Now | Ignored |
|----------|-------|-------------|---------|
| Commands | 6 | 2 (help, version) | 4 |
| Global flags | 5 | 3 (-h, -V, unknown) | 2 |
| Check flags | 4 | 0 | 4 |
| Config warnings | 3 | 0 | 3 |
| Env vars | 5 | 0 | 5 |
| Output snapshot | 1 | 0 | 1 (Phase 030) |
| **Total** | **24** | **5** | **19** |

### Running Tests

```bash
# Quick validation - passing tests
cargo test --test specs

# Show ignored count
cargo test --test specs -- --ignored 2>&1 | grep "test result"

# Compile check all tests
cargo test --test specs --no-run

# Full CI
make check
```

### Expected Output

```
running 24 tests
test help_exits_successfully ... ok
test version_exits_successfully ... ok
test short_help_flag_works ... ok
test short_version_flag_works ... ok
test unknown_global_flag_fails ... ok

test result: ok. 5 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out
```

## Summary

This phase creates comprehensive behavioral specifications for the CLI contract:

1. **Command specs**: Verify exact command set (none, help, check, report, init)
2. **Global flag specs**: Verify exact short flags (-h, -V, -C)
3. **Check flag specs**: Verify exact short flags (-o)
4. **Error handling specs**: Verify unrecognized flags produce errors
5. **Config warning specs**: Verify unknown config keys warn (not error)
6. **Environment specs**: Verify exact env vars (QUENCH_NO_COLOR, QUENCH_CONFIG, QUENCH_LOG)

All specs are black-box tests per `tests/specs/CLAUDE.md` conventions, marked ignored until implementation in Phase 005.
