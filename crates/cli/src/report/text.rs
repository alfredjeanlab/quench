// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Text format report output.

use std::fmt::Write;

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter};

/// Text format report formatter.
pub struct TextFormatter;

impl ReportFormatter for TextFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let mut output = String::new();

        // Header with baseline info
        writeln!(output, "Quench Report")?;
        writeln!(output, "=============")?;
        if let Some(ref commit) = baseline.commit {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(output, "Baseline: {} ({})", commit, date)?;
        } else {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(output, "Baseline: {}", date)?;
        }
        writeln!(output)?;

        // Access filtered metrics
        let filtered = FilteredMetrics::new(baseline, filter);

        // Coverage (mapped to "tests" check)
        if let Some(coverage) = filtered.coverage() {
            writeln!(output, "coverage: {:.1}%", coverage.total)?;
        }

        // Escapes
        if let Some(escapes) = filtered.escapes() {
            for (name, count) in &escapes.source {
                writeln!(output, "escapes.{}: {}", name, count)?;
            }
        }

        // Build time
        if let Some(build) = filtered.build_time() {
            writeln!(output, "build_time.cold: {:.1}s", build.cold)?;
            writeln!(output, "build_time.hot: {:.1}s", build.hot)?;
        }

        // Test time
        if let Some(tests) = filtered.test_time() {
            writeln!(output, "test_time.total: {:.1}s", tests.total)?;
        }

        // Binary size
        if let Some(sizes) = filtered.binary_size() {
            for (name, size) in sizes {
                writeln!(output, "binary_size.{}: {} bytes", name, size)?;
            }
        }

        Ok(output)
    }

    fn format_empty(&self) -> String {
        "No baseline found.\n".to_string()
    }
}
