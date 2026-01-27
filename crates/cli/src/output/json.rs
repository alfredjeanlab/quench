// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSON output formatter.
//!
//! Produces output conforming to docs/specs/output.schema.json.
//! JSON is buffered and written at the end (not streamed).

use std::io::Write;

use chrono::Utc;
use serde::Serialize;

use crate::check::{CheckOutput, CheckResult};
use crate::ratchet::{MetricComparison, MetricImprovement, RatchetResult};
use crate::timing::TimingInfo;

/// Ratchet comparison result for JSON output.
#[derive(Debug, Serialize)]
pub struct RatchetOutput {
    pub passed: bool,
    pub comparisons: Vec<MetricComparisonOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub improvements: Vec<MetricImprovementOutput>,
}

/// Individual metric comparison for JSON output.
#[derive(Debug, Serialize)]
pub struct MetricComparisonOutput {
    pub name: String,
    pub current: f64,
    pub baseline: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    /// Minimum allowed value (for floor metrics like coverage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_allowed: Option<f64>,
    /// Maximum allowed value (for ceiling metrics like size, escapes, timing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_allowed: Option<f64>,
    pub passed: bool,
    pub improved: bool,
}

/// Metric improvement for JSON output.
#[derive(Debug, Serialize)]
pub struct MetricImprovementOutput {
    pub name: String,
    pub old_value: f64,
    pub new_value: f64,
}

impl From<&RatchetResult> for RatchetOutput {
    fn from(result: &RatchetResult) -> Self {
        Self {
            passed: result.passed,
            comparisons: result.comparisons.iter().map(Into::into).collect(),
            improvements: result.improvements.iter().map(Into::into).collect(),
        }
    }
}

impl From<&MetricComparison> for MetricComparisonOutput {
    fn from(comp: &MetricComparison) -> Self {
        // Coverage uses min_allowed (floor), others use max_allowed (ceiling)
        let is_coverage = comp.name.starts_with("coverage.");
        let tolerance = if comp.tolerance > 0.0 {
            Some(comp.tolerance)
        } else {
            None
        };

        Self {
            name: comp.name.clone(),
            current: comp.current,
            baseline: comp.baseline,
            tolerance,
            min_allowed: if is_coverage {
                Some(comp.threshold)
            } else {
                None
            },
            max_allowed: if !is_coverage {
                Some(comp.threshold)
            } else {
                None
            },
            passed: comp.passed,
            improved: comp.improved,
        }
    }
}

impl From<&MetricImprovement> for MetricImprovementOutput {
    fn from(imp: &MetricImprovement) -> Self {
        Self {
            name: imp.name.clone(),
            old_value: imp.old_value,
            new_value: imp.new_value,
        }
    }
}

/// JSON output formatter.
pub struct JsonFormatter<W: Write> {
    writer: W,
}

/// Combined output with optional ratchet and timing results.
#[derive(Debug, Serialize)]
struct CombinedOutput<'a> {
    timestamp: &'a str,
    passed: bool,
    checks: &'a [CheckResult],
    #[serde(skip_serializing_if = "Option::is_none")]
    ratchet: Option<RatchetOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timing: Option<&'a TimingInfo>,
}

impl<W: Write> JsonFormatter<W> {
    /// Create a new JSON formatter.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write the complete JSON output.
    pub fn write(&mut self, output: &CheckOutput) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(output).map_err(std::io::Error::other)?;
        writeln!(self.writer, "{}", json)
    }

    /// Write JSON output with optional ratchet results (no timing).
    pub fn write_with_ratchet(
        &mut self,
        output: &CheckOutput,
        ratchet: Option<&RatchetResult>,
    ) -> std::io::Result<()> {
        self.write_with_timing(output, ratchet, None)
    }

    /// Write JSON output with optional ratchet and timing.
    pub fn write_with_timing(
        &mut self,
        output: &CheckOutput,
        ratchet: Option<&RatchetResult>,
        timing: Option<&TimingInfo>,
    ) -> std::io::Result<()> {
        let combined = CombinedOutput {
            timestamp: &output.timestamp,
            passed: output.passed && ratchet.as_ref().is_none_or(|r| r.passed),
            checks: &output.checks,
            ratchet: ratchet.map(Into::into),
            timing,
        };
        let json = serde_json::to_string_pretty(&combined).map_err(std::io::Error::other)?;
        writeln!(self.writer, "{}", json)
    }
}

/// Create CheckOutput with current timestamp.
pub fn create_output(checks: Vec<CheckResult>) -> CheckOutput {
    CheckOutput::new(
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checks,
    )
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
