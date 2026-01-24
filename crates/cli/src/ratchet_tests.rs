// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use serde_json::json;

use super::*;
use crate::baseline::{BaselineMetrics, EscapesMetrics as BaselineEscapes};
use crate::check::{CheckOutput, CheckResult};
use crate::config::{CheckLevel, RatchetConfig};

fn make_config(escapes: bool) -> RatchetConfig {
    RatchetConfig {
        check: CheckLevel::Error,
        escapes,
        ..Default::default()
    }
}

fn make_baseline_metrics(escapes: HashMap<String, usize>) -> BaselineMetrics {
    BaselineMetrics {
        escapes: Some(BaselineEscapes {
            source: escapes,
            test: None,
        }),
        ..Default::default()
    }
}

fn make_current_metrics(escapes: HashMap<String, usize>) -> CurrentMetrics {
    CurrentMetrics {
        escapes: Some(EscapesCurrent {
            source: escapes,
            test: HashMap::new(),
        }),
        ..Default::default()
    }
}

#[test]
fn compare_same_values_passes() {
    let config = make_config(true);
    let baseline = make_baseline_metrics(HashMap::from([("unsafe".to_string(), 5)]));
    let current = make_current_metrics(HashMap::from([("unsafe".to_string(), 5)]));

    let result = compare(&current, &baseline, &config);

    assert!(result.passed);
    assert!(result.improvements.is_empty());
    assert_eq!(result.comparisons.len(), 1);
    assert!(result.comparisons[0].passed);
    assert!(!result.comparisons[0].improved);
}

#[test]
fn compare_regression_fails() {
    let config = make_config(true);
    let baseline = make_baseline_metrics(HashMap::from([("unsafe".to_string(), 5)]));
    let current = make_current_metrics(HashMap::from([("unsafe".to_string(), 7)]));

    let result = compare(&current, &baseline, &config);

    assert!(!result.passed);
    assert!(result.improvements.is_empty());
    assert_eq!(result.comparisons.len(), 1);
    assert!(!result.comparisons[0].passed);
}

#[test]
fn compare_improvement_passes_and_tracks() {
    let config = make_config(true);
    let baseline = make_baseline_metrics(HashMap::from([("unsafe".to_string(), 10)]));
    let current = make_current_metrics(HashMap::from([("unsafe".to_string(), 3)]));

    let result = compare(&current, &baseline, &config);

    assert!(result.passed);
    assert_eq!(result.improvements.len(), 1);
    assert_eq!(result.improvements[0].name, "escapes.unsafe");
    assert_eq!(result.improvements[0].old_value, 10.0);
    assert_eq!(result.improvements[0].new_value, 3.0);
    assert!(result.comparisons[0].passed);
    assert!(result.comparisons[0].improved);
}

#[test]
fn compare_escapes_disabled_skips() {
    let config = make_config(false); // escapes = false
    let baseline = make_baseline_metrics(HashMap::from([("unsafe".to_string(), 5)]));
    let current = make_current_metrics(HashMap::from([("unsafe".to_string(), 100)])); // regression

    let result = compare(&current, &baseline, &config);

    // Should pass because escapes checking is disabled
    assert!(result.passed);
    assert!(result.comparisons.is_empty());
}

#[test]
fn compare_new_pattern_against_empty_baseline() {
    let config = make_config(true);
    let baseline = make_baseline_metrics(HashMap::new()); // no patterns
    let current = make_current_metrics(HashMap::from([("unwrap".to_string(), 3)]));

    let result = compare(&current, &baseline, &config);

    // New pattern with non-zero count is a regression from 0
    assert!(!result.passed);
    assert_eq!(result.comparisons.len(), 1);
    assert_eq!(result.comparisons[0].baseline, 0.0);
    assert_eq!(result.comparisons[0].current, 3.0);
}

#[test]
fn extract_metrics_from_check_output() {
    let metrics_json = json!({
        "source": { "unsafe": 5, "unwrap": 3 },
        "test": { "unsafe": 10 }
    });

    let check_result = CheckResult::passed("escapes").with_metrics(metrics_json);
    let output = CheckOutput::new("2026-01-20T00:00:00Z".to_string(), vec![check_result]);

    let current = CurrentMetrics::from_output(&output);

    assert!(current.escapes.is_some());
    let escapes = current.escapes.unwrap();
    assert_eq!(escapes.source.get("unsafe"), Some(&5));
    assert_eq!(escapes.source.get("unwrap"), Some(&3));
    assert_eq!(escapes.test.get("unsafe"), Some(&10));
}

