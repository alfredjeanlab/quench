// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Auto-detection for test runners.

use std::path::Path;

use serde_json::json;

use crate::check::{CheckResult, Violation};
use crate::config::TestSuiteConfig;

use super::runners::{
    RunnerContext, detect_go_runner, detect_js_runner, detect_py_runner, detect_rust_runner,
};
use super::suite::run_single_suite;

/// Auto-detect JavaScript test runner.
///
/// Returns None if no runner can be detected.
pub fn auto_detect_js_suite(root: &Path) -> Option<(TestSuiteConfig, String)> {
    // Only auto-detect if package.json exists
    if !root.join("package.json").exists() {
        return None;
    }

    let detection = detect_js_runner(root)?;

    let suite = TestSuiteConfig {
        runner: detection.runner.name().to_string(),
        name: Some(format!("{} (auto-detected)", detection.runner.name())),
        path: None,
        setup: None,
        command: None,
        targets: vec![],
        ci: false,
        max_total: None,
        max_avg: None,
        max_test: None,
        timeout: None,
    };

    Some((suite, detection.source.to_metric_string()))
}

/// Auto-detect Python test runner.
///
/// Returns None if no runner can be detected.
pub fn auto_detect_py_suite(root: &Path) -> Option<(TestSuiteConfig, String)> {
    let detection = detect_py_runner(root)?;

    let suite = TestSuiteConfig {
        runner: detection.runner.name().to_string(),
        name: Some(format!("{} (auto-detected)", detection.runner.name())),
        path: None,
        setup: None,
        command: None,
        targets: vec![],
        ci: false,
        max_total: None,
        max_avg: None,
        max_test: None,
        timeout: None,
    };

    Some((suite, detection.source.to_metric_string()))
}

/// Auto-detect Rust test runner.
///
/// Returns None if no Cargo.toml exists.
pub fn auto_detect_rust_suite(root: &Path) -> Option<(TestSuiteConfig, String)> {
    let detection = detect_rust_runner(root)?;

    let suite = TestSuiteConfig {
        runner: detection.runner.name().to_string(),
        name: Some(format!("{} (auto-detected)", detection.runner.name())),
        path: None,
        setup: None,
        command: None,
        targets: vec![],
        ci: false,
        max_total: None,
        max_avg: None,
        max_test: None,
        timeout: None,
    };

    Some((suite, detection.source.to_metric_string()))
}

/// Auto-detect Go test runner.
///
/// Returns None if no go.mod exists or go is not installed.
pub fn auto_detect_go_suite(root: &Path) -> Option<(TestSuiteConfig, String)> {
    let detection = detect_go_runner(root)?;

    let suite = TestSuiteConfig {
        runner: detection.runner.name().to_string(),
        name: Some(format!("{} (auto-detected)", detection.runner.name())),
        path: None,
        setup: None,
        command: None,
        targets: vec![],
        ci: false,
        max_total: None,
        max_avg: None,
        max_test: None,
        timeout: None,
    };

    Some((suite, detection.source.to_metric_string()))
}

/// Run an auto-detected test suite with optional detection source.
pub fn run_auto_detected_suite(
    check_name: &str,
    suite: TestSuiteConfig,
    detection_source: Option<String>,
    runner_ctx: &RunnerContext,
) -> CheckResult {
    let result = run_single_suite(&suite, runner_ctx);

    // Build metrics with auto_detected flag
    let mut metrics = json!({
        "test_count": result.test_count,
        "total_ms": result.total_ms,
        "auto_detected": true,
        "runner": suite.runner,
        "suites": [{
            "name": result.name,
            "runner": result.runner,
            "passed": result.passed,
            "test_count": result.test_count,
        }]
    });

    // Add optional timing metrics
    if let Some(avg) = result.avg_ms {
        metrics["avg_ms"] = json!(avg);
    }
    if let Some(max) = result.max_ms {
        metrics["max_ms"] = json!(max);
    }
    if let Some(ref test) = result.max_test {
        metrics["max_test"] = json!(test);
    }

    // Add detection source if provided
    if let Some(source) = detection_source {
        metrics["detection_source"] = json!(source);
    }

    // Add coverage if collected
    if let Some(ref coverage) = result.coverage {
        metrics["coverage"] = json!(coverage);
    }

    if result.passed || result.skipped {
        CheckResult::passed(check_name).with_metrics(metrics)
    } else {
        let violation = Violation::file_only(
            format!("<suite:{}>", result.name),
            "test_suite_failed",
            result.error.unwrap_or_else(|| "test suite failed".to_string()),
        );
        CheckResult::failed(check_name, vec![violation]).with_metrics(metrics)
    }
}
