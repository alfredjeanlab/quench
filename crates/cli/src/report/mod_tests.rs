// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::test_support::{AllChecks, ExcludeChecks, create_test_baseline};
use super::*;

/// Check if a metric is present in filtered metrics.
fn metric_is_present(filtered: &FilteredMetrics<'_>, metric: &str) -> bool {
    match metric {
        "coverage" => filtered.coverage().is_some(),
        "escapes" => filtered.escapes().is_some(),
        "build_time" => filtered.build_time().is_some(),
        "binary_size" => filtered.binary_size().is_some(),
        "test_time" => filtered.test_time().is_some(),
        _ => panic!("Unknown metric: {}", metric),
    }
}

/// Assert that excluding certain checks results in expected metric presence.
fn assert_filter_excludes(
    baseline: &Baseline,
    excluded: Vec<&'static str>,
    should_be_none: &[&str],
    should_be_some: &[&str],
) {
    let filter = ExcludeChecks(excluded.clone());
    let filtered = FilteredMetrics::new(baseline, &filter);

    for metric in should_be_none {
        assert!(
            !metric_is_present(&filtered, metric),
            "{} should be None when excluding {:?}",
            metric,
            excluded
        );
    }
    for metric in should_be_some {
        assert!(
            metric_is_present(&filtered, metric),
            "{} should be Some when excluding {:?}",
            metric,
            excluded
        );
    }
}

// --- human_bytes tests ---

#[test]
fn human_bytes_formats_bytes() {
    assert_eq!(human_bytes(0), "0 B");
    assert_eq!(human_bytes(500), "500 B");
    assert_eq!(human_bytes(1023), "1023 B");
}

#[test]
fn human_bytes_formats_kilobytes() {
    assert_eq!(human_bytes(1024), "1.0 KB");
    assert_eq!(human_bytes(1536), "1.5 KB");
    assert_eq!(human_bytes(10240), "10.0 KB");
}

#[test]
fn human_bytes_formats_megabytes() {
    assert_eq!(human_bytes(1048576), "1.0 MB");
    assert_eq!(human_bytes(1572864), "1.5 MB");
    assert_eq!(human_bytes(10485760), "10.0 MB");
}

#[test]
fn human_bytes_boundary_kb() {
    // Just under 1KB
    assert_eq!(human_bytes(1023), "1023 B");
    // Exactly 1KB
    assert_eq!(human_bytes(1024), "1.0 KB");
}

#[test]
fn human_bytes_boundary_mb() {
    // Just under 1MB
    assert_eq!(human_bytes(1048575), "1024.0 KB");
    // Exactly 1MB
    assert_eq!(human_bytes(1048576), "1.0 MB");
}

// --- FilteredMetrics tests ---

#[test]
fn filtered_metrics_includes_all_with_all_checks() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    assert!(filtered.coverage().is_some());
    assert!(filtered.escapes().is_some());
    assert!(filtered.build_time().is_some());
    assert!(filtered.binary_size().is_some());
    assert!(filtered.test_time().is_some());
}

#[test]
fn filtered_metrics_excludes_tests() {
    assert_filter_excludes(
        &create_test_baseline(),
        vec!["tests"],
        &["coverage", "test_time"],
        &["escapes", "build_time", "binary_size"],
    );
}

#[test]
fn filtered_metrics_excludes_build() {
    assert_filter_excludes(
        &create_test_baseline(),
        vec!["build"],
        &["build_time", "binary_size"],
        &["coverage", "escapes", "test_time"],
    );
}

#[test]
fn filtered_metrics_excludes_escapes() {
    assert_filter_excludes(
        &create_test_baseline(),
        vec!["escapes"],
        &["escapes"],
        &["coverage", "build_time", "binary_size", "test_time"],
    );
}

#[test]
fn filtered_metrics_count_all() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    // coverage (1) + escapes (2 patterns) + build_time (2) + binary_size (1) + test_time (1) = 7
    assert_eq!(filtered.count(), 7);
}