#[test]
fn extract_metrics_no_escapes_check() {
    let check_result = CheckResult::passed("cloc");
    let output = CheckOutput::new("2026-01-20T00:00:00Z".to_string(), vec![check_result]);

    let current = CurrentMetrics::from_output(&output);

    assert!(current.escapes.is_none());
}

#[test]
fn update_baseline_with_current() {
    let mut baseline = Baseline::new();
    let current = make_current_metrics(HashMap::from([
        ("unsafe".to_string(), 5),
        ("unwrap".to_string(), 3),
    ]));

    update_baseline(&mut baseline, &current);

    assert!(baseline.metrics.escapes.is_some());
    let escapes = baseline.metrics.escapes.unwrap();
    assert_eq!(escapes.source.get("unsafe"), Some(&5));
    assert_eq!(escapes.source.get("unwrap"), Some(&3));
}

#[test]
fn update_baseline_replaces_values() {
    let mut baseline = Baseline::new();
    baseline.metrics.escapes = Some(BaselineEscapes {
        source: HashMap::from([("unsafe".to_string(), 10)]),
        test: None,
    });

    let current = make_current_metrics(HashMap::from([("unsafe".to_string(), 3)]));

    update_baseline(&mut baseline, &current);

    let escapes = baseline.metrics.escapes.unwrap();
    assert_eq!(escapes.source.get("unsafe"), Some(&3));
}

#[test]
fn multiple_patterns_mixed_results() {
    let config = make_config(true);
    let baseline = make_baseline_metrics(HashMap::from([
        ("unsafe".to_string(), 5),
        ("unwrap".to_string(), 10),
    ]));
    let current = make_current_metrics(HashMap::from([
        ("unsafe".to_string(), 7), // regression
        ("unwrap".to_string(), 5), // improvement
    ]));

    let result = compare(&current, &baseline, &config);

    // Overall fails because of unsafe regression
    assert!(!result.passed);

    // But unwrap should be tracked as an improvement
    assert_eq!(result.improvements.len(), 1);
    assert_eq!(result.improvements[0].name, "escapes.unwrap");
}

// =============================================================================
// Performance Metrics Tests
// =============================================================================

use crate::baseline::BuildTimeMetrics as BaselineBuildTime;
use std::time::Duration;

fn make_binary_size_config(tolerance: Option<&str>) -> RatchetConfig {
    RatchetConfig {
        check: CheckLevel::Error,
        binary_size: true,
        binary_size_tolerance: tolerance.map(String::from),
        ..Default::default()
    }
}

#[test]
fn binary_size_regression_fails() {
    let config = make_binary_size_config(None);
    let baseline = BaselineMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_000_000)])),
        ..Default::default()
    };
    let current = CurrentMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_500_000)])),
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(!result.passed);
    assert_eq!(result.comparisons.len(), 1);
    assert!(!result.comparisons[0].passed);
    assert_eq!(result.comparisons[0].current, 1_500_000.0);
    assert_eq!(result.comparisons[0].baseline, 1_000_000.0);
}

#[test]
fn binary_size_within_tolerance_passes() {
    let config = make_binary_size_config(Some("100KB"));
    let baseline = BaselineMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_000_000)])),
        ..Default::default()
    };
    let current = CurrentMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_050_000)])),
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(result.passed);
    assert_eq!(result.comparisons.len(), 1);
    assert!(result.comparisons[0].passed);
}

#[test]
fn binary_size_exceeds_tolerance_fails() {
    let config = make_binary_size_config(Some("100KB")); // 102,400 bytes tolerance
    let baseline = BaselineMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_000_000)])),
        ..Default::default()
    };
    let current = CurrentMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_200_000)])), // +200KB
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(!result.passed);
}

#[test]
fn binary_size_improvement_tracked() {
    let config = make_binary_size_config(None);
    let baseline = BaselineMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 1_000_000)])),
        ..Default::default()
    };
    let current = CurrentMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 800_000)])),
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(result.passed);
    assert_eq!(result.improvements.len(), 1);
    assert_eq!(result.improvements[0].name, "binary_size.myapp");
}

