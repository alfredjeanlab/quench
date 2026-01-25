// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.
//!
//! Reads baseline files and outputs metrics in text, JSON, or HTML format.

mod html;
mod json;
mod markdown;
mod text;

use std::collections::HashMap;

use crate::baseline::{
    Baseline, BuildTimeMetrics, CoverageMetrics, EscapesMetrics, TestTimeMetrics,
};
use crate::cli::{CheckFilter, OutputFormat};

pub use html::HtmlFormatter;
pub use json::JsonFormatter;
pub use markdown::MarkdownFormatter;
pub use text::TextFormatter;

/// Helper for accessing filtered metrics.
///
/// Provides convenient access to baseline metrics while respecting
/// the check filter settings.
pub struct FilteredMetrics<'a> {
    baseline: &'a Baseline,
    filter: &'a dyn CheckFilter,
}

impl<'a> FilteredMetrics<'a> {
    /// Create a new filtered metrics accessor.
    pub fn new(baseline: &'a Baseline, filter: &'a dyn CheckFilter) -> Self {
        Self { baseline, filter }
    }

    /// Get coverage metrics if the "tests" check is included.
    pub fn coverage(&self) -> Option<&CoverageMetrics> {
        if self.filter.should_include("tests") {
            self.baseline.metrics.coverage.as_ref()
        } else {
            None
        }
    }

    /// Get escape metrics if the "escapes" check is included.
    pub fn escapes(&self) -> Option<&EscapesMetrics> {
        if self.filter.should_include("escapes") {
            self.baseline.metrics.escapes.as_ref()
        } else {
            None
        }
    }

    /// Get build time metrics if the "build" check is included.
    pub fn build_time(&self) -> Option<&BuildTimeMetrics> {
        if self.filter.should_include("build") {
            self.baseline.metrics.build_time.as_ref()
        } else {
            None
        }
    }

    /// Get binary size metrics if the "build" check is included.
    pub fn binary_size(&self) -> Option<&HashMap<String, u64>> {
        if self.filter.should_include("build") {
            self.baseline.metrics.binary_size.as_ref()
        } else {
            None
        }
    }

    /// Get test time metrics if the "tests" check is included.
    pub fn test_time(&self) -> Option<&TestTimeMetrics> {
        if self.filter.should_include("tests") {
            self.baseline.metrics.test_time.as_ref()
        } else {
            None
        }
    }

    /// Estimate number of metrics that will be included.
    pub fn count(&self) -> usize {
        let mut n = 0;
        if self.coverage().is_some() {
            n += 1;
        }
        if let Some(esc) = self.escapes() {
            n += esc.source.len();
        }
        if self.build_time().is_some() {
            n += 2; // cold + hot
        }
        if let Some(sizes) = self.binary_size() {
            n += sizes.len();
        }
        if self.test_time().is_some() {
            n += 1;
        }
        n
    }

    /// Iterate over escape source metrics in sorted order.
    /// Returns None if escapes check is filtered out.
    pub fn sorted_escapes(&self) -> Option<Vec<(&str, usize)>> {
        self.escapes().map(|esc| {
            let mut items: Vec<_> = esc.source.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            items.sort_by_key(|(k, _)| *k);
            items
        })
    }

    /// Iterate over test escape metrics in sorted order.
    /// Returns None if escapes check is filtered out or no test escapes present.
    pub fn sorted_test_escapes(&self) -> Option<Vec<(&str, usize)>> {
        self.escapes().and_then(|esc| {
            esc.test.as_ref().map(|test| {
                let mut items: Vec<_> = test.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                items.sort_by_key(|(k, _)| *k);
                items
            })
        })
    }

    /// Iterate over coverage by package in sorted order.
    /// Returns None if tests check is filtered out or no package coverage.
    pub fn sorted_package_coverage(&self) -> Option<Vec<(&str, f64)>> {
        self.coverage().and_then(|cov| {
            cov.by_package.as_ref().map(|packages| {
                let mut items: Vec<_> = packages.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                items.sort_by_key(|(k, _)| *k);
                items
            })
        })
    }

    /// Iterate over binary sizes in sorted order.
    /// Returns None if build check is filtered out or no binary sizes.
    pub fn sorted_binary_sizes(&self) -> Option<Vec<(&str, u64)>> {
        self.binary_size().map(|sizes| {
            let mut items: Vec<_> = sizes.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            items.sort_by_key(|(k, _)| *k);
            items
        })
    }
}

/// Trait for formatting baseline metrics into various output formats.
pub trait ReportFormatter {
    /// Format baseline metrics into the target format.
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String>;

    /// Format baseline metrics directly to a writer (streaming).
    ///
    /// This method uses dynamic dispatch for the writer to maintain trait object compatibility.
    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()>;

    /// Return output for when no baseline exists.
    fn format_empty(&self) -> String;

    /// Write empty output to a writer.
    fn format_empty_to(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        write!(writer, "{}", self.format_empty())
    }
}

/// Create formatter based on output format.
fn create_formatter(format: OutputFormat, compact: bool) -> Box<dyn ReportFormatter> {
    match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter::new(compact)),
        OutputFormat::Html => Box::new(HtmlFormatter),
        OutputFormat::Markdown => Box::new(MarkdownFormatter),
    }
}

/// Format a report based on output format, returning the output string.
///
/// If baseline is None, returns the format-specific empty output.
pub fn format_report<F: CheckFilter>(
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
) -> anyhow::Result<String> {
    format_report_with_options(format, baseline, filter, false)
}

/// Format a report with additional options.
///
/// The `compact` parameter enables compact JSON output (single line, no whitespace).
pub fn format_report_with_options<F: CheckFilter>(
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
    compact: bool,
) -> anyhow::Result<String> {
    let formatter = create_formatter(format, compact);

    match baseline {
        Some(b) => formatter.format(b, filter),
        None => Ok(formatter.format_empty()),
    }
}

/// Format a report directly to a writer (streaming).
///
/// This avoids intermediate String allocation when writing to stdout or files.
pub fn format_report_to<F: CheckFilter>(
    writer: &mut dyn std::io::Write,
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
    compact: bool,
) -> anyhow::Result<()> {
    let formatter = create_formatter(format, compact);

    match baseline {
        Some(b) => formatter.format_to(writer, b, filter),
        None => Ok(formatter.format_empty_to(writer)?),
    }
}

/// Helper to convert bytes to human-readable format (with space).
pub fn human_bytes(bytes: u64) -> String {
    crate::file_size::human_size(bytes, true)
}

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
