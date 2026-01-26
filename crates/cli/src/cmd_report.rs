// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Report command implementation.

use std::io::Write;

use anyhow::Context;

use quench::baseline::Baseline;
use quench::cli::{Cli, OutputFormat, ReportArgs};
use quench::config;
use quench::discovery;
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

    // Determine baseline path (CLI flag overrides config)
    let baseline_path = args
        .baseline
        .clone()
        .unwrap_or_else(|| cwd.join(&config.git.baseline));

    // Parse output target (format and optional file path)
    let (format, file_path) = args.output_target();

    // Validate --compact flag (only applies to JSON)
    if args.compact && !matches!(format, OutputFormat::Json) {
        eprintln!("warning: --compact only applies to JSON output, ignoring");
    }

    // Load baseline
    let baseline = Baseline::load(&baseline_path)
        .with_context(|| format!("failed to load baseline from {}", baseline_path.display()))?;

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
