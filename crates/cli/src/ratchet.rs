// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ratchet enforcement and metrics comparison.

use std::collections::HashMap;

use crate::baseline::{Baseline, BaselineMetrics, EscapesMetrics as BaselineEscapes};
use crate::check::CheckOutput;
use crate::config::RatchetConfig;

/// Current metrics extracted from check results.
#[derive(Debug, Clone, Default)]
pub struct CurrentMetrics {
    pub escapes: Option<EscapesCurrent>,
    // Coverage and timing metrics added in future phases
}

/// Current escape metrics extracted from check output.
#[derive(Debug, Clone)]
pub struct EscapesCurrent {
    pub source: HashMap<String, usize>,
    pub test: HashMap<String, usize>,
}

impl CurrentMetrics {
    /// Extract metrics from check output.
    pub fn from_output(output: &CheckOutput) -> Self {
        let mut metrics = Self::default();

        // Find escapes check result and extract metrics
        if let Some(escapes_result) = output.checks.iter().find(|c| c.name == "escapes")
            && let Some(ref metrics_json) = escapes_result.metrics
        {
            metrics.escapes = extract_escapes_metrics(metrics_json);
        }

        metrics
    }
}

fn extract_escapes_metrics(json: &serde_json::Value) -> Option<EscapesCurrent> {
    let source = json.get("source")?.as_object()?;
    let test = json.get("test")?.as_object()?;

    let source_map: HashMap<String, usize> = source
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize)))
        .collect();

    let test_map: HashMap<String, usize> = test
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize)))
        .collect();

    Some(EscapesCurrent {
        source: source_map,
        test: test_map,
    })
}

/// Result of ratchet comparison.
#[derive(Debug, Clone)]
pub struct RatchetResult {
    /// Whether all ratcheted metrics pass.
    pub passed: bool,

    /// Individual metric comparison results.
    pub comparisons: Vec<MetricComparison>,

    /// Metrics that improved (for baseline update).
    pub improvements: Vec<MetricImprovement>,
}

/// Comparison of a single metric.
#[derive(Debug, Clone)]
pub struct MetricComparison {
    pub name: String,
    pub current: f64,
    pub baseline: f64,
    // KEEP UNTIL: coverage/binary_size/build_time ratchet metrics are implemented
    #[allow(dead_code)]
    pub tolerance: f64,
    pub min_allowed: f64,
    pub passed: bool,
    pub improved: bool,
}

/// A metric that improved from baseline.
#[derive(Debug, Clone)]
pub struct MetricImprovement {
    pub name: String,
    pub old_value: f64,
    pub new_value: f64,
}

/// Compare current metrics against baseline using ratchet config.
pub fn compare(
    current: &CurrentMetrics,
    baseline: &BaselineMetrics,
    config: &RatchetConfig,
) -> RatchetResult {
    let mut comparisons = Vec::new();
    let mut improvements = Vec::new();
    let mut passed = true;

    // Compare escapes if enabled
    if config.escapes
        && let (Some(curr), Some(base)) = (&current.escapes, &baseline.escapes)
    {
        for (pattern, &curr_count) in &curr.source {
            let base_count = base.source.get(pattern).copied().unwrap_or(0);

            // Escapes ratchet down (lower is better)
            let comparison = MetricComparison {
                name: format!("escapes.{}", pattern),
                current: curr_count as f64,
                baseline: base_count as f64,
                tolerance: 0.0,                 // No tolerance for counts
                min_allowed: base_count as f64, // Can't exceed baseline
                passed: curr_count <= base_count,
                improved: curr_count < base_count,
            };

            if !comparison.passed {
                passed = false;
            }

            if comparison.improved {
                improvements.push(MetricImprovement {
                    name: format!("escapes.{}", pattern),
                    old_value: base_count as f64,
                    new_value: curr_count as f64,
                });
            }

            comparisons.push(comparison);
        }
    }

    // Coverage comparison would go here (ratchets up - higher is better)
    // Binary size comparison would go here (ratchets down - smaller is better)
    // Build/test time comparisons would go here (ratchet down - faster is better)

    RatchetResult {
        passed,
        comparisons,
        improvements,
    }
}

/// Update baseline with current metrics where improved.
pub fn update_baseline(
    baseline: &mut Baseline,
    current: &CurrentMetrics,
    _improvements: &[MetricImprovement],
) {
    // Update escapes metrics
    if let Some(curr_escapes) = &current.escapes {
        let base_escapes = baseline
            .metrics
            .escapes
            .get_or_insert_with(|| BaselineEscapes {
                source: HashMap::new(),
                test: None,
            });

        // Update all source counts (baseline is always current snapshot)
        for (pattern, &count) in &curr_escapes.source {
            base_escapes.source.insert(pattern.clone(), count);
        }

        // Optionally track test counts
        if !curr_escapes.test.is_empty() {
            base_escapes.test = Some(curr_escapes.test.clone());
        }
    }

    // Update timestamp
    baseline.touch();
}

#[cfg(test)]
#[path = "ratchet_tests.rs"]
mod tests;
