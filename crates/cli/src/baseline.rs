// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Baseline file I/O for ratcheting.

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current baseline format version.
pub const BASELINE_VERSION: u32 = 1;

/// Baseline file containing stored metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    /// Format version for forward compatibility.
    pub version: u32,

    /// Last update timestamp (ISO 8601).
    pub updated: DateTime<Utc>,

    /// Git commit hash when baseline was set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,

    /// Stored metrics.
    pub metrics: BaselineMetrics,
}

/// All tracked metrics in the baseline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaselineMetrics {
    /// Coverage percentage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageMetrics>,

    /// Escape hatch counts by pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escapes: Option<EscapesMetrics>,

    /// Binary sizes in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_size: Option<HashMap<String, u64>>,

    /// Build times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_time: Option<BuildTimeMetrics>,

    /// Test execution times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_time: Option<TestTimeMetrics>,
}

/// Coverage metrics with optional per-package breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    pub total: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_package: Option<HashMap<String, f64>>,
}

/// Escape hatch counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapesMetrics {
    /// Source file escape counts by pattern name.
    pub source: HashMap<String, usize>,
    /// Test file escape counts (tracked but not ratcheted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<HashMap<String, usize>>,
}

/// Build time metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTimeMetrics {
    pub cold: f64,
    pub hot: f64,
}

/// Test time metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTimeMetrics {
    pub total: f64,
    pub avg: f64,
    pub max: f64,
}

impl Default for Baseline {
    fn default() -> Self {
        Self::new()
    }
}

impl Baseline {
    /// Create a new baseline with current timestamp.
    pub fn new() -> Self {
        Self {
            version: BASELINE_VERSION,
            updated: Utc::now(),
            commit: None,
            metrics: BaselineMetrics::default(),
        }
    }

    /// Load baseline from file, returning None if not found.
    pub fn load(path: &Path) -> Result<Option<Self>, BaselineError> {
        if !path.exists() {
            return Ok(None);
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| BaselineError::Read(e.to_string()))?;

        let baseline: Baseline =
            serde_json::from_str(&content).map_err(|e| BaselineError::Parse(e.to_string()))?;

        // Version check for forward compatibility
        if baseline.version > BASELINE_VERSION {
            return Err(BaselineError::Version {
                found: baseline.version,
                supported: BASELINE_VERSION,
            });
        }

        Ok(Some(baseline))
    }

    /// Save baseline to file, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), BaselineError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BaselineError::Write(e.to_string()))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| BaselineError::Serialize(e.to_string()))?;

        std::fs::write(path, content).map_err(|e| BaselineError::Write(e.to_string()))?;

        Ok(())
    }

    /// Set git commit hash from current HEAD.
    pub fn with_commit(mut self, root: &Path) -> Self {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(root)
            .output()
            && output.status.success()
        {
            self.commit = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        self
    }

    /// Update the timestamp to now.
    pub fn touch(&mut self) {
        self.updated = Utc::now();
    }
}

/// Errors that can occur during baseline operations.
#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("failed to read baseline: {0}")]
    Read(String),

    #[error("failed to parse baseline: {0}")]
    Parse(String),

    #[error("baseline version {found} is newer than supported {supported}")]
    Version { found: u32, supported: u32 },

    #[error("failed to serialize baseline: {0}")]
    Serialize(String),

    #[error("failed to write baseline: {0}")]
    Write(String),
}

#[cfg(test)]
#[path = "baseline_tests.rs"]
mod tests;
