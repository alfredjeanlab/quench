//! Behavioral specs for `quench init` command.

use crate::prelude::*;

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile shell - Shell project defaults
#[test]
fn init_shell_profile_generates_config() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--profile", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Created quench.toml"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("[shell]"),
        "config should have [shell] section"
    );
    assert!(
        config.contains("[shell.suppress]"),
        "config should have [shell.suppress] section"
    );
    assert!(
        config.contains("[shell.policy]"),
        "config should have [shell.policy] section"
    );
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile includes escape patterns for set +e, eval, rm -rf
#[test]
fn init_shell_profile_includes_escape_patterns() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--profile", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("set \\\\+e") || config.contains("set \\+e"),
        "config should have set +e escape pattern"
    );
    assert!(
        config.contains("eval"),
        "config should have eval escape pattern"
    );
    assert!(
        config.contains("# OK:"),
        "config should have # OK: comment marker"
    );
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile rust,shell - Multi-language project
#[test]
fn init_combined_profiles_generates_both() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--profile", "rust,shell"])
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

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile output message includes profile name
#[test]
fn init_shell_profile_message() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--profile", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("shell"));
}
