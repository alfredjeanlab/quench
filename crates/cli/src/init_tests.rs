// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn detect_rust_from_cargo_toml() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Rust));
}

#[test]
fn detect_golang_from_go_mod() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("go.mod"), "module test").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Golang));
}

#[test]
fn detect_javascript_from_package_json() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::JavaScript));
}

#[test]
fn detect_javascript_from_tsconfig() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("tsconfig.json"), "{}").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::JavaScript));
}

#[test]
fn detect_shell_from_root_sh() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("build.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detect_shell_from_scripts_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    fs::write(temp.path().join("scripts/deploy.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detect_shell_from_bin_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("bin")).unwrap();
    fs::write(temp.path().join("bin/run.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn detection_is_additive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]").unwrap();
    fs::create_dir(temp.path().join("scripts")).unwrap();
    fs::write(temp.path().join("scripts/test.sh"), "#!/bin/bash").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Rust));
    assert!(detected.contains(&DetectedLanguage::Shell));
}

#[test]
fn no_markers_returns_empty() {
    let temp = TempDir::new().unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.is_empty());
}

// =============================================================================
// Ruby Detection Tests
// =============================================================================

#[test]
fn detect_ruby_from_gemfile() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Ruby));
}

#[test]
fn detect_ruby_from_gemspec() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("myapp.gemspec"), "Gem::Specification.new").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Ruby));
}

#[test]
fn detect_ruby_from_config_ru() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("config.ru"), "run MyApp").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Ruby));
}

#[test]
fn detect_ruby_from_rails() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(temp.path().join("config/application.rb"), "module MyApp").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Ruby));
}

// =============================================================================
// Python Detection Tests
// =============================================================================

#[test]
fn detect_python_from_pyproject_toml() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname = \"test\"",
    )
    .unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Python));
}

#[test]
fn detect_python_from_setup_py() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("setup.py"), "from setuptools import setup").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Python));
}

#[test]
fn detect_python_from_setup_cfg() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("setup.cfg"), "[metadata]\nname = test").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Python));
}

#[test]
fn detect_python_from_requirements_txt() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("requirements.txt"), "requests==2.28.0").unwrap();

    let detected = detect_languages(temp.path());
    assert!(detected.contains(&DetectedLanguage::Python));
}

// =============================================================================
// Agent Detection Tests
// =============================================================================

#[test]
fn detect_claude_from_claude_md() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("CLAUDE.md"), "# Project").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Claude));
}

#[test]
fn detect_cursor_from_cursorrules() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".cursorrules"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor(CursorMarker::Cursorrules)));
}

#[test]
fn detect_cursor_from_mdc_rules() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".cursor/rules")).unwrap();
    fs::write(temp.path().join(".cursor/rules/project.mdc"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor(CursorMarker::CursorRulesDir)));
}

#[test]
fn detect_cursor_from_md_rules() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".cursor/rules")).unwrap();
    fs::write(temp.path().join(".cursor/rules/project.md"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor(CursorMarker::CursorRulesDir)));
}

#[test]
fn agent_detection_is_additive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("CLAUDE.md"), "# Project").unwrap();
    fs::write(temp.path().join(".cursorrules"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Claude));
    assert!(detected.contains(&DetectedAgent::Cursor(CursorMarker::Cursorrules)));
}

#[test]
fn no_agent_markers_returns_empty() {
    let temp = TempDir::new().unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.is_empty());
}
