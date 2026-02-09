// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! HTML format report output.

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter, human_bytes};

/// HTML format report formatter.
pub struct HtmlFormatter;

/// Size estimation constants for pre-allocation.
const HTML_BASE_SIZE: usize = 1500; // Template + CSS
const HTML_CARD_SIZE: usize = 200;
const HTML_ROW_SIZE: usize = 80;

/// CSS styles for the report.
const CSS: &str = r#":root {
      --bg: #1a1a2e;
      --card-bg: #16213e;
      --text: #eef;
      --muted: #8892b0;
      --accent: #64ffda;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: var(--bg);
      color: var(--text);
      padding: 2rem;
      line-height: 1.6;
    }
    .container { max-width: 1200px; margin: 0 auto; }
    header {
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid var(--card-bg);
    }
    h1 { color: var(--accent); font-size: 1.5rem; }
    .meta { color: var(--muted); font-size: 0.875rem; margin-top: 0.5rem; }
    .cards {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      gap: 1rem;
      margin-bottom: 2rem;
    }
    .card {
      background: var(--card-bg);
      padding: 1.5rem;
      border-radius: 8px;
      border-left: 4px solid var(--accent);
    }
    .card.escapes { border-color: #f59e0b; }
    .card.build { border-color: #8b5cf6; }
    .card.tests { border-color: #10b981; }
    .card-title { color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }
    .card-value { font-size: 2rem; font-weight: 600; margin-top: 0.5rem; }
    table {
      width: 100%;
      border-collapse: collapse;
      background: var(--card-bg);
      border-radius: 8px;
      overflow: hidden;
    }
    th, td { padding: 0.75rem 1rem; text-align: left; }
    th { background: rgba(0,0,0,0.2); color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }
    tr:not(:last-child) td { border-bottom: 1px solid var(--bg); }
    td:last-child { text-align: right; font-family: monospace; }"#;

/// Write a metric card inline.
macro_rules! write_card {
    ($writer:expr, $title:expr, $value:expr, $category:expr) => {
        writeln!(
            $writer,
            r#"      <div class="card {}">
        <div class="card-title">{}</div>
        <div class="card-value">{}</div>
      </div>"#,
            $category, $title, $value
        )?;
    };
}

/// Write a table row inline.
macro_rules! write_row {
    ($writer:expr, $metric:expr, $value:expr) => {
        writeln!($writer, r#"        <tr><td>{}</td><td>{}</td></tr>"#, $metric, $value)?;
    };
}

/// Write HTML report content. This macro handles the common formatting logic
/// for both fmt::Write (String) and io::Write (stdout, files).
macro_rules! write_html_report {
    ($writer:expr, $baseline:expr, $filtered:expr) => {{
        let commit = $baseline.commit.as_deref().unwrap_or("unknown");
        let date = $baseline.updated.format("%Y-%m-%d %H:%M UTC");

        // Write document header
        write!(
            $writer,
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    {CSS}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Quench Report</h1>
      <div class="meta">Baseline: {commit} &middot; {date}</div>
    </header>
    <section class="cards">
"#
        )?;

        // Write cards section
        if let Some(coverage) = $filtered.coverage() {
            write_card!($writer, "Coverage", format!("{:.1}%", coverage.total), "tests");
        }

        if let Some(items) = $filtered.sorted_escapes() {
            for (name, count) in items {
                write_card!($writer, format!("Escapes: {}", name), count, "escapes");
            }
        }

        if let Some(build) = $filtered.build_time() {
            write_card!($writer, "Build (cold)", format!("{:.1}s", build.cold), "build");
            write_card!($writer, "Build (hot)", format!("{:.1}s", build.hot), "build");
        }

        if let Some(items) = $filtered.sorted_binary_sizes() {
            for (name, size) in items {
                write_card!($writer, format!("Binary: {}", name), human_bytes(size), "build");
            }
        }

        if let Some(tests) = $filtered.test_time() {
            write_card!($writer, "Test Time", format!("{:.1}s", tests.total), "tests");
        }

        // Write table section header
        write!(
            $writer,
            r#"    </section>
    <section>
      <table>
        <thead><tr><th>Metric</th><th>Value</th></tr></thead>
        <tbody>
"#
        )?;

        // Write table rows
        if let Some(coverage) = $filtered.coverage() {
            write_row!($writer, "coverage", format!("{:.1}%", coverage.total));

            if let Some(packages) = $filtered.sorted_package_coverage() {
                for (name, pct) in packages {
                    write_row!($writer, format!("coverage.{}", name), format!("{:.1}%", pct));
                }
            }
        }

        if let Some(items) = $filtered.sorted_escapes() {
            for (name, count) in items {
                write_row!($writer, format!("escapes.{}", name), count);
            }
        }

        if let Some(items) = $filtered.sorted_test_escapes() {
            for (name, count) in items {
                write_row!($writer, format!("escapes.test.{}", name), count);
            }
        }

        if let Some(build) = $filtered.build_time() {
            write_row!($writer, "build_time.cold", format!("{:.1}s", build.cold));
            write_row!($writer, "build_time.hot", format!("{:.1}s", build.hot));
        }

        if let Some(items) = $filtered.sorted_binary_sizes() {
            for (name, size) in items {
                write_row!($writer, format!("binary_size.{}", name), human_bytes(size));
            }
        }

        if let Some(tests) = $filtered.test_time() {
            write_row!($writer, "test_time.total", format!("{:.1}s", tests.total));
        }

        // Write document footer
        write!(
            $writer,
            r#"        </tbody>
      </table>
    </section>
  </div>
</body>
</html>"#
        )?;
    }};
}

impl ReportFormatter for HtmlFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        use std::fmt::Write;

        let filtered = FilteredMetrics::new(baseline, filter);
        let capacity = HTML_BASE_SIZE + filtered.count() * (HTML_CARD_SIZE + HTML_ROW_SIZE);
        let mut output = String::with_capacity(capacity);
        write_html_report!(&mut output, baseline, &filtered);
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        write_html_report!(writer, baseline, &filtered);
        Ok(())
    }

    fn format_empty(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Quench Report</title>
</head>
<body>
  <h1>No baseline found.</h1>
</body>
</html>"#
            .to_string()
    }
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
