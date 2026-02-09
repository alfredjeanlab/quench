// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Auto-detection for Go test runners.

use std::path::Path;
use std::process::{Command, Stdio};

/// Go test runner (only go test supported).
#[derive(Debug)]
pub enum GoRunner {
    Go,
}

impl GoRunner {
    pub fn name(&self) -> &str {
        match self {
            Self::Go => "go",
        }
    }
}

/// Detection result for Go test runner.
#[derive(Debug)]
pub struct GoDetectionResult {
    pub runner: GoRunner,
    pub source: GoDetectionSource,
}

/// How the Go runner was detected.
#[derive(Debug)]
pub enum GoDetectionSource {
    /// Detected from go.mod.
    GoMod,
}

impl GoDetectionSource {
    pub fn to_metric_string(&self) -> String {
        match self {
            Self::GoMod => "go_mod".to_string(),
        }
    }
}

/// Detect Go test runner.
///
/// Returns None if go.mod doesn't exist or go binary is not available.
pub fn detect_go_runner(root: &Path) -> Option<GoDetectionResult> {
    if !root.join("go.mod").exists() {
        return None;
    }

    // Verify go binary is available
    let go_available = Command::new("go")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if go_available {
        Some(GoDetectionResult { runner: GoRunner::Go, source: GoDetectionSource::GoMod })
    } else {
        None
    }
}
