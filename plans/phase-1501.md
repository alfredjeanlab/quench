# Phase 1501: Init Command - Behavioral Specs

**Root Feature:** `quench-91c5-2`

## Overview

Add comprehensive behavioral specs for the `quench init` command. This phase focuses on test coverage for existing functionality, ensuring the init command behavior is fully specified through black-box tests.

**Note:** The CLI uses `--with` flag (not `--profile`). The task outline mentions `--profile` but the implementation uses `--with`.

## Project Structure

```
tests/specs/cli/
└── init.rs          # Behavioral specs for quench init command
```

Key reference files:
- `docs/specs/01-cli.md` - CLI specification (quench init section)
- `crates/cli/src/cmd_init.rs` - Init command implementation
- `crates/cli/src/init.rs` - Detection logic
- `crates/cli/src/profiles.rs` - Profile templates

## Dependencies

No new dependencies. Uses existing test infrastructure:
- `tests/specs/prelude.rs` - Test helpers (`Project`, `quench_cmd`, etc.)
- `assert_cmd` - Command testing
- `predicates` - Output assertions

## Implementation Phases

### Phase 1: Basic Creation Specs

Add specs for fundamental init behavior.

**File:** `tests/specs/cli/init.rs`

```rust
/// Spec: docs/specs/01-cli.md#quench-init
///
/// > quench init creates quench.toml in current directory
#[test]
fn init_creates_quench_toml_in_current_directory() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    assert!(temp.path().join("quench.toml").exists());
}
```

**Milestone:** Basic creation spec passes.

### Phase 2: Force Flag Specs

Add specs for `--force` overwrite behavior.

```rust
/// Spec: docs/specs/01-cli.md#quench-init
///
/// > Refuses to overwrite existing quench.toml without --force
#[test]
fn init_refuses_to_overwrite_without_force() {
    let temp = Project::empty();
    temp.file("quench.toml", "version = 1\n# existing\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("already exists"))
        .stderr(predicates::str::contains("--force"));
}

/// Spec: docs/specs/01-cli.md#quench-init
///
/// > --force overwrites existing quench.toml
#[test]
fn init_force_overwrites_existing_config() {
    let temp = Project::empty();
    temp.file("quench.toml", "version = 1\n# existing content\n");

    quench_cmd()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(!config.contains("# existing content"), "should overwrite");
    assert!(config.contains("version = 1"));
}
```

**Milestone:** Force flag specs pass.

### Phase 3: Explicit Profile Specs

Add specs for each profile type when using `--with`.

```rust
/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with rust configures Rust defaults
#[test]
fn init_with_rust_configures_rust_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("rust"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("[rust.suppress]"));
    assert!(config.contains("[rust.policy]"));
    assert!(config.contains("unsafe"), "should have unsafe escape pattern");
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with claude configures CLAUDE.md defaults
#[test]
fn init_with_claude_configures_claude_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "claude"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("CLAUDE.md"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with cursor configures .cursorrules defaults
#[test]
fn init_with_cursor_configures_cursor_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "cursor"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains(".cursorrules"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with rust,claude combines profiles
#[test]
fn init_with_combined_profiles() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust,claude"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("CLAUDE.md"));
}
```

**Milestone:** All explicit profile specs pass.

### Phase 4: Auto-Detection Specs

Add/verify specs for auto-detection behavior.

```rust
/// Spec: docs/specs/01-cli.md#auto-detection
///
/// > No --with triggers auto-detection from project root
#[test]
fn init_without_with_auto_detects() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("detected"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
}

/// Spec: docs/specs/01-cli.md#auto-detection
///
/// > Auto-detects Shell when *.sh in root
#[test]
fn init_auto_detects_shell_from_root_sh() {
    let temp = Project::empty();
    temp.file("build.sh", "#!/bin/bash\necho hello\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/01-cli.md#auto-detection
///
/// > Auto-detects Shell when *.sh in bin/
#[test]
fn init_auto_detects_shell_from_bin_dir() {
    let temp = Project::empty();
    temp.file("bin/run.sh", "#!/bin/bash\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}
```

