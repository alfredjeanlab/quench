// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Latest metrics cache for local viewing.
//!
//! `.quench/latest.json` caches the most recent metrics locally for:
//! - Quick metric viewing without git operations
//! - `quench report` without requiring git notes fetch
//! - Named "latest" to distinguish from "baseline" (comparison target)

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::check::CheckOutput;

/// Latest metrics cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMetrics {
    /// Last update timestamp (ISO 8601).
    pub updated: DateTime<Utc>,

    /// Git commit hash when metrics were captured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,

    /// Check output with all metrics.
    pub output: CheckOutput,
}

impl LatestMetrics {
    /// Save latest metrics to file, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load latest metrics from file, returning None if not found.
    pub fn load(path: &Path) -> anyhow::Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }
}

/// Get the current HEAD commit hash (short form).
pub fn get_head_commit(root: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        anyhow::bail!("git rev-parse failed")
    }
}

#[cfg(test)]
#[path = "latest_tests.rs"]
mod tests;
