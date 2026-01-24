// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.
//!
//! Reads baseline files and outputs metrics in text, JSON, or HTML format.

mod html;
mod json;
mod text;

use std::collections::HashMap;

use crate::baseline::{
    Baseline, BuildTimeMetrics, CoverageMetrics, EscapesMetrics, TestTimeMetrics,
};
use crate::cli::{CheckFilter, OutputFormat};

pub use html::HtmlFormatter;
pub use json::JsonFormatter;
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
}

/// Trait for formatting baseline metrics into various output formats.
pub trait ReportFormatter {
    /// Format baseline metrics into the target format.
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String>;

    /// Return output for when no baseline exists.
    fn format_empty(&self) -> String;
}

/// Format a report based on output format, returning the output string.
///
/// If baseline is None, returns the format-specific empty output.
pub fn format_report<F: CheckFilter>(
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
) -> anyhow::Result<String> {
    let formatter: Box<dyn ReportFormatter> = match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Html => Box::new(HtmlFormatter),
    };

    match baseline {
        Some(b) => formatter.format(b, filter),
        None => Ok(formatter.format_empty()),
    }
}

/// Helper to convert bytes to human-readable format.
pub fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
