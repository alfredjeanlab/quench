// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.
//!
//! Reads baseline files and outputs metrics in text or JSON format.

use quench::baseline::Baseline;
use quench::cli::{OutputFormat, ReportArgs};
use serde_json::json;

/// Format a report based on output format.
pub fn format_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    match args.output {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
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
fn format_text_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();

    // Header with baseline info
    println!("Quench Report");
    println!("=============");
    if let Some(ref commit) = baseline.commit {
        let date = baseline.updated.format("%Y-%m-%d");
        println!("Baseline: {} ({})", commit, date);
    } else {
        let date = baseline.updated.format("%Y-%m-%d");
        println!("Baseline: {}", date);
    }
    println!();

    // Filter and display metrics
    let metrics = &baseline.metrics;

    // Coverage (mapped to "tests" check)
    if should_show("tests", &enabled, &disabled)
        && let Some(ref coverage) = metrics.coverage
    {
        println!("coverage: {:.1}%", coverage.total);
    }

    // Escapes
    if should_show("escapes", &enabled, &disabled)
        && let Some(ref escapes) = metrics.escapes
    {
        for (name, count) in &escapes.source {
            println!("escapes.{}: {}", name, count);
        }
    }

    // Build time
    if should_show("build", &enabled, &disabled)
        && let Some(ref build) = metrics.build_time
    {
        println!("build_time.cold: {:.1}s", build.cold);
        println!("build_time.hot: {:.1}s", build.hot);
    }

    // Test time
    if should_show("tests", &enabled, &disabled)
        && let Some(ref tests) = metrics.test_time
    {
        println!("test_time.total: {:.1}s", tests.total);
    }

    // Binary size
    if should_show("build", &enabled, &disabled)
        && let Some(ref sizes) = metrics.binary_size
    {
        for (name, size) in sizes {
            println!("binary_size.{}: {} bytes", name, size);
        }
    }

    Ok(())
}

/// Format baseline metrics as JSON.
fn format_json_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
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

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(output))?
    );
    Ok(())
}
