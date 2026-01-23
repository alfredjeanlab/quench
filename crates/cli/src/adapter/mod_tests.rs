// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
    assert_eq!(
        registry.adapter_for(Path::new("src/lib.rs")).name(),
        "generic"
    );
}
