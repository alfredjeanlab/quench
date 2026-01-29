// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Language and agent detection specs.

use crate::prelude::*;

// =============================================================================
// Shell Detection Specs
// =============================================================================

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

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > *.sh in root/bin/scripts -> shell
#[test]
fn init_detects_shell_from_scripts_dir() {
    let temp = Project::empty();
    temp.file("scripts/build.sh", "#!/bin/bash\necho hello\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

// =============================================================================
// Language Detection Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Cargo.toml -> rust
#[test]
fn init_detects_rust_from_cargo_toml() {
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
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > go.mod -> golang
#[test]
fn init_detects_golang_from_go_mod() {
    let temp = Project::empty();
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[golang]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > package.json -> javascript
#[test]
fn init_detects_javascript_from_package_json() {
    let temp = Project::empty();
    temp.file("package.json", "{\"name\": \"test\"}\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[javascript]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detection is additive (multiple languages/agents)
#[test]
fn init_detection_is_additive() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("scripts/deploy.sh", "#!/bin/bash\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should detect rust");
    assert!(config.contains("[shell]"), "should detect shell");
}

// =============================================================================
// Ruby Detection Specs
// =============================================================================

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Auto-detects Ruby from Gemfile
#[test]
fn init_auto_detects_ruby_from_gemfile() {
    let temp = Project::empty();
    temp.file("Gemfile", "source 'https://rubygems.org'\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
}

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Auto-detects Ruby from *.gemspec
#[test]
fn init_auto_detects_ruby_from_gemspec() {
    let temp = Project::empty();
    temp.file("myapp.gemspec", "Gem::Specification.new do |s|\nend\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
}

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Auto-detects Ruby from config.ru (Rack)
#[test]
fn init_auto_detects_ruby_from_config_ru() {
    let temp = Project::empty();
    temp.file("config.ru", "run MyApp\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
}

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Auto-detects Ruby from config/application.rb (Rails)
#[test]
fn init_auto_detects_ruby_from_rails() {
    let temp = Project::empty();
    temp.file("config/application.rb", "module MyApp\nend\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[ruby]"));
}

// =============================================================================
// Python Detection Specs
// =============================================================================

/// Spec: docs/specs/langs/python.md#detection
///
/// > Auto-detects Python from pyproject.toml
#[test]
fn init_auto_detects_python_from_pyproject_toml() {
    let temp = Project::empty();
    temp.file("pyproject.toml", "[project]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"));
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Auto-detects Python from setup.py
#[test]
fn init_auto_detects_python_from_setup_py() {
    let temp = Project::empty();
    temp.file("setup.py", "from setuptools import setup\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"));
}

/// Spec: docs/specs/langs/python.md#detection
///
/// > Auto-detects Python from requirements.txt
#[test]
fn init_auto_detects_python_from_requirements_txt() {
    let temp = Project::empty();
    temp.file("requirements.txt", "requests>=2.28.0\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[python]"));
}

// =============================================================================
// Agent Detection Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > CLAUDE.md -> claude
#[test]
fn init_detects_claude_from_claude_md() {
    let temp = Project::empty();
    temp.file("CLAUDE.md", "# Project\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains("CLAUDE.md"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursorrules -> cursor
#[test]
fn init_detects_cursor_from_cursorrules() {
    let temp = Project::empty();
    temp.file(".cursorrules", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains(".cursorrules"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursor/rules/*.md[c] -> cursor
#[test]
fn init_detects_cursor_from_mdc_rules() {
    let temp = Project::empty();
    temp.file(".cursor/rules/project.mdc", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    // Should detect cursor agent presence
}
