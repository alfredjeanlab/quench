// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn registry_fallback_to_generic() {
    let registry = AdapterRegistry::default();
    let adapter = registry.adapter_for(Path::new("unknown.xyz"));
    assert_eq!(adapter.name(), "generic");
}

#[test]
fn registry_extension_lookup_falls_back() {
    // With no language adapters registered, all files fall back to generic
    let registry = AdapterRegistry::default();
    assert_eq!(registry.adapter_for(Path::new("foo.rs")).name(), "generic");
    assert_eq!(registry.adapter_for(Path::new("bar.py")).name(), "generic");
}

#[test]
fn detect_language_rust_with_cargo_toml() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    assert_eq!(detect_language(dir.path()), ProjectLanguage::Rust);
}

#[test]
fn detect_language_generic_without_cargo_toml() {
    let dir = TempDir::new().unwrap();
    // No Cargo.toml

    assert_eq!(detect_language(dir.path()), ProjectLanguage::Generic);
}

#[test]
fn for_project_registers_rust_adapter() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let registry = AdapterRegistry::for_project(dir.path());
    // With Rust adapter registered, .rs files use rust adapter
    assert_eq!(registry.adapter_for(Path::new("src/lib.rs")).name(), "rust");
}

#[test]
fn for_project_generic_fallback() {
    let dir = TempDir::new().unwrap();
    // No Cargo.toml

    let registry = AdapterRegistry::for_project(dir.path());
    // Without Rust adapter, .rs files fall back to generic
    assert_eq!(registry.adapter_for(Path::new("src/lib.rs")).name(), "generic");
}

#[test]
fn detect_all_languages_single() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let langs = detect_all_languages(dir.path());
    assert_eq!(langs, vec![ProjectLanguage::Rust]);
}

#[test]
fn detect_all_languages_multiple() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    // Add shell scripts
    std::fs::write(dir.path().join("build.sh"), "#!/bin/bash\necho hi\n").unwrap();

    let langs = detect_all_languages(dir.path());
    assert_eq!(langs, vec![ProjectLanguage::Rust, ProjectLanguage::Shell]);
}

#[test]
fn detect_all_languages_empty_is_generic() {
    let dir = TempDir::new().unwrap();
    let langs = detect_all_languages(dir.path());
    assert_eq!(langs, vec![ProjectLanguage::Generic]);
}

#[test]
fn project_language_display() {
    assert_eq!(ProjectLanguage::Rust.to_string(), "Rust");
    assert_eq!(ProjectLanguage::Go.to_string(), "Go");
    assert_eq!(ProjectLanguage::JavaScript.to_string(), "JavaScript");
    assert_eq!(ProjectLanguage::Python.to_string(), "Python");
    assert_eq!(ProjectLanguage::Ruby.to_string(), "Ruby");
    assert_eq!(ProjectLanguage::Shell.to_string(), "Shell");
    assert_eq!(ProjectLanguage::Generic.to_string(), "Generic");
}
