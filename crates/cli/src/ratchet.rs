// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ratchet enforcement and metrics comparison.

use std::collections::HashMap;
use std::time::Duration;

use crate::baseline::{
    Baseline, BaselineMetrics, BuildTimeMetrics as BaselineBuildTime,
    CoverageMetrics as BaselineCoverage, EscapesMetrics as BaselineEscapes,
    TestTimeMetrics as BaselineTestTime,
};
use crate::check::CheckOutput;
use crate::config::RatchetConfig;

/// Current metrics extracted from check results.
#[derive(Debug, Clone, Default)]
pub struct CurrentMetrics {
    pub escapes: Option<EscapesCurrent>,
    pub coverage: Option<CoverageCurrent>,
    pub binary_size: Option<HashMap<String, u64>>,
    pub build_time: Option<BuildTimeCurrent>,
    pub test_time: Option<TestTimeCurrent>,
}

/// Current coverage metrics extracted from tests output.
#[derive(Debug, Clone)]
pub struct CoverageCurrent {
    pub total: f64,
    pub by_package: HashMap<String, f64>,
}

/// Current escape metrics extracted from check output.
#[derive(Debug, Clone)]
pub struct EscapesCurrent {
    pub source: HashMap<String, usize>,
    pub test: HashMap<String, usize>,
}

/// Current build time metrics.
#[derive(Debug, Clone)]
pub struct BuildTimeCurrent {
    pub cold: Option<Duration>,
    pub hot: Option<Duration>,
}

/// Current test time metrics.
#[derive(Debug, Clone)]
pub struct TestTimeCurrent {
    pub total: Duration,
    pub avg: Duration,
    pub max: Duration,
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

        // Extract build metrics (binary size and build time)
        if let Some(build_result) = output.checks.iter().find(|c| c.name == "build")
            && let Some(ref metrics_json) = build_result.metrics
        {
            metrics.binary_size = extract_binary_size(metrics_json);
            metrics.build_time = extract_build_time(metrics_json);
        }

        // Extract test time and coverage metrics
        if let Some(tests_result) = output.checks.iter().find(|c| c.name == "tests")
            && let Some(ref metrics_json) = tests_result.metrics
        {
            metrics.test_time = extract_test_time(metrics_json);
            metrics.coverage = extract_coverage_metrics(metrics_json);
        }

        metrics
    }
}

fn extract_escapes_metrics(json: &serde_json::Value) -> Option<EscapesCurrent> {
    let source = json.get("source")?.as_object()?;
    let test = json.get("test")?.as_object()?;

    let source_map: HashMap<String, usize> =
        source.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize))).collect();

    let test_map: HashMap<String, usize> =
        test.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize))).collect();

    Some(EscapesCurrent { source: source_map, test: test_map })
}

fn extract_binary_size(json: &serde_json::Value) -> Option<HashMap<String, u64>> {
    let size = json.get("size")?.as_object()?;
    let map: HashMap<String, u64> =
        size.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n))).collect();

    if map.is_empty() { None } else { Some(map) }
}

fn extract_build_time(json: &serde_json::Value) -> Option<BuildTimeCurrent> {
    let time = json.get("time")?;

    let cold = time.get("cold").and_then(|v| v.as_f64()).map(Duration::from_secs_f64);

    let hot = time.get("hot").and_then(|v| v.as_f64()).map(Duration::from_secs_f64);

    if cold.is_none() && hot.is_none() { None } else { Some(BuildTimeCurrent { cold, hot }) }
}

/// Extract test time metrics from JSON.
///
/// Note: `avg` and `max` default to 0.0 if not present in the metrics.
/// This allows ratcheting on just `total` without requiring all fields.
/// However, be aware that missing fields will appear as "improved to 0".
fn extract_test_time(json: &serde_json::Value) -> Option<TestTimeCurrent> {
    let total = json.get("total").and_then(|v| v.as_f64())?;
    // Default to 0.0 for optional timing fields
    let avg = json.get("avg").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max = json.get("max").and_then(|v| v.as_f64()).unwrap_or(0.0);

    Some(TestTimeCurrent {
        total: Duration::from_secs_f64(total),
        avg: Duration::from_secs_f64(avg),
        max: Duration::from_secs_f64(max),
    })
}

/// Extract coverage metrics from tests check JSON.
///
/// Coverage is stored as a fraction (0.0 to 1.0) in the baseline.
/// The JSON format has coverage keyed by language (e.g., "rust": 0.82).
fn extract_coverage_metrics(json: &serde_json::Value) -> Option<CoverageCurrent> {
    let coverage = json.get("coverage")?;

    // Extract total from first language (typically "rust" or "typescript")
    let total = coverage.as_object()?.values().next()?.as_f64()?;

    // Extract per-package if available
    let by_package = json
        .get("coverage_by_package")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f))).collect())
        .unwrap_or_default();

    Some(CoverageCurrent { total, by_package })
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
    pub tolerance: f64,
    /// The allowed threshold (baseline ± tolerance).
    /// For "lower is better" metrics: max allowed = baseline + tolerance.
    /// For "higher is better" metrics: min allowed = baseline - tolerance.
    pub threshold: f64,
    pub passed: bool,
    pub improved: bool,
}

