// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Markdown format report output.

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter, human_bytes};

/// Markdown format report formatter.
pub struct MarkdownFormatter;

/// Write markdown report content. This macro handles the common formatting logic
/// for both fmt::Write (String) and io::Write (stdout, files).
macro_rules! write_markdown_report {
    ($writer:expr, $baseline:expr, $filtered:expr) => {
        // Header
        writeln!($writer, "# Quench Report\n")?;
        if let Some(ref commit) = $baseline.commit {
            let date = $baseline.updated.format("%Y-%m-%d");
            writeln!($writer, "**Baseline:** {} ({})\n", commit, date)?;
        }

        // Summary table
        writeln!($writer, "| Metric | Value |")?;
        writeln!($writer, "|--------|------:|")?;

        if let Some(coverage) = $filtered.coverage() {
            writeln!($writer, "| Coverage | {:.1}% |", coverage.total)?;

            if let Some(packages) = $filtered.sorted_package_coverage() {
                for (name, pct) in packages {
                    writeln!($writer, "| Coverage ({}) | {:.1}% |", name, pct)?;
                }
            }
        }

        if let Some(items) = $filtered.sorted_escapes() {
            for (name, count) in items {
                writeln!($writer, "| Escapes ({}) | {} |", name, count)?;
            }
        }

        // Test escapes (if present)
        if let Some(items) = $filtered.sorted_test_escapes() {
            for (name, count) in items {
                writeln!($writer, "| Escapes test ({}) | {} |", name, count)?;
            }
        }

        if let Some(build) = $filtered.build_time() {
            writeln!($writer, "| Build (cold) | {:.1}s |", build.cold)?;
            writeln!($writer, "| Build (hot) | {:.1}s |", build.hot)?;
        }

        if let Some(tests) = $filtered.test_time() {
            writeln!($writer, "| Test time | {:.1}s |", tests.total)?;
        }

        if let Some(items) = $filtered.sorted_binary_sizes() {
            for (name, size) in items {
                writeln!($writer, "| Binary ({}) | {} |", name, human_bytes(size))?;
            }
        }
    };
}

impl ReportFormatter for MarkdownFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        use std::fmt::Write;

        let filtered = FilteredMetrics::new(baseline, filter);
        let mut output = String::with_capacity(512);
        write_markdown_report!(&mut output, baseline, &filtered);
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        write_markdown_report!(writer, baseline, &filtered);
        Ok(())
    }

    fn format_empty(&self) -> String {
        "# Quench Report\n\n*No baseline found.*\n".to_string()
    }
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
