// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Profile-specific configuration specs for shell, ruby, and python.

use crate::prelude::*;

// =============================================================================
// Shell Profile Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile shell - Shell project defaults
#[test]
fn init_shell_profile_generates_config() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Created quench.toml"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"), "config should have [shell] section");
    assert!(config.contains("[shell.suppress]"), "config should have [shell.suppress] section");
    assert!(config.contains("[shell.policy]"), "config should have [shell.policy] section");
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile includes suppress and policy sections
#[test]
fn init_shell_profile_includes_escape_patterns() {
    let temp = Project::empty();

    quench_cmd().args(["init", "--with", "shell"]).current_dir(temp.path()).assert().success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"), "config should have [shell] section");
    assert!(config.contains("[shell.suppress]"), "config should have [shell.suppress] section");
    assert!(config.contains("[shell.policy]"), "config should have [shell.policy] section");
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile output message includes profile name
#[test]
fn init_shell_profile_message() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("shell"));
}

// =============================================================================
// Ruby Profile Specs
// =============================================================================

/// Spec: docs/specs/langs/ruby.md#profile-defaults
///
/// > --with ruby configures Ruby defaults
#[test]
fn init_with_ruby_configures_ruby_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "ruby"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("ruby"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
    assert!(config.contains("[ruby.suppress]"));
    assert!(config.contains("[ruby.policy]"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with rb is an alias for ruby
#[test]
fn init_with_rb_alias() {
    let temp = Project::empty();

    quench_cmd().args(["init", "--with", "rb"]).current_dir(temp.path()).assert().success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
}

/// Spec: docs/specs/langs/ruby.md#profile-defaults
///
/// > Ruby profile includes suppress and policy sections
#[test]
fn init_ruby_profile_includes_debugger_patterns() {
    let temp = Project::empty();

    quench_cmd().args(["init", "--with", "ruby"]).current_dir(temp.path()).assert().success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"), "config should have [ruby] section");
    assert!(config.contains("[ruby.suppress]"), "config should have [ruby.suppress] section");
    assert!(config.contains("[ruby.policy]"), "config should have [ruby.policy] section");
}

// =============================================================================
// Python Profile Specs
// =============================================================================

/// Spec: docs/specs/langs/python.md#profile-defaults
///
/// > --with python configures Python defaults
#[test]
fn init_with_python_configures_python_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "python"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("python"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"));
    assert!(config.contains("[python.suppress]"));
    assert!(config.contains("[python.policy]"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with py is an alias for python
#[test]
fn init_with_py_alias() {
    let temp = Project::empty();

    quench_cmd().args(["init", "--with", "py"]).current_dir(temp.path()).assert().success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"));
}

/// Spec: docs/specs/langs/python.md#profile-defaults
///
/// > Python profile includes suppress and policy sections
#[test]
fn init_python_profile_includes_debugger_patterns() {
    let temp = Project::empty();

    quench_cmd().args(["init", "--with", "python"]).current_dir(temp.path()).assert().success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"), "config should have [python] section");
    assert!(config.contains("[python.suppress]"), "config should have [python.suppress] section");
    assert!(config.contains("[python.policy]"), "config should have [python.policy] section");
}