fn make_build_time_config(cold: bool, hot: bool, tolerance: Option<&str>) -> RatchetConfig {
    RatchetConfig {
        check: CheckLevel::Error,
        build_time_cold: cold,
        build_time_hot: hot,
        build_time_tolerance: tolerance.map(String::from),
        ..Default::default()
    }
}

#[test]
fn build_time_cold_regression_fails() {
    let config = make_build_time_config(true, false, None);
    let baseline = BaselineMetrics {
        build_time: Some(BaselineBuildTime {
            cold: 10.0,
            hot: 5.0,
        }),
        ..Default::default()
    };
    let current = CurrentMetrics {
        build_time: Some(BuildTimeCurrent {
            cold: Some(Duration::from_secs(15)),
            hot: None,
        }),
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(!result.passed);
}

#[test]
fn build_time_within_tolerance_passes() {
    let config = make_build_time_config(true, false, Some("5s"));
    let baseline = BaselineMetrics {
        build_time: Some(BaselineBuildTime {
            cold: 10.0,
            hot: 5.0,
        }),
        ..Default::default()
    };
    let current = CurrentMetrics {
        build_time: Some(BuildTimeCurrent {
            cold: Some(Duration::from_secs(12)), // +2s, within 5s tolerance
            hot: None,
        }),
        ..Default::default()
    };

    let result = compare(&current, &baseline, &config);

    assert!(result.passed);
}

#[test]
fn extract_build_metrics() {
    let metrics_json = json!({
        "size": { "myapp": 1000000 },
        "time": { "cold": 10.5, "hot": 2.3 }
    });

    let check_result = CheckResult::passed("build").with_metrics(metrics_json);
    let output = CheckOutput::new("2026-01-20T00:00:00Z".to_string(), vec![check_result]);

    let current = CurrentMetrics::from_output(&output);

    assert!(current.binary_size.is_some());
    assert_eq!(
        current.binary_size.as_ref().unwrap().get("myapp"),
        Some(&1000000)
    );

    assert!(current.build_time.is_some());
    let build_time = current.build_time.as_ref().unwrap();
    assert!(build_time.cold.is_some());
    assert!(build_time.hot.is_some());
}

#[test]
fn extract_test_time_metrics() {
    let metrics_json = json!({
        "total": 30.5,
        "avg": 0.5,
        "max": 2.0
    });

    let check_result = CheckResult::passed("tests").with_metrics(metrics_json);
    let output = CheckOutput::new("2026-01-20T00:00:00Z".to_string(), vec![check_result]);

    let current = CurrentMetrics::from_output(&output);

    assert!(current.test_time.is_some());
    let test_time = current.test_time.as_ref().unwrap();
    assert_eq!(test_time.total.as_secs_f64(), 30.5);
    assert_eq!(test_time.avg.as_secs_f64(), 0.5);
    assert_eq!(test_time.max.as_secs_f64(), 2.0);
}

#[test]
fn update_baseline_with_perf_metrics() {
    let mut baseline = Baseline::new();
    let current = CurrentMetrics {
        binary_size: Some(HashMap::from([("myapp".to_string(), 800_000)])),
        build_time: Some(BuildTimeCurrent {
            cold: Some(Duration::from_secs(10)),
            hot: Some(Duration::from_secs(2)),
        }),
        test_time: Some(TestTimeCurrent {
            total: Duration::from_secs(30),
            avg: Duration::from_millis(500),
            max: Duration::from_secs(2),
        }),
        ..Default::default()
    };

    update_baseline(&mut baseline, &current);

    assert!(baseline.metrics.binary_size.is_some());
    assert_eq!(
        baseline.metrics.binary_size.as_ref().unwrap().get("myapp"),
        Some(&800_000)
    );

    assert!(baseline.metrics.build_time.is_some());
    let build_time = baseline.metrics.build_time.as_ref().unwrap();
    assert_eq!(build_time.cold, 10.0);
    assert_eq!(build_time.hot, 2.0);

    assert!(baseline.metrics.test_time.is_some());
    let test_time = baseline.metrics.test_time.as_ref().unwrap();
    assert_eq!(test_time.total, 30.0);
}
