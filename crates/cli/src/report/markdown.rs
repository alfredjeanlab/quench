// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Markdown format report output.

use std::fmt::Write;

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter, human_bytes};

/// Markdown format report formatter.
pub struct MarkdownFormatter;

impl ReportFormatter for MarkdownFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);
        let mut output = String::with_capacity(512);

        // Header
        writeln!(output, "# Quench Report\n")?;
        if let Some(ref commit) = baseline.commit {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(output, "**Baseline:** {} ({})\n", commit, date)?;
        }

        // Summary table
        writeln!(output, "| Metric | Value |")?;
        writeln!(output, "|--------|------:|")?;

        if let Some(coverage) = filtered.coverage() {
            writeln!(output, "| Coverage | {:.1}% |", coverage.total)?;

            if let Some(ref packages) = coverage.by_package {
                let mut keys: Vec<_> = packages.keys().collect();
                keys.sort();
                for name in keys {
                    writeln!(output, "| Coverage ({}) | {:.1}% |", name, packages[name])?;
                }
            }
        }

        if let Some(escapes) = filtered.escapes() {
            let mut keys: Vec<_> = escapes.source.keys().collect();
            keys.sort();
            for name in keys {
                writeln!(output, "| Escapes ({}) | {} |", name, escapes.source[name])?;
            }

            // Test escapes (if present)
            if let Some(ref test) = escapes.test {
                let mut keys: Vec<_> = test.keys().collect();
                keys.sort();
                for name in keys {
                    writeln!(output, "| Escapes test ({}) | {} |", name, test[name])?;
                }
            }
        }

        if let Some(build) = filtered.build_time() {
            writeln!(output, "| Build (cold) | {:.1}s |", build.cold)?;
            writeln!(output, "| Build (hot) | {:.1}s |", build.hot)?;
        }

        if let Some(tests) = filtered.test_time() {
            writeln!(output, "| Test time | {:.1}s |", tests.total)?;
        }

        if let Some(sizes) = filtered.binary_size() {
            let mut keys: Vec<_> = sizes.keys().collect();
            keys.sort();
            for name in keys {
                writeln!(
                    output,
                    "| Binary ({}) | {} |",
                    name,
                    human_bytes(sizes[name])
                )?;
            }
        }

        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let output = self.format(baseline, filter)?;
        write!(writer, "{}", output)?;
        Ok(())
    }

    fn format_empty(&self) -> String {
        "# Quench Report\n\n*No baseline found.*\n".to_string()
    }
}
