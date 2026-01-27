// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::tempdir;

#[test]
fn all_runners_returns_expected_count() {
    let runners = all_runners();
    // cargo, bats, go, pytest, vitest, bun, jest, custom = 8 runners
    assert_eq!(runners.len(), 8);
}

#[test]
fn get_runner_finds_cargo() {
    let runner = get_runner("cargo");
    assert!(runner.is_some());
    assert_eq!(runner.unwrap().name(), "cargo");
}

#[test]
fn get_runner_finds_bats() {
    let runner = get_runner("bats");
    assert!(runner.is_some());
    assert_eq!(runner.unwrap().name(), "bats");
}

#[test]
fn get_runner_returns_none_for_unknown() {
    assert!(get_runner("unknown_runner").is_none());
}

#[test]
fn filter_suites_ci_mode_includes_all() {
    let suites = vec![
        TestSuiteConfig {
            runner: "cargo".to_string(),
            ci: false,
            name: None,
            path: None,
            setup: None,
            command: None,
            targets: vec![],
            max_total: None,
            max_avg: None,
            max_test: None,
            timeout: None,
        },
        TestSuiteConfig {
            runner: "pytest".to_string(),
            ci: true, // CI-only suite
            name: None,
            path: None,
            setup: None,
            command: None,
            targets: vec![],
            max_total: None,
            max_avg: None,
            max_test: None,
            timeout: None,
        },
    ];

    // In CI mode, all suites should be included
    let filtered = filter_suites_for_mode(&suites, true);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn filter_suites_fast_mode_excludes_ci_only() {
    let suites = vec![
        TestSuiteConfig {
            runner: "cargo".to_string(),
            ci: false,
            name: None,
            path: None,
            setup: None,
            command: None,
            targets: vec![],
            max_total: None,
            max_avg: None,
            max_test: None,
            timeout: None,
        },
        TestSuiteConfig {
            runner: "pytest".to_string(),
            ci: true, // CI-only suite
            name: None,
            path: None,
            setup: None,
            command: None,
            targets: vec![],
            max_total: None,
            max_avg: None,
            max_test: None,
            timeout: None,
        },
    ];

    // In fast mode (not CI), CI-only suites should be excluded
    let filtered = filter_suites_for_mode(&suites, false);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].runner, "cargo");
}

#[test]
fn run_setup_command_succeeds() {
    let temp = tempdir().unwrap();
    let result = run_setup_command("echo hello", temp.path());
    assert!(result.is_ok());
}

#[test]
fn run_setup_command_fails_on_bad_command() {
    let temp = tempdir().unwrap();
    let result = run_setup_command("exit 1", temp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("setup command failed"));
}

#[test]
fn run_setup_command_reports_stderr() {
    let temp = tempdir().unwrap();
    let result = run_setup_command("echo 'error message' >&2 && exit 1", temp.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("error message"));
}

#[test]
fn runner_names_contains_expected_entries() {
    assert!(RUNNER_NAMES.contains(&"cargo"));
    assert!(RUNNER_NAMES.contains(&"bats"));
    assert!(RUNNER_NAMES.contains(&"pytest"));
    assert!(RUNNER_NAMES.contains(&"custom"));
}

// =============================================================================
// Coverage Aggregation Tests
// =============================================================================

use std::time::Duration;