#[test]
fn filtered_metrics_count_with_exclusions() {
    let baseline = create_test_baseline();
    let filter = ExcludeChecks(vec!["tests"]);
    let filtered = FilteredMetrics::new(&baseline, &filter);

    // escapes (2) + build_time (2) + binary_size (1) = 5
    assert_eq!(filtered.count(), 5);
}

#[test]
fn filtered_metrics_handles_empty_baseline() {
    let baseline = Baseline::default();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    assert!(filtered.coverage().is_none());
    assert!(filtered.escapes().is_none());
    assert!(filtered.build_time().is_none());
    assert!(filtered.binary_size().is_none());
    assert!(filtered.test_time().is_none());
    assert_eq!(filtered.count(), 0);
}

// --- Sorted helpers tests ---

#[test]
fn sorted_escapes_returns_alphabetical_order() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    let escapes = filtered.sorted_escapes().unwrap();
    // "expect" comes before "unwrap"
    assert_eq!(escapes.len(), 2);
    assert_eq!(escapes[0], ("expect", 5_usize));
    assert_eq!(escapes[1], ("unwrap", 10_usize));
}

#[test]
fn sorted_escapes_returns_none_when_filtered() {
    let baseline = create_test_baseline();
    let filter = ExcludeChecks(vec!["escapes"]);
    let filtered = FilteredMetrics::new(&baseline, &filter);

    assert!(filtered.sorted_escapes().is_none());
}

#[test]
fn sorted_test_escapes_returns_none_when_no_test_escapes() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    // The test baseline has no test escapes
    assert!(filtered.sorted_test_escapes().is_none());
}

#[test]
fn sorted_test_escapes_returns_sorted_when_present() {
    use crate::baseline::{BaselineMetrics, EscapesMetrics};

    let baseline = Baseline {
        metrics: BaselineMetrics {
            escapes: Some(EscapesMetrics {
                source: [("unwrap".to_string(), 1_usize)].into_iter().collect(),
                test: Some(
                    [("zebra".to_string(), 1_usize), ("alpha".to_string(), 2_usize)]
                        .into_iter()
                        .collect(),
                ),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let filtered = FilteredMetrics::new(&baseline, &AllChecks);
    let test_escapes = filtered.sorted_test_escapes().unwrap();

    assert_eq!(test_escapes.len(), 2);
    assert_eq!(test_escapes[0], ("alpha", 2_usize));
    assert_eq!(test_escapes[1], ("zebra", 1_usize));
}

#[test]
fn sorted_package_coverage_returns_none_when_no_packages() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    // The test baseline has no package coverage
    assert!(filtered.sorted_package_coverage().is_none());
}

#[test]
fn sorted_package_coverage_returns_sorted_when_present() {
    use crate::baseline::{BaselineMetrics, CoverageMetrics};

    let baseline = Baseline {
        metrics: BaselineMetrics {
            coverage: Some(CoverageMetrics {
                total: 80.0,
                by_package: Some(
                    [("zebra".to_string(), 70.0), ("alpha".to_string(), 90.0)]
                        .into_iter()
                        .collect(),
                ),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    let filtered = FilteredMetrics::new(&baseline, &AllChecks);
    let packages = filtered.sorted_package_coverage().unwrap();

    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0], ("alpha", 90.0));
    assert_eq!(packages[1], ("zebra", 70.0));
}

#[test]
fn sorted_binary_sizes_returns_alphabetical_order() {
    let baseline = create_test_baseline();
    let filtered = FilteredMetrics::new(&baseline, &AllChecks);

    let sizes = filtered.sorted_binary_sizes().unwrap();
    assert_eq!(sizes.len(), 1);
    assert_eq!(sizes[0], ("quench", 5_242_880));
}

#[test]
fn sorted_binary_sizes_returns_none_when_filtered() {
    let baseline = create_test_baseline();
    let filter = ExcludeChecks(vec!["build"]);
    let filtered = FilteredMetrics::new(&baseline, &filter);

    assert!(filtered.sorted_binary_sizes().is_none());
}
