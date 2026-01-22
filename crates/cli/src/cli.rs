//! CLI argument parsing with clap derive.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::color::ColorMode;

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

    /// Color output mode
    #[arg(long, default_value = "auto", value_name = "WHEN")]
    pub color: ColorMode,

    /// Disable color output (shorthand for --color=never)
    #[arg(long)]
    pub no_color: bool,

    /// Maximum violations to display (default: 15)
    #[arg(long, default_value_t = 15, value_name = "N")]
    pub limit: usize,

    /// Show all violations (no limit)
    #[arg(long)]
    pub no_limit: bool,

    /// Validate config and exit without running checks
    #[arg(long = "config-only")]
    pub config_only: bool,

    /// Maximum directory depth to traverse
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// List scanned files (for debugging)
    #[arg(long, hide = true)]
    pub debug_files: bool,

    /// Enable verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    // Check enable flags (run only these checks)
    /// Run only the cloc check
    #[arg(long)]
    pub cloc: bool,

    /// Run only the escapes check
    #[arg(long)]
    pub escapes: bool,

    /// Run only the agents check
    #[arg(long)]
    pub agents: bool,

    /// Run only the docs check
    #[arg(long)]
    pub docs: bool,

    /// Run only the tests check
    #[arg(long = "tests")]
    pub tests_check: bool,

    /// Run only the git check
    #[arg(long)]
    pub git: bool,

    /// Run only the build check
    #[arg(long)]
    pub build: bool,

    /// Run only the license check
    #[arg(long)]
    pub license: bool,

    // Check disable flags (skip these checks)
    /// Skip the cloc check
    #[arg(long)]
    pub no_cloc: bool,

    /// Skip the escapes check
    #[arg(long)]
    pub no_escapes: bool,

    /// Skip the agents check
    #[arg(long)]
    pub no_agents: bool,

    /// Skip the docs check
    #[arg(long)]
    pub no_docs: bool,

    /// Skip the tests check
    #[arg(long)]
    pub no_tests: bool,

    /// Skip the git check
    #[arg(long)]
    pub no_git: bool,

    /// Skip the build check
    #[arg(long)]
    pub no_build: bool,

    /// Skip the license check
    #[arg(long)]
    pub no_license: bool,
}

impl CheckArgs {
    /// Get list of explicitly enabled checks.
    pub fn enabled_checks(&self) -> Vec<String> {
        let mut enabled = Vec::new();
        if self.cloc {
            enabled.push("cloc".to_string());
        }
        if self.escapes {
            enabled.push("escapes".to_string());
        }
        if self.agents {
            enabled.push("agents".to_string());
        }
        if self.docs {
            enabled.push("docs".to_string());
        }
        if self.tests_check {
            enabled.push("tests".to_string());
        }
        if self.git {
            enabled.push("git".to_string());
        }
        if self.build {
            enabled.push("build".to_string());
        }
        if self.license {
            enabled.push("license".to_string());
        }
        enabled
    }

    /// Get list of explicitly disabled checks.
    pub fn disabled_checks(&self) -> Vec<String> {
        let mut disabled = Vec::new();
        if self.no_cloc {
            disabled.push("cloc".to_string());
        }
        if self.no_escapes {
            disabled.push("escapes".to_string());
        }
        if self.no_agents {
            disabled.push("agents".to_string());
        }
        if self.no_docs {
            disabled.push("docs".to_string());
        }
        if self.no_tests {
            disabled.push("tests".to_string());
        }
        if self.no_git {
            disabled.push("git".to_string());
        }
        if self.no_build {
            disabled.push("build".to_string());
        }
        if self.no_license {
            disabled.push("license".to_string());
        }
        disabled
    }
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
