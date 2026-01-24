// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.
//!
//! Reads baseline files and outputs metrics in text, JSON, or HTML format.

use std::fmt::Write;

use quench::baseline::Baseline;
use quench::cli::{OutputFormat, ReportArgs};
use serde_json::json;

/// Format a report based on output format, returning the output string.
pub fn format_report_to_string(
    args: &ReportArgs,
    baseline: &Baseline,
    format: OutputFormat,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
        OutputFormat::Html => format_html_report(args, baseline),
    }
}

/// Check if a metric should be shown based on enabled/disabled checks.
fn should_show(metric: &str, enabled: &[String], disabled: &[String]) -> bool {
    if !enabled.is_empty() {
        // Explicit enable mode: only show specified metrics
        enabled.iter().any(|e| e == metric)
    } else {
        // Default mode: show all except disabled
        !disabled.iter().any(|d| d == metric)
    }
}

/// Format baseline metrics as human-readable text.
fn format_text_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<String> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();
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

    // Filter and display metrics
    let metrics = &baseline.metrics;

    // Coverage (mapped to "tests" check)
    if should_show("tests", &enabled, &disabled)
        && let Some(ref coverage) = metrics.coverage
    {
        writeln!(output, "coverage: {:.1}%", coverage.total)?;
    }

    // Escapes
    if should_show("escapes", &enabled, &disabled)
        && let Some(ref escapes) = metrics.escapes
    {
        for (name, count) in &escapes.source {
            writeln!(output, "escapes.{}: {}", name, count)?;
        }
    }

    // Build time
    if should_show("build", &enabled, &disabled)
        && let Some(ref build) = metrics.build_time
    {
        writeln!(output, "build_time.cold: {:.1}s", build.cold)?;
        writeln!(output, "build_time.hot: {:.1}s", build.hot)?;
    }

    // Test time
    if should_show("tests", &enabled, &disabled)
        && let Some(ref tests) = metrics.test_time
    {
        writeln!(output, "test_time.total: {:.1}s", tests.total)?;
    }

    // Binary size
    if should_show("build", &enabled, &disabled)
        && let Some(ref sizes) = metrics.binary_size
    {
        for (name, size) in sizes {
            writeln!(output, "binary_size.{}: {} bytes", name, size)?;
        }
    }

    Ok(output)
}

/// Format baseline metrics as JSON.
fn format_json_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<String> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();
    let metrics = &baseline.metrics;

    let mut output = serde_json::Map::new();

    // Metadata
    output.insert("updated".to_string(), json!(baseline.updated.to_rfc3339()));
    if let Some(ref commit) = baseline.commit {
        output.insert("commit".to_string(), json!(commit));
    }

    // Filtered metrics
    let mut filtered_metrics = serde_json::Map::new();

    if should_show("tests", &enabled, &disabled)
        && let Some(ref coverage) = metrics.coverage
    {
        filtered_metrics.insert("coverage".to_string(), json!({ "total": coverage.total }));
    }

    if should_show("escapes", &enabled, &disabled)
        && let Some(ref escapes) = metrics.escapes
    {
        filtered_metrics.insert("escapes".to_string(), json!({ "source": escapes.source }));
    }

    if should_show("build", &enabled, &disabled)
        && let Some(ref build) = metrics.build_time
    {
        filtered_metrics.insert(
            "build_time".to_string(),
            json!({
                "cold": build.cold,
                "hot": build.hot,
            }),
        );
    }
    if should_show("build", &enabled, &disabled)
        && let Some(ref sizes) = metrics.binary_size
    {
        filtered_metrics.insert("binary_size".to_string(), json!(sizes));
    }

    if should_show("tests", &enabled, &disabled)
        && let Some(ref tests) = metrics.test_time
    {
        filtered_metrics.insert(
            "test_time".to_string(),
            json!({
                "total": tests.total,
                "avg": tests.avg,
                "max": tests.max,
            }),
        );
    }

    output.insert(
        "metrics".to_string(),
        serde_json::Value::Object(filtered_metrics),
    );

    Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
        output,
    ))?)
}

