// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for `quench config` command.

use crate::prelude::*;

// =============================================================================
// Basic Command Behavior
// =============================================================================

/// Spec: `quench config <feature>` outputs the guide template for that feature
///
/// > When users want configuration examples, they can run `quench config <feature>`
/// > to see the reference guide for that check or language.
#[test]
fn outputs_guide_template_for_feature() {
    let output = quench_cmd().args(["config", "rust"]).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Rust Configuration Guide"),
        "Should output Rust configuration guide header"
    );
    assert!(
        stdout.contains("Configuration reference for Rust language support"),
        "Should include guide description"
    );
    assert!(
        stdout.contains("[rust]"),
        "Should contain TOML configuration examples"
    );
}

/// Spec: Language aliases are supported (e.g., `js` for `javascript`)
///
/// > Common abbreviations should work as shortcuts.
#[test]
fn supports_language_aliases() {
    let output = quench_cmd().args(["config", "js"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# JavaScript/TypeScript Configuration Guide"),
        "js alias should show JavaScript guide"
    );

    let output = quench_cmd().args(["config", "rs"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Rust Configuration Guide"),
        "rs alias should show Rust guide"
    );

    let output = quench_cmd().args(["config", "py"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Python Configuration Guide"),
        "py alias should show Python guide"
    );

    let output = quench_cmd().args(["config", "rb"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Ruby Configuration Guide"),
        "rb alias should show Ruby guide"
    );

    let output = quench_cmd().args(["config", "go"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Go Configuration Guide"),
        "go alias should show Go guide"
    );

    let output = quench_cmd().args(["config", "sh"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Shell Configuration Guide"),
        "sh alias should show Shell guide"
    );

    let output = quench_cmd().args(["config", "bash"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# Shell Configuration Guide"),
        "bash alias should show Shell guide"
    );

    let output = quench_cmd().args(["config", "ts"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# JavaScript/TypeScript Configuration Guide"),
        "ts alias should show JavaScript/TypeScript guide"
    );

    let output = quench_cmd()
        .args(["config", "typescript"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("# JavaScript/TypeScript Configuration Guide"),
        "typescript alias should show JavaScript/TypeScript guide"
    );
}

/// Spec: Check names are supported
///
/// > All check names should have corresponding guides.
#[test]
fn supports_check_names() {
    for check in &[
        "agents", "build", "cloc", "docs", "escapes", "git", "license", "tests",
    ] {
        let output = quench_cmd().args(["config", check]).assert().success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout);
        assert!(
            stdout.contains("Configuration Guide"),
            "Check {} should have a configuration guide",
            check
        );
    }
}

/// Spec: Unknown features show helpful error message
///
/// > When an invalid feature is requested, show available options.
#[test]
fn unknown_feature_shows_helpful_error() {
    let output = quench_cmd().args(["config", "invalid"]).assert().failure();
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);

    assert!(
        stderr.contains("Unknown feature 'invalid'"),
        "Should indicate the feature is unknown"
    );
    assert!(
        stderr.contains("Available features:"),
        "Should list available features"
    );
    assert!(stderr.contains("agents, build, cloc"), "Should list checks");
    assert!(
        stderr.contains("golang (go), javascript (js/ts/typescript)"),
        "Should list languages with aliases"
    );
}

/// Spec: Feature names are case-insensitive
///
/// > Users should be able to use RUST, Rust, or rust.
#[test]
fn feature_names_are_case_insensitive() {
    let lowercase = quench_cmd().args(["config", "rust"]).assert().success();
    let uppercase = quench_cmd().args(["config", "RUST"]).assert().success();
    let mixed = quench_cmd().args(["config", "Rust"]).assert().success();

    assert_eq!(
        lowercase.get_output().stdout,
        uppercase.get_output().stdout,
        "RUST should match rust"
    );
    assert_eq!(
        lowercase.get_output().stdout,
        mixed.get_output().stdout,
        "Rust should match rust"
    );
}

// =============================================================================
// Help Output
// =============================================================================

/// Spec: `quench config` with no arguments shows help
///
/// > Running config without a feature argument should display usage help
/// > and exit successfully.
#[test]
fn no_args_shows_help() {
    let output = quench_cmd().args(["config"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(
        stdout.contains("Read configuration reference documentation"),
        "Should show command description"
    );
    assert!(stdout.contains("[FEATURE]"), "Should show feature argument");
}

/// Spec: `quench config --help` shows usage information
#[test]
fn help_shows_usage() {
    let output = quench_cmd().args(["config", "--help"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(
        stdout.contains("Read configuration reference documentation"),
        "Should show command description"
    );
    assert!(stdout.contains("[FEATURE]"), "Should show feature argument");
    assert!(
        stdout.contains("Feature to show configuration for"),
        "Should show feature description"
    );
}

/// Spec: `quench --help` lists the config command
#[test]
fn listed_in_main_help() {
    let output = quench_cmd().args(["--help"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(stdout.contains("config"), "Should list config command");
    assert!(
        stdout.contains("Read configuration reference documentation"),
        "Should show config description"
    );
}
