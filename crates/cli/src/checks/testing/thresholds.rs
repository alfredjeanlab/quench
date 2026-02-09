// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Threshold checking for test coverage and timing.

use std::collections::HashMap;

use crate::check::Violation;
use crate::config::{TestSuiteConfig, TestsConfig};

use super::suite::SuiteResult;

/// Check coverage against configured thresholds.
pub fn check_coverage_thresholds(
    config: &TestsConfig,
    coverage: &HashMap<String, f64>,
    packages: &HashMap<String, f64>,
) -> Vec<(Violation, bool)> {
    let cov_config = &config.coverage;
    if cov_config.check == "off" {
        return Vec::new();
    }

    let is_error = cov_config.check == "error";
    let mut violations = Vec::new();

    // Check global minimum
    if let Some(min) = cov_config.min {
        for (lang, &actual) in coverage {
            if actual < min {
                let advice = format!("Coverage {:.1}% below minimum {:.1}%", actual, min);
                let v = Violation::file_only(
                    format!("<coverage:{}>", lang),
                    "coverage_below_min",
                    advice,
                )
                .with_threshold(actual as i64, min as i64);
                violations.push((v, is_error));
            }
        }
    }

    // Check per-package thresholds
    for (pkg, pkg_config) in &cov_config.package {
        if let Some(&actual) = packages.get(pkg)
            && actual < pkg_config.min
        {
            let advice = format!(
                "Package '{}' coverage {:.1}% below minimum {:.1}%",
                pkg, actual, pkg_config.min
            );
            let v =
                Violation::file_only(format!("<coverage:{}>", pkg), "coverage_below_min", advice)
                    .with_threshold(actual as i64, pkg_config.min as i64);
            violations.push((v, is_error));
        }
    }

    violations
}

/// Check time thresholds for a suite.
pub fn check_time_thresholds(
    config: &TestsConfig,
    suite: &TestSuiteConfig,
    result: &SuiteResult,
) -> Vec<(Violation, bool)> {
    let time_config = &config.time;
    if time_config.check == "off" {
        return Vec::new();
    }

    let is_error = time_config.check == "error";
    let mut violations = Vec::new();
    let suite_name = &result.name;

    // Check max_total
    if let Some(max_total) = suite.max_total {
        let max_ms = max_total.as_millis() as u64;
        if result.total_ms > max_ms {
            let advice = format!(
                "Suite '{}' took {}ms, exceeds max_total {}ms",
                suite_name, result.total_ms, max_ms
            );
            let v = Violation::file_only(
                format!("<suite:{}>", suite_name),
                "time_total_exceeded",
                advice,
            )
            .with_threshold(result.total_ms as i64, max_ms as i64);
            violations.push((v, is_error));
        }
    }

    // Check max_avg
    if let Some(max_avg) = suite.max_avg
        && let Some(avg_ms) = result.avg_ms
    {
        let max_ms = max_avg.as_millis() as u64;
        if avg_ms > max_ms {
            let advice = format!(
                "Suite '{}' average {}ms/test, exceeds max_avg {}ms",
                suite_name, avg_ms, max_ms
            );
            let v = Violation::file_only(
                format!("<suite:{}>", suite_name),
                "time_avg_exceeded",
                advice,
            )
            .with_threshold(avg_ms as i64, max_ms as i64);
            violations.push((v, is_error));
        }
    }

    // Check max_test
    if let Some(max_test) = suite.max_test
        && let Some(max_ms) = result.max_ms
    {
        let threshold_ms = max_test.as_millis() as u64;
        if max_ms > threshold_ms {
            let test_name = result.max_test.as_deref().unwrap_or("unknown");
            let advice = format!(
                "Test '{}' took {}ms, exceeds max_test {}ms",
                test_name, max_ms, threshold_ms
            );
            let v =
                Violation::file_only(format!("<test:{}>", test_name), "time_test_exceeded", advice)
                    .with_threshold(max_ms as i64, threshold_ms as i64);
            violations.push((v, is_error));
        }
    }

    violations
}