/// Format baseline metrics as HTML dashboard.
fn format_html_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<String> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();
    let metrics = &baseline.metrics;

    let mut cards = Vec::new();
    let mut rows = Vec::new();

    // Coverage card
    if should_show("tests", &enabled, &disabled)
        && let Some(ref coverage) = metrics.coverage
    {
        cards.push(metric_card(
            "Coverage",
            &format!("{:.1}%", coverage.total),
            "tests",
        ));
        rows.push(table_row("coverage", &format!("{:.1}%", coverage.total)));
    }

    // Escapes cards
    if should_show("escapes", &enabled, &disabled)
        && let Some(ref escapes) = metrics.escapes
    {
        for (name, count) in &escapes.source {
            cards.push(metric_card(
                &format!("Escapes: {}", name),
                &count.to_string(),
                "escapes",
            ));
            rows.push(table_row(&format!("escapes.{}", name), &count.to_string()));
        }
    }

    // Build metrics
    if should_show("build", &enabled, &disabled)
        && let Some(ref build) = metrics.build_time
    {
        cards.push(metric_card(
            "Build (cold)",
            &format!("{:.1}s", build.cold),
            "build",
        ));
        cards.push(metric_card(
            "Build (hot)",
            &format!("{:.1}s", build.hot),
            "build",
        ));
        rows.push(table_row("build_time.cold", &format!("{:.1}s", build.cold)));
        rows.push(table_row("build_time.hot", &format!("{:.1}s", build.hot)));
    }
    if should_show("build", &enabled, &disabled)
        && let Some(ref sizes) = metrics.binary_size
    {
        for (name, size) in sizes {
            let human = human_bytes(*size);
            cards.push(metric_card(&format!("Binary: {}", name), &human, "build"));
            rows.push(table_row(&format!("binary_size.{}", name), &human));
        }
    }

    // Test time
    if should_show("tests", &enabled, &disabled)
        && let Some(ref tests) = metrics.test_time
    {
        cards.push(metric_card(
            "Test Time",
            &format!("{:.1}s", tests.total),
            "tests",
        ));
        rows.push(table_row(
            "test_time.total",
            &format!("{:.1}s", tests.total),
        ));
    }

    // Generate HTML
    Ok(html_template(baseline, &cards.join("\n"), &rows.join("\n")))
}

fn metric_card(title: &str, value: &str, category: &str) -> String {
    format!(
        r#"      <div class="card {category}">
        <div class="card-title">{title}</div>
        <div class="card-value">{value}</div>
      </div>"#
    )
}

fn table_row(metric: &str, value: &str) -> String {
    format!(r#"        <tr><td>{metric}</td><td>{value}</td></tr>"#)
}

fn human_bytes(bytes: u64) -> String {
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

fn html_template(baseline: &Baseline, cards: &str, rows: &str) -> String {
    let commit = baseline.commit.as_deref().unwrap_or("unknown");
    let date = baseline.updated.format("%Y-%m-%d %H:%M UTC");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    :root {{
      --bg: #1a1a2e;
      --card-bg: #16213e;
      --text: #eef;
      --muted: #8892b0;
      --accent: #64ffda;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: var(--bg);
      color: var(--text);
      padding: 2rem;
      line-height: 1.6;
    }}
    .container {{ max-width: 1200px; margin: 0 auto; }}
    header {{
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid var(--card-bg);
    }}
    h1 {{ color: var(--accent); font-size: 1.5rem; }}
    .meta {{ color: var(--muted); font-size: 0.875rem; margin-top: 0.5rem; }}
    .cards {{
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      gap: 1rem;
      margin-bottom: 2rem;
    }}
    .card {{
      background: var(--card-bg);
      padding: 1.5rem;
      border-radius: 8px;
      border-left: 4px solid var(--accent);
    }}
    .card.escapes {{ border-color: #f59e0b; }}
    .card.build {{ border-color: #8b5cf6; }}
    .card.tests {{ border-color: #10b981; }}
    .card-title {{ color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }}
    .card-value {{ font-size: 2rem; font-weight: 600; margin-top: 0.5rem; }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: var(--card-bg);
      border-radius: 8px;
      overflow: hidden;
    }}
    th, td {{ padding: 0.75rem 1rem; text-align: left; }}
    th {{ background: rgba(0,0,0,0.2); color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }}
    tr:not(:last-child) td {{ border-bottom: 1px solid var(--bg); }}
    td:last-child {{ text-align: right; font-family: monospace; }}
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
