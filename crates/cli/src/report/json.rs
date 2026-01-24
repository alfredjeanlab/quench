// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JSON format report output.

use crate::baseline::Baseline;
use crate::cli::CheckFilter;
use serde_json::json;

use super::{FilteredMetrics, ReportFormatter};

/// JSON format report formatter.
pub struct JsonFormatter {
    compact: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter.
    ///
    /// If `compact` is true, outputs single-line JSON without whitespace.
    pub fn new(compact: bool) -> Self {
        Self { compact }
    }

    /// Build the JSON value from baseline and filter.
    fn build_json(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> serde_json::Value {
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

        serde_json::Value::Object(output)
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ReportFormatter for JsonFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let value = self.build_json(baseline, filter);
        if self.compact {
            Ok(serde_json::to_string(&value)?)
        } else {
            Ok(serde_json::to_string_pretty(&value)?)
        }
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let value = self.build_json(baseline, filter);
        if self.compact {
            serde_json::to_writer(writer, &value)?;
        } else {
            serde_json::to_writer_pretty(writer, &value)?;
        }
        Ok(())
    }

    fn format_empty(&self) -> String {
        if self.compact {
            r#"{"metrics":{}}"#.to_string()
        } else {
            "{\n  \"metrics\": {}\n}".to_string()
        }
    }
}