impl MetricComparison {
    /// Format the value based on metric type.
    pub fn format_value(&self, value: f64) -> String {
        format_metric_value(&self.name, value)
    }

    /// Get contextual advice for this metric failure.
    pub fn advice(&self) -> &'static str {
        if self.name.starts_with("escapes.") {
            match self.name.as_str() {
                n if n.contains("unsafe") => "Reduce unsafe blocks or add // SAFETY: comments.",
                n if n.contains("unwrap") => "Replace .unwrap() with proper error handling.",
                n if n.contains("todo") || n.contains("fixme") => {
                    "Resolve TODO/FIXME comments before merging."
                }
                _ => "Reduce escape hatch usage or update baseline with --fix.",
            }
        } else if self.name.starts_with("binary_size.") {
            "Reduce binary size: strip symbols, remove unused deps, enable LTO."
        } else if self.name.starts_with("build_time.") {
            "Reduce build time: check for new heavy deps or complex generics."
        } else if self.name.starts_with("test_time.") {
            "Reduce test time: parallelize tests or optimize slow tests."
        } else if self.name.starts_with("coverage.") {
            "Increase test coverage for changed code."
        } else {
            "Metric regressed. Clean up or update baseline with --fix."
        }
    }
}

/// A metric that improved from baseline.
#[derive(Debug, Clone)]
pub struct MetricImprovement {
    pub name: String,
    pub old_value: f64,
    pub new_value: f64,
}

/// Format a metric value based on its type (determined by name prefix).
fn format_metric_value(name: &str, value: f64) -> String {
    if name.starts_with("build_time.") || name.starts_with("test_time.") {
        format!("{:.1}s", value)
    } else if name.starts_with("coverage.") {
        format!("{:.1}%", value * 100.0)
    } else {
        format!("{}", value as i64)
    }
}

impl MetricImprovement {
    /// Format the value based on metric type.
    pub fn format_value(&self, value: f64) -> String {
        format_metric_value(&self.name, value)
    }
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
                tolerance: 0.0,               // No tolerance for counts
                threshold: base_count as f64, // Can't exceed baseline
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

    // Coverage: ratchets UP (higher is better)
    if config.coverage
        && let (Some(curr), Some(base)) = (&current.coverage, &baseline.coverage)
    {
        let tolerance = config.coverage_tolerance_pct().unwrap_or(0.0);
        let min_allowed = base.total - tolerance;

        let comparison = MetricComparison {
            name: "coverage.total".to_string(),
            current: curr.total,
            baseline: base.total,
            tolerance,
            threshold: min_allowed, // min allowed (floor)
            passed: curr.total >= min_allowed,
            improved: curr.total > base.total,
        };

        if !comparison.passed {
            passed = false;
        }
        if comparison.improved {
            improvements.push(MetricImprovement {
                name: "coverage.total".to_string(),
                old_value: base.total,
                new_value: curr.total,
            });
        }
        comparisons.push(comparison);

        // Per-package coverage comparison
        if let Some(base_by_pkg) = &base.by_package {
            let tolerance = config.coverage_tolerance_pct().unwrap_or(0.0);

            for (pkg, &base_pct) in base_by_pkg {
                // Check if this package has ratcheting enabled
                if !config.is_coverage_ratcheted(pkg) {
                    continue;
                }

                let curr_pct = curr.by_package.get(pkg).copied().unwrap_or(0.0);
                let min_allowed = base_pct - tolerance;

                let comparison = MetricComparison {
                    name: format!("coverage.{}", pkg),
                    current: curr_pct,
                    baseline: base_pct,
                    tolerance,
                    threshold: min_allowed,
                    passed: curr_pct >= min_allowed,
                    improved: curr_pct > base_pct,
                };

                if !comparison.passed {
                    passed = false;
                }
                if comparison.improved {
                    improvements.push(MetricImprovement {
                        name: format!("coverage.{}", pkg),
                        old_value: base_pct,
                        new_value: curr_pct,
                    });
                }
                comparisons.push(comparison);
            }
        }
    }

    // Binary size: ratchets down (smaller is better)
    if config.binary_size
        && let (Some(curr), Some(base)) = (&current.binary_size, &baseline.binary_size)
    {
        let tolerance = config.binary_size_tolerance_bytes().unwrap_or(0);
        for (target, &curr_size) in curr {
            let base_size = base.get(target).copied().unwrap_or(0);
            let max_allowed = base_size.saturating_add(tolerance);

            let comparison = MetricComparison {
                name: format!("binary_size.{}", target),
                current: curr_size as f64,
                baseline: base_size as f64,
                tolerance: tolerance as f64,
                threshold: max_allowed as f64,
                passed: curr_size <= max_allowed,
                improved: curr_size < base_size,
            };

            if !comparison.passed {
                passed = false;
            }
            if comparison.improved {
                improvements.push(MetricImprovement {
                    name: comparison.name.clone(),
                    old_value: base_size as f64,
                    new_value: curr_size as f64,
                });
            }
            comparisons.push(comparison);
        }
    }

