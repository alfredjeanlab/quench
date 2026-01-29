// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Basic creation, force flag, and profile validation specs.

use crate::prelude::*;

// =============================================================================
// Basic Creation Specs
// =============================================================================

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

// =============================================================================
// Force Flag Specs
// =============================================================================

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

// =============================================================================
// Explicit Profile Specs
// =============================================================================

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

// =============================================================================
// Profile Name Validation Specs
// =============================================================================

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
        .args(["init", "--with", "cobol"])
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

// =============================================================================
// --with Flag Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with flag accepts comma-separated profiles
#[test]
fn init_with_accepts_comma_separated_profiles() {
    let temp = Project::empty();
    quench_cmd()
        .args(["init", "--with", "rust,shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with skips auto-detection
#[test]
fn init_with_skips_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
    // Check for actual section headers, not comments
    assert!(
        !config.lines().any(|l| l.trim() == "[rust]"),
        "--with should skip rust detection"
    );
    assert!(
        !config.lines().any(|l| l.trim() == "[golang]"),
        "--with should skip go detection"
    );
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > no --with triggers auto-detection
#[test]
fn init_without_with_triggers_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should auto-detect rust");
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile rust,shell - Multi-language project
#[test]
fn init_combined_profiles_generates_both() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust,shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("rust, shell"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("[rust]"),
        "config should have [rust] section"
    );
    assert!(
        config.contains("[shell]"),
        "config should have [shell] section"
    );
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detected language appends [lang] section with dotted keys
#[test]
fn init_detected_language_uses_dotted_keys() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    assert!(config.contains("[rust]"));
    assert!(config.contains("rust.cloc.check"));
    assert!(config.contains("rust.policy.check"));
    assert!(config.contains("rust.suppress.check"));
}
