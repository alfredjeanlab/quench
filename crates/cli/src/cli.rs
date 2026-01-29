// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! CLI argument parsing with clap derive.

use std::path::PathBuf;

use crate::help;
use clap::{Parser, Subcommand};
use clap_complete::Shell;

/// A fast linting tool for AI agents that measures quality signals
#[derive(Parser)]
#[command(name = "quench")]
#[command(version, about, long_about = None, disable_version_flag = true)]
#[command(propagate_version = true)]
#[command(styles = help::styles())]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", global = true, action = clap::ArgAction::Version)]
    version: (),

    /// Hidden alias for backwards compatibility
    #[arg(short = 'V', global = true, hide = true, action = clap::ArgAction::Version)]
    version_compat: (),

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run quality checks
    Check(CheckArgs),
    /// Count lines of code by language
    Cloc(ClocArgs),
    /// Generate reports from stored metrics
    Report(ReportArgs),
    /// Initialize quench configuration
    Init(InitArgs),
    /// Show configuration examples
    Config(ConfigArgs),
    /// Generate shell completions
    Completions(CompletionsArgs),
}

#[derive(clap::Args)]
pub struct ClocArgs {
    /// Files or directories to count
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Maximum directory depth to traverse
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// Files or directories to check
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,

    /// Maximum violations to display (default: 15)
    #[arg(long, default_value_t = 15, value_name = "N")]
    pub limit: usize,

    /// Show all violations (no limit)
    #[arg(long)]
    pub no_limit: bool,

    /// Maximum directory depth to traverse
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// Compare against a git base ref (e.g., main, HEAD~1)
    #[arg(long, value_name = "REF")]
    pub base: Option<String>,

    /// Check only staged changes (pre-commit hook)
    #[arg(long)]
    pub staged: bool,

    /// Bypass the cache (force fresh check)
    #[arg(long)]
    pub no_cache: bool,

    /// Automatically fix violations when possible
    #[arg(long)]
    pub fix: bool,

    /// Show what --fix would change without changing it
    #[arg(long)]
    pub dry_run: bool,

    /// CI mode: run slow checks, auto-detect base branch
    #[arg(long)]
    pub ci: bool,

    /// Show verbose diagnostic output (always enabled in --ci mode)
    #[arg(long)]
    pub verbose: bool,

    /// Show timing breakdown (phases, per-check, cache stats)
    #[arg(long)]
    pub timing: bool,

    /// Save metrics to file (CI mode)
    #[arg(long, value_name = "FILE")]
    pub save: Option<std::path::PathBuf>,

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

/// Trait for filtering checks/metrics by name.
///
/// Both `CheckArgs` and `ReportArgs` implement this trait to provide
/// consistent filtering behavior for check enable/disable flags.
pub trait CheckFilter {
    /// Get list of explicitly enabled checks.
    fn enabled_checks(&self) -> Vec<String>;

    /// Get list of explicitly disabled checks.
    fn disabled_checks(&self) -> Vec<String>;

    /// Check if a metric/check should be included based on filters.
    ///
    /// If any checks are explicitly enabled, only those are included.
    /// Otherwise, all checks are included except those explicitly disabled.
    fn should_include(&self, check_name: &str) -> bool {
        let enabled = self.enabled_checks();
        let disabled = self.disabled_checks();

        if !enabled.is_empty() {
            // Explicit enable mode: only show specified checks
            enabled.iter().any(|e| e == check_name)
        } else {
            // Default mode: show all except disabled
            !disabled.iter().any(|d| d == check_name)
        }
    }
}

/// Collect check names from boolean flags.
macro_rules! collect_checks {
    ($self:expr, $($flag:ident => $name:expr),+ $(,)?) => {{
        let mut checks = Vec::new();
        $(
            if $self.$flag {
                checks.push($name.to_string());
            }
        )+
        checks
    }};
}

impl CheckFilter for CheckArgs {
    fn enabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            cloc => "cloc",
            escapes => "escapes",
            agents => "agents",
            docs => "docs",
            tests_check => "tests",
            git => "git",
            build => "build",
            license => "license",
        )
    }

    fn disabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            no_cloc => "cloc",
            no_escapes => "escapes",
            no_agents => "agents",
            no_docs => "docs",
            no_tests => "tests",
            no_git => "git",
            no_build => "build",
            no_license => "license",
        )
    }
}