    // Build time cold: ratchets down (faster is better)
    if config.build_time_cold {
        compare_timing(
            "build_time.cold",
            current.build_time.as_ref().and_then(|t| t.cold),
            baseline.build_time.as_ref().map(|t| t.cold),
            config.build_time_tolerance_duration(),
            &mut comparisons,
            &mut improvements,
            &mut passed,
        );
    }

    // Build time hot: ratchets down
    if config.build_time_hot {
        compare_timing(
            "build_time.hot",
            current.build_time.as_ref().and_then(|t| t.hot),
            baseline.build_time.as_ref().map(|t| t.hot),
            config.build_time_tolerance_duration(),
            &mut comparisons,
            &mut improvements,
            &mut passed,
        );
    }

    // Test time total: ratchets down
    if config.test_time_total {
        compare_timing(
            "test_time.total",
            current.test_time.as_ref().map(|t| t.total),
            baseline.test_time.as_ref().map(|t| t.total),
            config.test_time_tolerance_duration(),
            &mut comparisons,
            &mut improvements,
            &mut passed,
        );
    }

    // Test time avg: ratchets down
    if config.test_time_avg {
        compare_timing(
            "test_time.avg",
            current.test_time.as_ref().map(|t| t.avg),
            baseline.test_time.as_ref().map(|t| t.avg),
            config.test_time_tolerance_duration(),
            &mut comparisons,
            &mut improvements,
            &mut passed,
        );
    }

    // Test time max: ratchets down
    if config.test_time_max {
        compare_timing(
            "test_time.max",
            current.test_time.as_ref().map(|t| t.max),
            baseline.test_time.as_ref().map(|t| t.max),
            config.test_time_tolerance_duration(),
            &mut comparisons,
            &mut improvements,
            &mut passed,
        );
    }

    RatchetResult { passed, comparisons, improvements }
}

/// Compare a timing metric against baseline with tolerance.
fn compare_timing(
    name: &str,
    current: Option<Duration>,
    baseline: Option<f64>,
    tolerance: Option<Duration>,
    comparisons: &mut Vec<MetricComparison>,
    improvements: &mut Vec<MetricImprovement>,
    passed: &mut bool,
) {
    if let (Some(curr), Some(base)) = (current, baseline) {
        let curr_secs = curr.as_secs_f64();
        let tolerance_secs = tolerance.map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let max_allowed = base + tolerance_secs;

        let comparison = MetricComparison {
            name: name.to_string(),
            current: curr_secs,
            baseline: base,
            tolerance: tolerance_secs,
            threshold: max_allowed,
            passed: curr_secs <= max_allowed,
            improved: curr_secs < base,
        };

        if !comparison.passed {
            *passed = false;
        }
        if comparison.improved {
            improvements.push(MetricImprovement {
                name: name.to_string(),
                old_value: base,
                new_value: curr_secs,
            });
        }
        comparisons.push(comparison);
    }
}

/// Update baseline with current metrics.
pub fn update_baseline(baseline: &mut Baseline, current: &CurrentMetrics) {
    // Update escapes metrics
    if let Some(curr_escapes) = &current.escapes {
        let base_escapes = baseline
            .metrics
            .escapes
            .get_or_insert_with(|| BaselineEscapes { source: HashMap::new(), test: None });

        // Update all source counts (baseline is always current snapshot)
        for (pattern, &count) in &curr_escapes.source {
            base_escapes.source.insert(pattern.clone(), count);
        }

        // Optionally track test counts
        if !curr_escapes.test.is_empty() {
            base_escapes.test = Some(curr_escapes.test.clone());
        }
    }

    // Update coverage metrics
    if let Some(curr_cov) = &current.coverage {
        baseline.metrics.coverage = Some(BaselineCoverage {
            total: curr_cov.total,
            by_package: if curr_cov.by_package.is_empty() {
                None
            } else {
                Some(curr_cov.by_package.clone())
            },
        });
    }

    // Update binary size metrics
    if let Some(curr_sizes) = &current.binary_size {
        let base_sizes = baseline.metrics.binary_size.get_or_insert_with(HashMap::new);
        for (target, &size) in curr_sizes {
            base_sizes.insert(target.clone(), size);
        }
    }

    // Update build time metrics
    if let Some(curr_time) = &current.build_time {
        let base_time =
            baseline.metrics.build_time.get_or_insert(BaselineBuildTime { cold: 0.0, hot: 0.0 });
        if let Some(cold) = curr_time.cold {
            base_time.cold = cold.as_secs_f64();
        }
        if let Some(hot) = curr_time.hot {
            base_time.hot = hot.as_secs_f64();
        }
    }

    // Update test time metrics
    if let Some(curr_time) = &current.test_time {
        baseline.metrics.test_time = Some(BaselineTestTime {
            total: curr_time.total.as_secs_f64(),
            avg: curr_time.avg.as_secs_f64(),
            max: curr_time.max.as_secs_f64(),
        });
    }

    // Update timestamp
    baseline.touch();
}

#[cfg(test)]
#[path = "ratchet_tests.rs"]
mod tests;
