// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared test utilities for report formatter tests.

use crate::baseline::{
    Baseline, BaselineMetrics, BuildTimeMetrics, CoverageMetrics, EscapesMetrics, TestTimeMetrics,
};
use crate::cli::CheckFilter;

/// Test filter that includes all checks.
pub struct AllChecks;

impl CheckFilter for AllChecks {
    fn enabled_checks(&self) -> Vec<String> {
        Vec::new() // Empty means all enabled
    }

    fn disabled_checks(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Test filter that excludes specific checks.
pub struct ExcludeChecks(pub Vec<&'static str>);

impl CheckFilter for ExcludeChecks {
    fn enabled_checks(&self) -> Vec<String> {
        Vec::new()
    }

    fn disabled_checks(&self) -> Vec<String> {
        self.0.iter().map(|s| s.to_string()).collect()
    }
}

/// Create a standard test baseline with all metric types populated.
pub fn create_test_baseline() -> Baseline {
    Baseline {
        version: 1,
        updated: chrono::Utc::now(),
        commit: Some("abc1234".to_string()),
        metrics: BaselineMetrics {
            coverage: Some(CoverageMetrics {
                total: 85.5,
                by_package: None,
            }),
            escapes: Some(EscapesMetrics {
                source: [("unwrap".to_string(), 10), ("expect".to_string(), 5)]
                    .into_iter()
                    .collect(),
                test: None,
            }),
            build_time: Some(BuildTimeMetrics {
                cold: 45.0,
                hot: 12.5,
            }),
            binary_size: Some([("quench".to_string(), 5_242_880)].into_iter().collect()),
            test_time: Some(TestTimeMetrics {
                total: 30.5,
                avg: 0.5,
                max: 2.0,
            }),
        },
    }
}