#[derive(clap::Args, Default)]
pub struct ReportArgs {
    /// Compare against a git ref or JSON file (e.g., main, HEAD~1, baseline.json)
    #[arg(long)]
    pub base: Option<String>,

    /// Output format or file path (e.g., text, json, html, report.html)
    #[arg(short, long, default_value = "text")]
    pub output: String,

    /// Output compact JSON (no whitespace, single line)
    #[arg(long)]
    pub compact: bool,

    // Check enable flags (show only these metrics)
    /// Show only cloc metrics
    #[arg(long)]
    pub cloc: bool,

    /// Show only escapes metrics
    #[arg(long)]
    pub escapes: bool,

    /// Show only agents metrics
    #[arg(long)]
    pub agents: bool,

    /// Show only docs metrics
    #[arg(long)]
    pub docs: bool,

    /// Show only tests metrics
    #[arg(long = "tests")]
    pub tests_check: bool,

    /// Show only git metrics
    #[arg(long)]
    pub git: bool,

    /// Show only build metrics
    #[arg(long)]
    pub build: bool,

    /// Show only license metrics
    #[arg(long)]
    pub license: bool,

    // Check disable flags (skip these metrics)
    /// Skip cloc metrics
    #[arg(long)]
    pub no_cloc: bool,

    /// Skip escapes metrics
    #[arg(long)]
    pub no_escapes: bool,

    /// Skip agents metrics
    #[arg(long)]
    pub no_agents: bool,

    /// Skip docs metrics
    #[arg(long)]
    pub no_docs: bool,

    /// Skip tests metrics
    #[arg(long)]
    pub no_tests: bool,

    /// Skip git metrics
    #[arg(long)]
    pub no_git: bool,

    /// Skip build metrics
    #[arg(long)]
    pub no_build: bool,

    /// Skip license metrics
    #[arg(long)]
    pub no_license: bool,
}

impl ReportArgs {
    /// Parse output argument into format and optional file path.
    pub fn output_target(&self) -> (OutputFormat, Option<PathBuf>) {
        let val = self.output.to_lowercase();

        // Check for file extension
        if val.ends_with(".html") {
            (OutputFormat::Html, Some(PathBuf::from(&self.output)))
        } else if val.ends_with(".json") {
            (OutputFormat::Json, Some(PathBuf::from(&self.output)))
        } else if val.ends_with(".md") {
            (OutputFormat::Markdown, Some(PathBuf::from(&self.output)))
        } else if val.ends_with(".txt") {
            (OutputFormat::Text, Some(PathBuf::from(&self.output)))
        } else {
            // Parse as format name
            let format = match val.as_str() {
                "json" => OutputFormat::Json,
                "html" => OutputFormat::Html,
                "md" | "markdown" => OutputFormat::Markdown,
                _ => OutputFormat::Text,
            };
            (format, None)
        }
    }
}

impl CheckFilter for ReportArgs {
    fn enabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            cloc => "cloc",
            escapes => "escapes",
            agents => "agents",
            docs => "docs",
            tests_check => "tests",
            git => "git",
            build => "build",
            license => "license",
        )
    }

    fn disabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            no_cloc => "cloc",
            no_escapes => "escapes",
            no_agents => "agents",
            no_docs => "docs",
            no_tests => "tests",
            no_git => "git",
            no_build => "build",
            no_license => "license",
        )
    }
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// Overwrite existing config
    #[arg(long)]
    pub force: bool,

    /// Profile(s) to include (e.g., rust, shell, claude)
    #[arg(long = "with", value_delimiter = ',')]
    pub with_profiles: Vec<String>,
}

#[derive(clap::Args)]
pub struct ConfigArgs {
    /// Feature to show configuration for (e.g., rust, tests, cloc)
    #[arg(value_name = "FEATURE")]
    pub feature: Option<String>,
}

#[derive(clap::Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Html,
    Markdown,
}

// Re-export profile-related items from the profiles module for backward compatibility
pub use crate::profiles::{
    ProfileRegistry, agents_detected_section, agents_section, claude_profile_defaults,
    cursor_profile_defaults, default_template, default_template_base, default_template_suffix,
    golang_detected_section, golang_landing_items, golang_profile_defaults,
    javascript_detected_section, javascript_landing_items, javascript_landing_items_for,
    javascript_profile_defaults, python_landing_items, python_landing_items_for,
    rust_detected_section, rust_landing_items, rust_profile_defaults, shell_detected_section,
    shell_landing_items, shell_profile_defaults,
};

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
