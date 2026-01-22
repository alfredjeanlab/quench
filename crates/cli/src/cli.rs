//! CLI argument parsing with clap derive.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// A fast linting tool for AI agents that measures quality signals
#[derive(Parser)]
#[command(name = "quench")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Use specific config file
    #[arg(short = 'C', long = "config", global = true, env = "QUENCH_CONFIG")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run quality checks
    Check(CheckArgs),
    /// Generate reports from stored metrics
    Report(ReportArgs),
    /// Initialize quench configuration
    Init(InitArgs),
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// Files or directories to check
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,

    /// Maximum directory depth to traverse
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// List scanned files (for debugging)
    #[arg(long, hide = true)]
    pub debug_files: bool,

    /// Enable verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(clap::Args)]
pub struct ReportArgs {
    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// Overwrite existing config
    #[arg(long)]
    pub force: bool,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
