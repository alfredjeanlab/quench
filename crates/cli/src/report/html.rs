// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! HTML format report output.

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{FilteredMetrics, ReportFormatter, human_bytes};

/// HTML format report formatter.
pub struct HtmlFormatter;

impl HtmlFormatter {
    /// Generate CSS styles for the report.
    fn css() -> &'static str {
        r#":root {
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
    td:last-child { text-align: right; font-family: monospace; }"#
    }

    /// Render a metric card.
    fn render_card(title: &str, value: &str, category: &str) -> String {
        format!(
            r#"      <div class="card {category}">
        <div class="card-title">{title}</div>
        <div class="card-value">{value}</div>
      </div>"#
        )
    }

    /// Render a table row.
    fn render_table_row(metric: &str, value: &str) -> String {
        format!(r#"        <tr><td>{metric}</td><td>{value}</td></tr>"#)
    }

    /// Render the header section.
    fn render_header(baseline: &Baseline) -> (String, String) {
        let commit = baseline.commit.as_deref().unwrap_or("unknown");
        let date = baseline.updated.format("%Y-%m-%d %H:%M UTC");
        (commit.to_string(), date.to_string())
    }

    /// Render the complete HTML document.
    fn render_document(commit: &str, date: &str, cards: &str, rows: &str) -> String {
        let css = Self::css();
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    {css}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Quench Report</h1>
      <div class="meta">Baseline: {commit} &middot; {date}</div>
    </header>
    <section class="cards">
{cards}
    </section>
    <section>
      <table>
        <thead><tr><th>Metric</th><th>Value</th></tr></thead>
        <tbody>
{rows}
        </tbody>
      </table>
    </section>
  </div>
</body>
</html>"#
        )
    }

    /// Collect metrics into cards and table rows based on filter.
    fn collect_metrics(
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> (Vec<String>, Vec<String>) {
        let filtered = FilteredMetrics::new(baseline, filter);
        let mut cards = Vec::new();
        let mut rows = Vec::new();

        // Coverage card
        if let Some(coverage) = filtered.coverage() {
            cards.push(Self::render_card(
                "Coverage",
                &format!("{:.1}%", coverage.total),
                "tests",
            ));
            rows.push(Self::render_table_row(
                "coverage",
                &format!("{:.1}%", coverage.total),
            ));
        }

        // Escapes cards
        if let Some(escapes) = filtered.escapes() {
            for (name, count) in &escapes.source {
                cards.push(Self::render_card(
                    &format!("Escapes: {}", name),
                    &count.to_string(),
                    "escapes",
                ));
                rows.push(Self::render_table_row(
                    &format!("escapes.{}", name),
                    &count.to_string(),
                ));
            }
        }

        // Build metrics
        if let Some(build) = filtered.build_time() {
            cards.push(Self::render_card(
                "Build (cold)",
                &format!("{:.1}s", build.cold),
                "build",
            ));
            cards.push(Self::render_card(
                "Build (hot)",
                &format!("{:.1}s", build.hot),
                "build",
            ));
            rows.push(Self::render_table_row(
                "build_time.cold",
                &format!("{:.1}s", build.cold),
            ));
            rows.push(Self::render_table_row(
                "build_time.hot",
                &format!("{:.1}s", build.hot),
            ));
        }

        if let Some(sizes) = filtered.binary_size() {
            for (name, size) in sizes {
                let human = human_bytes(*size);
                cards.push(Self::render_card(
                    &format!("Binary: {}", name),
                    &human,
                    "build",
                ));
                rows.push(Self::render_table_row(
                    &format!("binary_size.{}", name),
                    &human,
                ));
            }
        }

        // Test time
        if let Some(tests) = filtered.test_time() {
            cards.push(Self::render_card(
                "Test Time",
                &format!("{:.1}s", tests.total),
                "tests",
            ));
            rows.push(Self::render_table_row(
                "test_time.total",
                &format!("{:.1}s", tests.total),
            ));
        }

        (cards, rows)
    }
}

/// Size estimation constants for pre-allocation.
const HTML_BASE_SIZE: usize = 1500; // Template + CSS
const HTML_CARD_SIZE: usize = 200;
const HTML_ROW_SIZE: usize = 80;

impl ReportFormatter for HtmlFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);
        // Pre-allocate buffer based on estimated size
        let metric_count = filtered.count();
        let capacity = HTML_BASE_SIZE + metric_count * (HTML_CARD_SIZE + HTML_ROW_SIZE);
        let mut output = String::with_capacity(capacity);

        let (commit, date) = Self::render_header(baseline);
        let (cards, rows) = Self::collect_metrics(baseline, filter);
        output.push_str(&Self::render_document(
            &commit,
            &date,
            &cards.join("\n"),
            &rows.join("\n"),
        ));
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let (commit, date) = Self::render_header(baseline);
        let (cards, rows) = Self::collect_metrics(baseline, filter);
        let css = Self::css();

        // Write header
        write!(
            writer,
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    {css}
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

        // Write cards
        for card in &cards {
            writeln!(writer, "{}", card)?;
        }

        // Write table section
        write!(
            writer,
            r#"    </section>
    <section>
      <table>
        <thead><tr><th>Metric</th><th>Value</th></tr></thead>
        <tbody>
"#
        )?;

        // Write table rows
        for row in &rows {
            writeln!(writer, "{}", row)?;
        }

        // Write footer
        write!(
            writer,
            r#"        </tbody>
      </table>
    </section>
  </div>
</body>
</html>"#
        )?;

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
