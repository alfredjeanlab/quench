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
    let baseline: Option<Baseline> = if let Some(ref base) = args.base {
        if base.ends_with(".json") {
            // Direct file load (e.g., --base baseline.json)
            let path = std::path::Path::new(base);
            let loaded = Baseline::load(&cwd.join(path))
                .with_context(|| format!("failed to load baseline from {}", path.display()))?;
            if loaded.is_none() {
                eprintln!("warning: baseline not found at {}", path.display());
            }
            loaded
        } else {
            // Git ref (e.g., --base main, --base HEAD~1)
            load_baseline_for_ref(&cwd, &config, base)?
        }
    } else {
        // No --base specified: use HEAD
        load_baseline_for_ref(&cwd, &config, "HEAD")?
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

/// Load baseline for a git reference using configured baseline source.
///
/// Strategy:
/// 1. If git notes configured: load from git notes for the ref
/// 2. If file-based baseline: load from configured file
/// 3. For HEAD only: fall back to .quench/latest.json cache
///
/// Returns None if no baseline is found.
fn load_baseline_for_ref(
    root: &Path,
    config: &Config,
    git_ref: &str,
) -> anyhow::Result<Option<Baseline>> {
    // For HEAD, try latest.json cache first (fast path)
    if git_ref == "HEAD" {
        let latest_path = root.join(".quench/latest.json");
        if let Ok(Some(latest)) = LatestMetrics::load(&latest_path) {
            return Ok(Some(Baseline {
                version: quench::baseline::BASELINE_VERSION,
                updated: latest.updated,
                commit: latest.commit,
                metrics: extract_baseline_metrics(&latest.output),
            }));
        }
    }

    // Use configured baseline source
    if config.git.uses_notes() && is_git_repo(root) {
        // Git notes mode
        match Baseline::load_from_notes(root, git_ref) {
            Ok(baseline) => Ok(baseline),
            Err(e) => {
                eprintln!("warning: failed to load baseline from git notes for {}: {}", git_ref, e);
                Ok(None)
            }
        }
    } else if let Some(path) = config.git.baseline_path() {
        // File-based baseline (ref is ignored)
        match Baseline::load(&root.join(path)) {
            Ok(baseline) => Ok(baseline),
            Err(e) => {
                eprintln!("warning: failed to load baseline from {}: {}", path, e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
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
