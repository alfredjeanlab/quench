// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Auto-detection for Rust test runners.

use std::path::Path;

/// Rust test runner (only cargo supported).
#[derive(Debug)]
pub enum RustRunner {
    Cargo,
}

impl RustRunner {
    pub fn name(&self) -> &str {
        match self {
            Self::Cargo => "cargo",
        }
    }
}

/// Detection result for Rust test runner.
#[derive(Debug)]
pub struct RustDetectionResult {
    pub runner: RustRunner,
    pub source: RustDetectionSource,
}

/// How the Rust runner was detected.
#[derive(Debug)]
pub enum RustDetectionSource {
    /// Detected from Cargo.toml.
    CargoToml,
}

impl RustDetectionSource {
    pub fn to_metric_string(&self) -> String {
        match self {
            Self::CargoToml => "cargo_toml".to_string(),
        }
    }
}

/// Detect Rust test runner.
///
/// Returns None if no Cargo.toml exists.
pub fn detect_rust_runner(root: &Path) -> Option<RustDetectionResult> {
    if root.join("Cargo.toml").exists() {
        Some(RustDetectionResult {
            runner: RustRunner::Cargo,
            source: RustDetectionSource::CargoToml,
        })
    } else {
        None
    }
}
