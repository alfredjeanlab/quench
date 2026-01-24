// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ratcheting configuration.

use serde::Deserialize;

use super::CheckLevel;

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
}

fn default_true() -> bool {
    true
}
