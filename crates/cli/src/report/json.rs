// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSON format report output.

use quench::baseline::Baseline;
use quench::cli::CheckFilter;
use serde_json::json;

use super::{FilteredMetrics, ReportFormatter};

/// JSON format report formatter.
pub struct JsonFormatter;

impl ReportFormatter for JsonFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);

        let mut output = serde_json::Map::new();

        // Metadata
        output.insert("updated".to_string(), json!(baseline.updated.to_rfc3339()));
        if let Some(ref commit) = baseline.commit {
            output.insert("commit".to_string(), json!(commit));
        }

        // Filtered metrics
        let mut metrics = serde_json::Map::new();

        if let Some(coverage) = filtered.coverage() {
            metrics.insert("coverage".to_string(), json!({ "total": coverage.total }));
        }

        if let Some(escapes) = filtered.escapes() {
            metrics.insert("escapes".to_string(), json!({ "source": escapes.source }));
        }

        if let Some(build) = filtered.build_time() {
            metrics.insert(
                "build_time".to_string(),
                json!({
                    "cold": build.cold,
                    "hot": build.hot,
                }),
            );
        }

        if let Some(sizes) = filtered.binary_size() {
            metrics.insert("binary_size".to_string(), json!(sizes));
        }

        if let Some(tests) = filtered.test_time() {
            metrics.insert(
                "test_time".to_string(),
                json!({
                    "total": tests.total,
                    "avg": tests.avg,
                    "max": tests.max,
                }),
            );
        }

        output.insert("metrics".to_string(), serde_json::Value::Object(metrics));

        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
            output,
        ))?)
    }

    fn format_empty(&self) -> String {
        r#"{"metrics": {}}"#.to_string()
    }
}
