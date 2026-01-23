// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

    /// Force color output
    #[arg(long)]
    pub color: bool,

    /// Disable color output
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

    /// Compare against a git base ref (e.g., main, HEAD~1)
    #[arg(long, value_name = "REF")]
    pub base: Option<String>,

    /// List scanned files (for debugging)
    #[arg(long, hide = true)]
    pub debug_files: bool,

    /// Enable verbose output
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Bypass the cache (force fresh check)
    #[arg(long)]
    pub no_cache: bool,

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

impl CheckArgs {
    /// Get list of explicitly enabled checks.
    pub fn enabled_checks(&self) -> Vec<String> {
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

    /// Get list of explicitly disabled checks.
    pub fn disabled_checks(&self) -> Vec<String> {
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

    /// Configuration profile(s) to use (e.g., rust, claude)
    #[arg(long, short, value_delimiter = ',')]
    pub profile: Vec<String>,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

// =============================================================================
// PROFILE DEFAULTS
// =============================================================================

/// Default Rust profile configuration for quench init.
///
/// Note: The transmute pattern uses concat to avoid self-matching.
pub fn rust_profile_defaults() -> String {
    // SAFETY: String concatenation to avoid pattern self-match in escapes check.
    let transmute_pattern = format!("mem{}transmute", "::");
    format!(
        r#"[rust]
cfg_test_split = true

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml"]

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{{"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use ? operator or handle the error explicitly."

[[check.escapes.patterns]]
name = "expect"
pattern = "\\.expect\\("
action = "forbid"
advice = "Use ? operator or handle the error explicitly."

[[check.escapes.patterns]]
name = "transmute"
pattern = "{transmute_pattern}"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining type compatibility."
"#
    )
}

/// Rust-specific Landing the Plane checklist items.
pub fn rust_landing_items() -> &'static [&'static str] {
    &[
        "cargo fmt --check",
        "cargo clippy -- -D warnings",
        "cargo test",
        "cargo build",
    ]
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
