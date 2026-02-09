// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use std::time::Duration;

#[test]
fn test_result_passed() {
    let result = TestResult::passed("test_one", Duration::from_millis(100));
    assert_eq!(result.name, "test_one");
    assert!(result.passed);
    assert_eq!(result.duration, Duration::from_millis(100));
}

#[test]
fn test_result_failed() {
    let result = TestResult::failed("test_two", Duration::from_millis(50));
    assert_eq!(result.name, "test_two");
    assert!(!result.passed);
}

#[test]
fn run_result_passed() {
    let result = TestRunResult::passed(Duration::from_secs(1));
    assert!(result.passed);
    assert!(!result.skipped);
    assert!(result.error.is_none());
    assert_eq!(result.total_time, Duration::from_secs(1));
}

#[test]
fn run_result_failed() {
    let result = TestRunResult::failed(Duration::from_secs(2), "some tests failed");
    assert!(!result.passed);
    assert!(!result.skipped);
    assert_eq!(result.error, Some("some tests failed".to_string()));
}

#[test]
fn run_result_skipped() {
    let result = TestRunResult::skipped("runner not installed");
    assert!(!result.passed);
    assert!(result.skipped);
    assert_eq!(result.error, Some("runner not installed".to_string()));
    assert_eq!(result.total_time, Duration::ZERO);
}

#[test]
fn run_result_with_tests() {
    let tests = vec![
        TestResult::passed("test_one", Duration::from_millis(100)),
        TestResult::passed("test_two", Duration::from_millis(200)),
    ];
    let result = TestRunResult::passed(Duration::from_millis(300)).with_tests(tests);
    assert_eq!(result.test_count(), 2);
    assert!(result.passed);
}

#[test]
fn run_result_with_tests_updates_passed() {
    let tests = vec![
        TestResult::passed("test_one", Duration::from_millis(100)),
        TestResult::failed("test_two", Duration::from_millis(200)),
    ];
    // Even though we called passed(), with_tests should update based on actual results
    let result = TestRunResult::passed(Duration::from_millis(300)).with_tests(tests);
    assert!(!result.passed);
    assert_eq!(result.passed_count(), 1);
    assert_eq!(result.failed_count(), 1);
}

#[test]
fn run_result_avg_duration() {
    let tests = vec![
        TestResult::passed("test_one", Duration::from_millis(100)),
        TestResult::passed("test_two", Duration::from_millis(200)),
        TestResult::passed("test_three", Duration::from_millis(300)),
    ];
    let result = TestRunResult::passed(Duration::from_millis(600)).with_tests(tests);
    assert_eq!(result.avg_duration(), Some(Duration::from_millis(200)));
}

#[test]
fn run_result_avg_duration_empty() {
    let result = TestRunResult::passed(Duration::from_millis(100));
    assert!(result.avg_duration().is_none());
}

#[test]
fn run_result_slowest_test() {
    let tests = vec![
        TestResult::passed("fast", Duration::from_millis(50)),
        TestResult::passed("slow", Duration::from_millis(500)),
        TestResult::passed("medium", Duration::from_millis(200)),
    ];
    let result = TestRunResult::passed(Duration::from_millis(750)).with_tests(tests);
    let slowest = result.slowest_test().unwrap();
    assert_eq!(slowest.name, "slow");
    assert_eq!(slowest.duration, Duration::from_millis(500));
}

#[test]
fn run_result_slowest_test_empty() {
    let result = TestRunResult::passed(Duration::from_millis(100));
    assert!(result.slowest_test().is_none());
}

#[test]
fn run_result_with_coverage() {
    let mut coverage = HashMap::new();
    coverage.insert("rust".to_string(), 82.5);
    coverage.insert("python".to_string(), 71.0);

    let result = TestRunResult::passed(Duration::from_secs(1)).with_coverage(coverage);
    let cov = result.coverage.as_ref().unwrap();
    assert_eq!(cov.get("rust"), Some(&82.5));
    assert_eq!(cov.get("python"), Some(&71.0));
}

