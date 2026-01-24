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

    /// Automatically fix violations when possible
    #[arg(long)]
    pub fix: bool,

    /// Show what --fix would change without changing it
    #[arg(long)]
    pub dry_run: bool,

    /// CI mode: run slow checks, auto-detect base branch
    #[arg(long)]
    pub ci: bool,

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

    /// Profile(s) to include (e.g., rust, shell, claude)
    #[arg(long = "with", value_delimiter = ',')]
    pub with_profiles: Vec<String>,
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

/// Default Shell profile configuration for quench init.
pub fn shell_profile_defaults() -> String {
    r##"[shell]
source = ["**/*.sh", "**/*.bash"]
tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh", "**/*_test.sh"]

[shell.suppress]
check = "comment"
comment = "# OK:"

[shell.suppress.test]
check = "allow"

[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]

[[check.escapes.patterns]]
name = "set_plus_e"
pattern = "set \\+e"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why error checking is disabled."

[[check.escapes.patterns]]
name = "eval"
pattern = "\\beval\\s"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why eval is safe here."

[[check.escapes.patterns]]
name = "rm_rf"
pattern = "rm\\s+-rf"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining the rm -rf is safe."
"##
    .to_string()
}

/// Shell-specific Landing the Plane checklist items.
pub fn shell_landing_items() -> &'static [&'static str] {
    &["shellcheck **/*.sh", "bats tests/"]
}

/// Default Go profile configuration for quench init.
pub fn golang_profile_defaults() -> String {
    r#"[golang]
binary_size = true
build_time = true

[golang.suppress]
check = "comment"

[golang.suppress.test]
check = "allow"

[golang.policy]
lint_changes = "standalone"
lint_config = [".golangci.yml", ".golangci.yaml", ".golangci.toml"]

[[check.escapes.patterns]]
name = "unsafe_pointer"
pattern = "unsafe\\.Pointer"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining pointer validity."

[[check.escapes.patterns]]
name = "go_linkname"
pattern = "//go:linkname"
action = "comment"
comment = "// LINKNAME:"
advice = "Add a // LINKNAME: comment explaining the external symbol dependency."

[[check.escapes.patterns]]
name = "go_noescape"
pattern = "//go:noescape"
action = "comment"
comment = "// NOESCAPE:"
advice = "Add a // NOESCAPE: comment explaining why escape analysis should be bypassed."
"#
    .to_string()
}

/// Go-specific Landing the Plane checklist items.
pub fn golang_landing_items() -> &'static [&'static str] {
    &[
        "go fmt ./...",
        "go vet ./...",
        "golangci-lint run",
        "go test ./...",
        "go build ./...",
    ]
}

// =============================================================================
// DETECTED LANGUAGE SECTIONS
// =============================================================================

/// Minimal Rust section for auto-detection output.
///
/// Uses dotted keys per spec: docs/specs/commands/quench-init.md
pub fn rust_detected_section() -> &'static str {
    r#"[rust]
rust.cloc.check = "error"
rust.policy.check = "error"
rust.suppress.check = "comment"
"#
}

/// Minimal Go section for auto-detection output.
pub fn golang_detected_section() -> &'static str {
    r#"[golang]
golang.cloc.check = "error"
golang.policy.check = "error"
golang.suppress.check = "comment"
"#
}

/// Minimal JavaScript section for auto-detection output.
pub fn javascript_detected_section() -> &'static str {
    r#"[javascript]
javascript.cloc.check = "error"
javascript.policy.check = "error"
javascript.suppress.check = "comment"
"#
}

/// Minimal Shell section for auto-detection output.
///
/// Note: Shell uses "forbid" for suppress by default.
pub fn shell_detected_section() -> &'static str {
    r#"[shell]
shell.cloc.check = "error"
shell.policy.check = "error"
shell.suppress.check = "forbid"
"#
}

// =============================================================================
// DEFAULT TEMPLATE
// =============================================================================

/// Default template for quench init without profiles.
///
/// Matches docs/specs/templates/init.default.toml
pub fn default_template() -> &'static str {
    r#"# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

[check.agents]
check = "error"

[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git.commit]
check = "off"  # stub in quench v0.3.0

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
