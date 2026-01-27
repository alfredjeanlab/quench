// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.

use std::io::Write;
use std::path::Path;

use anyhow::Context;

use quench::baseline::Baseline;
use quench::cli::{Cli, OutputFormat, ReportArgs};
use quench::config::{self, Config};
use quench::discovery;
use quench::git::is_git_repo;
use quench::latest::LatestMetrics;
use quench::report;

/// Run the report command.
pub fn run(_cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;

    // Find and load config
    let config = if let Some(path) = discovery::find_config(&cwd) {
        config::load_with_warnings(&path)?
    } else {
        config::Config::default()
    };

    // Parse output target (format and optional file path)
    let (format, file_path) = args.output_target();

    // Validate --compact flag (only applies to JSON)
    if args.compact && !matches!(format, OutputFormat::Json) {
        eprintln!("warning: --compact only applies to JSON output, ignoring");
    }

    // Load baseline from the best available source
    let baseline: Option<Baseline> = if let Some(ref path) = args.baseline {
        // Explicit --baseline flag
        let loaded = Baseline::load(&cwd.join(path))
            .with_context(|| format!("failed to load baseline from {}", path.display()))?;
        if loaded.is_none() {
            eprintln!("warning: baseline not found at {}", path.display());
        }
        loaded
    } else {
        // Try sources in order (returns None if nothing found)
        load_latest_or_baseline(&cwd, &config)
    };

    // Write output using streaming when possible
    match file_path {
        Some(path) => {
            // File output: use buffered writer for efficiency
            let file = std::fs::File::create(&path)?;
            let mut writer = std::io::BufWriter::new(file);
            report::format_report_to(&mut writer, format, baseline.as_ref(), args, args.compact)?;
            writer.flush()?;
        }
        None => {
            // Stdout: use stdout lock for efficiency
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            report::format_report_to(&mut handle, format, baseline.as_ref(), args, args.compact)?;
            // Add trailing newline for JSON output
            if matches!(format, OutputFormat::Json) {
                writeln!(handle)?;
            }
        }
    }
    Ok(())
}

/// Load metrics from the best available source.
///
/// Tries sources in order:
/// 1. .quench/latest.json (local cache)
/// 2. Git notes for HEAD
/// 3. Configured baseline file
///
/// Returns None if no metrics are found.
fn load_latest_or_baseline(root: &Path, config: &Config) -> Option<Baseline> {
    // Try latest.json first
    let latest_path = root.join(".quench/latest.json");
    if let Ok(Some(latest)) = LatestMetrics::load(&latest_path) {
        // Convert LatestMetrics to Baseline for report
        return Some(Baseline {
            version: quench::baseline::BASELINE_VERSION,
            updated: latest.updated,
            commit: latest.commit,
            metrics: extract_baseline_metrics(&latest.output),
        });
    }

    // Try git notes
    if config.git.uses_notes()
        && is_git_repo(root)
        && let Ok(Some(baseline)) = Baseline::load_from_notes(root, "HEAD")
    {
        return Some(baseline);
    }

    // Try baseline file
    if let Some(path) = config.git.baseline_path()
        && let Ok(Some(baseline)) = Baseline::load(&root.join(path))
    {
        return Some(baseline);
    }

    None
}

/// Extract baseline metrics from CheckOutput.
fn extract_baseline_metrics(
    output: &quench::check::CheckOutput,
) -> quench::baseline::BaselineMetrics {
    use quench::baseline::{BaselineMetrics, EscapesMetrics};
    use std::collections::HashMap;

    let mut metrics = BaselineMetrics::default();

    for check in &output.checks {
        if check.name == "escapes"
            && let Some(check_metrics) = &check.metrics
        {
            let mut source: HashMap<String, usize> = HashMap::new();

            if let Some(source_obj) = check_metrics.get("source").and_then(|s| s.as_object()) {
                for (key, value) in source_obj {
                    if let Some(count) = value.as_u64() {
                        source.insert(key.clone(), count as usize);
                    }
                }
            }

            if !source.is_empty() {
                metrics.escapes = Some(EscapesMetrics { source, test: None });
            }
        }
        // Add other metric types as needed (coverage, build_time, etc.)
    }

    metrics
}
