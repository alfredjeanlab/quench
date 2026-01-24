// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Text format report output.

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter, human_bytes};

/// Text format report formatter.
pub struct TextFormatter;

/// Size estimation constants for pre-allocation.
const TEXT_HEADER_SIZE: usize = 100;
const TEXT_METRIC_SIZE: usize = 50;

/// Write text report content. This macro handles the common formatting logic
/// for both fmt::Write (String) and io::Write (stdout, files).
macro_rules! write_text_report {
    ($writer:expr, $baseline:expr, $filtered:expr) => {
        // Header with baseline info
        writeln!($writer, "Quench Report")?;
        writeln!($writer, "=============")?;
        if let Some(ref commit) = $baseline.commit {
            let date = $baseline.updated.format("%Y-%m-%d");
            writeln!($writer, "Baseline: {} ({})", commit, date)?;
        } else {
            let date = $baseline.updated.format("%Y-%m-%d");
            writeln!($writer, "Baseline: {}", date)?;
        }
        writeln!($writer)?;

        // Coverage (mapped to "tests" check)
        if let Some(coverage) = $filtered.coverage() {
            writeln!($writer, "coverage: {:.1}%", coverage.total)?;

            if let Some(packages) = $filtered.sorted_package_coverage() {
                for (name, pct) in packages {
                    writeln!($writer, "  {}: {:.1}%", name, pct)?;
                }
            }
        }

        // Escapes
        if let Some(items) = $filtered.sorted_escapes() {
            for (name, count) in items {
                writeln!($writer, "escapes.{}: {}", name, count)?;
            }
        }

        // Test escapes (if present)
        if let Some(items) = $filtered.sorted_test_escapes() {
            for (name, count) in items {
                writeln!($writer, "escapes.test.{}: {}", name, count)?;
            }
        }

        // Build time
        if let Some(build) = $filtered.build_time() {
            writeln!($writer, "build_time.cold: {:.1}s", build.cold)?;
            writeln!($writer, "build_time.hot: {:.1}s", build.hot)?;
        }

        // Test time
        if let Some(tests) = $filtered.test_time() {
            writeln!($writer, "test_time.total: {:.1}s", tests.total)?;
        }

        // Binary size
        if let Some(items) = $filtered.sorted_binary_sizes() {
            for (name, size) in items {
                writeln!($writer, "binary_size.{}: {}", name, human_bytes(size))?;
            }
        }
    };
}

impl ReportFormatter for TextFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        use std::fmt::Write;

        let filtered = FilteredMetrics::new(baseline, filter);
        // Pre-allocate buffer based on estimated size
        let capacity = TEXT_HEADER_SIZE + filtered.count() * TEXT_METRIC_SIZE;
        let mut output = String::with_capacity(capacity);
        write_text_report!(&mut output, baseline, &filtered);
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        write_text_report!(writer, baseline, &filtered);
        Ok(())
    }

    fn format_empty(&self) -> String {
        "No baseline found.\n".to_string()
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
