// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ratcheting configuration.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use super::CheckLevel;
use crate::tolerance::{parse_duration, parse_size};

/// Ratcheting configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RatchetConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: CheckLevel,

    /// Ratchet coverage (default: true).
    #[serde(default = "default_true")]
    pub coverage: bool,

    /// Ratchet escape hatch counts (default: true).
    #[serde(default = "default_true")]
    pub escapes: bool,

    /// Ratchet binary size (default: false).
    #[serde(default)]
    pub binary_size: bool,

    /// Ratchet cold build time (default: false).
    #[serde(default)]
    pub build_time_cold: bool,

    /// Ratchet hot build time (default: false).
    #[serde(default)]
    pub build_time_hot: bool,

    /// Ratchet total test time (default: false).
    #[serde(default)]
    pub test_time_total: bool,

    /// Ratchet average test time (default: false).
    #[serde(default)]
    pub test_time_avg: bool,

    /// Ratchet max single test time (default: false).
    #[serde(default)]
    pub test_time_max: bool,

    /// Coverage tolerance (percentage points allowed to drop).
    #[serde(default)]
    pub coverage_tolerance: Option<f64>,

    /// Binary size tolerance (e.g., "100KB").
    #[serde(default)]
    pub binary_size_tolerance: Option<String>,

    /// Build time tolerance (e.g., "5s").
    #[serde(default)]
    pub build_time_tolerance: Option<String>,

    /// Test time tolerance (e.g., "2s"). Defaults to build_time_tolerance.
    #[serde(default)]
    pub test_time_tolerance: Option<String>,

    /// Days before baseline is considered stale (0 to disable, default: 30).
    #[serde(default = "default_stale_days")]
    pub stale_days: u32,

    /// Per-package ratchet settings.
    #[serde(default)]
    pub package: HashMap<String, RatchetPackageConfig>,
}

/// Per-package ratcheting configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RatchetPackageConfig {
    /// Override coverage ratcheting for this package (None = inherit global).
    pub coverage: Option<bool>,

    /// Override escapes ratcheting for this package (None = inherit global).
    pub escapes: Option<bool>,
}

fn default_stale_days() -> u32 {
    30
}

impl RatchetConfig {
    /// Get coverage tolerance in percentage points.
    pub fn coverage_tolerance_pct(&self) -> Option<f64> {
        self.coverage_tolerance
    }

    /// Get binary size tolerance in bytes.
    pub fn binary_size_tolerance_bytes(&self) -> Option<u64> {
        self.binary_size_tolerance.as_ref().and_then(|s| parse_size(s).ok())
    }

    /// Get build time tolerance as Duration.
    pub fn build_time_tolerance_duration(&self) -> Option<Duration> {
        self.build_time_tolerance.as_ref().and_then(|s| parse_duration(s).ok())
    }

    /// Get test time tolerance as Duration (uses build_time_tolerance if not separately configured).
    pub fn test_time_tolerance_duration(&self) -> Option<Duration> {
        self.test_time_tolerance
            .as_ref()
            .and_then(|s| parse_duration(s).ok())
            .or_else(|| self.build_time_tolerance_duration())
    }

    /// Check if coverage is ratcheted for a specific package.
    ///
    /// Returns the package-specific setting if configured, otherwise the global setting.
    pub fn is_coverage_ratcheted(&self, package: &str) -> bool {
        self.package.get(package).and_then(|p| p.coverage).unwrap_or(self.coverage)
    }

    /// Check if escapes are ratcheted for a specific package.
    ///
    /// Returns the package-specific setting if configured, otherwise the global setting.
    pub fn is_escapes_ratcheted(&self, package: &str) -> bool {
        self.package.get(package).and_then(|p| p.escapes).unwrap_or(self.escapes)
    }
}

fn default_true() -> bool {
    true
}