**Milestone:** Auto-detection specs pass.

### Phase 5: Profile Name Validation Specs

Add specs to verify profile name constraints.

```rust
/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Valid profile names: rust, shell, claude, cursor (plus golang, javascript)
#[test]
fn init_accepts_valid_profile_names() {
    for profile in ["rust", "shell", "claude", "cursor"] {
        let temp = Project::empty();

        quench_cmd()
            .args(["init", "--with", profile])
            .current_dir(temp.path())
            .assert()
            .success();

        assert!(temp.path().join("quench.toml").exists());
    }
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Unknown profile names produce warning
#[test]
fn init_warns_on_unknown_profile() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "python"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("unknown profile"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Profile names are case-insensitive
#[test]
fn init_profile_names_case_insensitive() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "RUST"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
}
```

**Milestone:** Profile validation specs pass.

### Phase 6: Cleanup and Verification

Review all specs and ensure complete coverage.

**Checklist:**
- [ ] Spec: creates quench.toml in current directory
- [ ] Spec: refuses to overwrite without --force
- [ ] Spec: --force overwrites existing config
- [ ] Spec: --with rust configures Rust defaults
- [ ] Spec: --with shell configures Shell defaults
- [ ] Spec: --with claude configures CLAUDE.md defaults
- [ ] Spec: --with cursor configures .cursorrules defaults
- [ ] Spec: --with rust,claude combines profiles
- [ ] Spec: no --with auto-detects from project root
- [ ] Spec: auto-detects Rust when Cargo.toml present
- [ ] Spec: auto-detects Shell when *.sh in root/bin/scripts
- [ ] Spec: auto-detects Claude when CLAUDE.md exists
- [ ] Spec: auto-detects Cursor when .cursorrules exists
- [ ] Spec: profile names validation

**Milestone:** All specs pass, `make check` succeeds.

## Key Implementation Details

### Existing Test Coverage

The following tests already exist in `tests/specs/cli/init.rs`:
- `init_shell_profile_generates_config` - Shell profile generation
- `init_combined_profiles_generates_both` - Combined profiles (rust,shell)
- `init_with_skips_auto_detection` - --with disables auto-detect
- `init_without_with_triggers_auto_detection` - Auto-detection trigger
- `init_detects_rust_from_cargo_toml` - Rust detection
- `init_detects_shell_from_scripts_dir` - Shell detection (scripts/)
- `init_detects_claude_from_claude_md` - Claude detection
- `init_detects_cursor_from_cursorrules` - Cursor detection
- `init_detected_language_uses_dotted_keys` - Dotted key format

### New Specs to Add

1. **Basic creation** - Verify file is created in current directory
2. **Force flag** - Overwrite behavior with/without --force
3. **Explicit rust profile** - Full Rust profile configuration
4. **Explicit claude profile** - Claude agent configuration
5. **Explicit cursor profile** - Cursor agent configuration
6. **Shell detection from root** - *.sh in project root
7. **Shell detection from bin/** - *.sh in bin/ directory
8. **Profile name validation** - Valid/invalid names, case sensitivity

### Exit Codes

Per `docs/specs/01-cli.md`:
- `0` - Success
- `2` - Configuration error (e.g., quench.toml already exists)

### Test Pattern

All specs use the `Project::empty()` helper for isolated test directories:

```rust
let temp = Project::empty();
// Create files as needed
temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

quench_cmd()
    .args(["init"])
    .current_dir(temp.path())
    .assert()
    .success();
```

## Verification Plan

### Run Specs

```bash
# Run all init specs
cargo test --test specs init

# Run specific spec
cargo test --test specs init_refuses_to_overwrite
```

### Full Check

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

## Checklist

- [ ] Add basic creation spec
- [ ] Add --force flag specs (2 tests)
- [ ] Add explicit profile specs (rust, claude, cursor)
- [ ] Add combined profile spec
- [ ] Add shell detection from root spec
- [ ] Add shell detection from bin/ spec
- [ ] Add profile name validation specs
- [ ] Verify existing specs still pass
- [ ] Run `make check`
- [ ] Remove any `#[ignore]` from implemented specs
