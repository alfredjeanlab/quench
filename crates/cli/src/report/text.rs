// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Text format report output.

use std::fmt::Write;

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter};

/// Text format report formatter.
pub struct TextFormatter;

/// Size estimation constants for pre-allocation.
const TEXT_HEADER_SIZE: usize = 100;
const TEXT_METRIC_SIZE: usize = 50;

impl ReportFormatter for TextFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);
        // Pre-allocate buffer based on estimated size
        let capacity = TEXT_HEADER_SIZE + filtered.count() * TEXT_METRIC_SIZE;
        let mut output = String::with_capacity(capacity);
        self.write_to_fmt(&mut output, baseline, &filtered)?;
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        self.write_to_io(writer, baseline, &filtered)
    }

    fn format_empty(&self) -> String {
        "No baseline found.\n".to_string()
    }
}

impl TextFormatter {
    /// Write formatted output to a fmt::Write (String, etc.).
    fn write_to_fmt(
        &self,
        output: &mut String,
        baseline: &Baseline,
        filtered: &FilteredMetrics<'_>,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }

    /// Write formatted output to an io::Write (stdout, files, etc.).
    fn write_to_io(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filtered: &FilteredMetrics<'_>,
    ) -> anyhow::Result<()> {
        // Header with baseline info
        writeln!(writer, "Quench Report")?;
        writeln!(writer, "=============")?;
        if let Some(ref commit) = baseline.commit {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(writer, "Baseline: {} ({})", commit, date)?;
        } else {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(writer, "Baseline: {}", date)?;
        }
        writeln!(writer)?;

        // Coverage (mapped to "tests" check)
        if let Some(coverage) = filtered.coverage() {
            writeln!(writer, "coverage: {:.1}%", coverage.total)?;
        }

        // Escapes
        if let Some(escapes) = filtered.escapes() {
            for (name, count) in &escapes.source {
                writeln!(writer, "escapes.{}: {}", name, count)?;
            }
        }

        // Build time
        if let Some(build) = filtered.build_time() {
            writeln!(writer, "build_time.cold: {:.1}s", build.cold)?;
            writeln!(writer, "build_time.hot: {:.1}s", build.hot)?;
        }

        // Test time
        if let Some(tests) = filtered.test_time() {
            writeln!(writer, "test_time.total: {:.1}s", tests.total)?;
        }

        // Binary size
        if let Some(sizes) = filtered.binary_size() {
            for (name, size) in sizes {
                writeln!(writer, "binary_size.{}: {} bytes", name, size)?;
            }
        }

        Ok(())
    }
}