#[test]
fn merge_coverage_results_combines_files() {
    let a = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(70.0),
        files: [("src/a.rs".to_string(), 70.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let b = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(2),
        line_coverage: Some(80.0),
        files: [("src/b.rs".to_string(), 80.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let merged = merge_coverage_results(a, b);
    assert!(merged.success);
    assert_eq!(merged.files.len(), 2);
    assert_eq!(merged.files.get("src/a.rs"), Some(&70.0));
    assert_eq!(merged.files.get("src/b.rs"), Some(&80.0));
    assert_eq!(merged.duration, Duration::from_secs(3));
}

#[test]
fn merge_coverage_results_takes_max_for_same_file() {
    let a = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(60.0),
        files: [("src/lib.rs".to_string(), 60.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let b = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(80.0),
        files: [("src/lib.rs".to_string(), 80.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let merged = merge_coverage_results(a, b);
    assert_eq!(merged.files.get("src/lib.rs"), Some(&80.0));
    // Overall should be recalculated from files
    assert_eq!(merged.line_coverage, Some(80.0));
}

#[test]
fn merge_coverage_results_recalculates_overall() {
    let a = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(50.0),
        files: [("src/a.rs".to_string(), 50.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let b = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(90.0),
        files: [("src/b.rs".to_string(), 90.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let merged = merge_coverage_results(a, b);
    // Average of 50% and 90% = 70%
    assert_eq!(merged.line_coverage, Some(70.0));
}

#[test]
fn merge_coverage_results_handles_empty_files() {
    let a = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(60.0),
        files: HashMap::new(),
        packages: HashMap::new(),
    };

    let b = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(80.0),
        files: HashMap::new(),
        packages: HashMap::new(),
    };

    let merged = merge_coverage_results(a, b);
    // With no files, take max of overall
    assert_eq!(merged.line_coverage, Some(80.0));
}

#[test]
fn aggregated_coverage_merge_rust() {
    let mut agg = AggregatedCoverage::default();

    let result1 = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(70.0),
        files: [("src/a.rs".to_string(), 70.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    let result2 = CoverageResult {
        success: true,
        error: None,
        duration: Duration::from_secs(1),
        line_coverage: Some(90.0),
        files: [("src/b.rs".to_string(), 90.0)].into_iter().collect(),
        packages: HashMap::new(),
    };

    agg.merge_rust(result1);
    agg.merge_rust(result2);

    assert!(agg.rust.is_some());
    let rust = agg.rust.unwrap();
    assert_eq!(rust.files.len(), 2);
    // Average of 70% and 90% = 80%
    assert_eq!(rust.line_coverage, Some(80.0));
}

#[test]
fn aggregated_coverage_to_coverage_map() {
    let agg = AggregatedCoverage {
        rust: Some(CoverageResult {
            success: true,
            error: None,
            duration: Duration::ZERO,
            line_coverage: Some(75.0),
            files: HashMap::new(),
            packages: HashMap::new(),
        }),
        shell: Some(CoverageResult {
            success: true,
            error: None,
            duration: Duration::ZERO,
            line_coverage: Some(60.0),
            files: HashMap::new(),
            packages: HashMap::new(),
        }),
        go: None,
    };

    let map = agg.to_coverage_map();
    assert_eq!(map.get("rust"), Some(&75.0));
    assert_eq!(map.get("shell"), Some(&60.0));
}

#[test]
fn aggregated_coverage_has_data() {
    let agg_empty = AggregatedCoverage::default();
    assert!(!agg_empty.has_data());

    let agg_skipped = AggregatedCoverage {
        rust: Some(CoverageResult::skipped()),
        ..Default::default()
    };
    assert!(!agg_skipped.has_data()); // skipped has no line_coverage

    let agg_with_data = AggregatedCoverage {
        rust: Some(CoverageResult {
            success: true,
            error: None,
            duration: Duration::ZERO,
            line_coverage: Some(50.0),
            files: HashMap::new(),
            packages: HashMap::new(),
        }),
        ..Default::default()
    };
    assert!(agg_with_data.has_data());
}

// =============================================================================
// Timeout Tests
// =============================================================================

use std::io::ErrorKind;
use std::process::Command;

#[test]
fn run_with_timeout_no_timeout_succeeds() {
    let child = Command::new("echo")
        .arg("hello")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let result = run_with_timeout(child, None);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.status.success());
}

#[test]
fn run_with_timeout_fast_command_completes() {
    let child = Command::new("echo")
        .arg("fast")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // 10 second timeout - more than enough for echo
    let result = run_with_timeout(child, Some(Duration::from_secs(10)));
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fast"));
}

#[test]
fn run_with_timeout_slow_command_times_out() {
    // Sleep for 5 seconds but timeout after 100ms
    let child = Command::new("sleep")
        .arg("5")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let result = run_with_timeout(child, Some(Duration::from_millis(100)));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::TimedOut);
    assert!(err.to_string().contains("timed out"));
}