#[test]
fn test_result_skipped() {
    let result = TestResult::skipped("test_ignored");
    assert_eq!(result.name, "test_ignored");
    assert!(result.passed);
    assert!(result.skipped);
    assert_eq!(result.duration, Duration::ZERO);
}

#[test]
fn run_result_skipped_count() {
    let tests = vec![
        TestResult::passed("test_one", Duration::from_millis(100)),
        TestResult::skipped("test_two"),
        TestResult::failed("test_three", Duration::from_millis(200)),
        TestResult::skipped("test_four"),
    ];
    let result = TestRunResult::passed(Duration::from_millis(300)).with_tests(tests);
    assert_eq!(result.test_count(), 4);
    assert_eq!(result.passed_count(), 1);
    assert_eq!(result.failed_count(), 1);
    assert_eq!(result.skipped_count(), 2);
}

#[test]
fn run_result_passed_excludes_skipped() {
    let tests = vec![
        TestResult::passed("test_one", Duration::from_millis(100)),
        TestResult::skipped("test_two"),
    ];
    let result = TestRunResult::passed(Duration::from_millis(100)).with_tests(tests);
    // Only count non-skipped passing tests
    assert_eq!(result.passed_count(), 1);
}

#[test]
fn percentile_duration_empty() {
    let result = TestRunResult::passed(Duration::from_millis(100));
    assert!(result.percentile_duration(50.0).is_none());
}

#[test]
fn percentile_duration_single_test() {
    let tests = vec![TestResult::passed("test_one", Duration::from_millis(100))];
    let result = TestRunResult::passed(Duration::from_millis(100)).with_tests(tests);
    assert_eq!(result.percentile_duration(50.0), Some(Duration::from_millis(100)));
    assert_eq!(result.percentile_duration(99.0), Some(Duration::from_millis(100)));
}

#[test]
fn percentile_duration_multiple_tests() {
    let tests = vec![
        TestResult::passed("t1", Duration::from_millis(10)),
        TestResult::passed("t2", Duration::from_millis(20)),
        TestResult::passed("t3", Duration::from_millis(30)),
        TestResult::passed("t4", Duration::from_millis(40)),
        TestResult::passed("t5", Duration::from_millis(50)),
        TestResult::passed("t6", Duration::from_millis(60)),
        TestResult::passed("t7", Duration::from_millis(70)),
        TestResult::passed("t8", Duration::from_millis(80)),
        TestResult::passed("t9", Duration::from_millis(90)),
        TestResult::passed("t10", Duration::from_millis(100)),
    ];
    let result = TestRunResult::passed(Duration::from_millis(550)).with_tests(tests);

    // p50 should be at the 5th element (index 4) = 50ms
    assert_eq!(result.percentile_duration(50.0), Some(Duration::from_millis(50)));
    // p90 should be at the 9th element (index 8) = 90ms
    assert_eq!(result.percentile_duration(90.0), Some(Duration::from_millis(90)));
    // p99 should be at the 10th element (index 9) = 100ms
    assert_eq!(result.percentile_duration(99.0), Some(Duration::from_millis(100)));
}

#[test]
fn percentile_duration_excludes_skipped() {
    let tests = vec![
        TestResult::passed("t1", Duration::from_millis(100)),
        TestResult::skipped("t2"),
        TestResult::passed("t3", Duration::from_millis(200)),
        TestResult::skipped("t4"),
    ];
    let result = TestRunResult::passed(Duration::from_millis(300)).with_tests(tests);

    // Only 2 non-skipped tests: 100ms and 200ms
    assert_eq!(result.percentile_duration(50.0), Some(Duration::from_millis(100)));
    assert_eq!(result.percentile_duration(99.0), Some(Duration::from_millis(200)));
}

#[test]
fn percentile_duration_all_skipped() {
    let tests = vec![TestResult::skipped("t1"), TestResult::skipped("t2")];
    let result = TestRunResult::passed(Duration::ZERO).with_tests(tests);
    assert!(result.percentile_duration(50.0).is_none());
}
