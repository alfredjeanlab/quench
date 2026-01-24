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

    /// Check only staged changes (pre-commit hook)
    #[arg(long)]
    pub staged: bool,

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

    /// Run only the placeholders check
    #[arg(long)]
    pub placeholders: bool,

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

    /// Skip the placeholders check
    #[arg(long)]
    pub no_placeholders: bool,
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
            placeholders => "placeholders",
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
            no_placeholders => "placeholders",
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

/// Claude agent profile configuration for quench init.
///
/// Sets up [check.agents] with CLAUDE.md as required.
pub fn claude_profile_defaults() -> &'static str {
    r#"[check.agents]
check = "error"
required = ["CLAUDE.md"]
"#
}

/// Cursor agent profile configuration for quench init.
///
/// Sets up [check.agents] with .cursorrules as required.
pub fn cursor_profile_defaults() -> &'static str {
    r#"[check.agents]
check = "error"
required = [".cursorrules"]
"#
}

/// JavaScript profile configuration for quench init.
pub fn javascript_profile_defaults() -> String {
    r#"[javascript]
source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
tests = ["**/*.test.*", "**/*.spec.*", "**/test/**", "**/tests/**", "**/__tests__/**"]

[javascript.suppress]
check = "comment"

[javascript.suppress.test]
check = "allow"

[javascript.policy]
lint_changes = "standalone"
lint_config = [".eslintrc", ".eslintrc.json", ".eslintrc.js", "eslint.config.js", ".prettierrc", ".prettierrc.json"]

[[check.escapes.patterns]]
name = "any_type"
pattern = ": any\\b"
action = "comment"
comment = "// ANY:"
advice = "Add a // ANY: comment explaining why any is needed."

[[check.escapes.patterns]]
name = "ts_ignore"
pattern = "@ts-ignore"
action = "forbid"
advice = "Use @ts-expect-error with an explanation instead."

[[check.escapes.patterns]]
name = "eslint_disable"
pattern = "eslint-disable"
action = "comment"
advice = "Add a comment explaining why this rule is disabled."
"#
    .to_string()
}

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
// PROFILE REGISTRY
// =============================================================================

/// Profile registry for init command.
///
/// Centralizes profile lookup for maintainability and extensibility.
pub struct ProfileRegistry;

impl ProfileRegistry {
    /// Get all available profile names.
    pub fn available() -> &'static [&'static str] {
        &["rust", "golang", "javascript", "shell", "claude", "cursor"]
    }

    /// Get profile content by name.
    ///
    /// Returns the full profile configuration string for the given profile name,
    /// or None if the profile is not recognized.
    pub fn get(name: &str) -> Option<String> {
        match name.to_lowercase().as_str() {
            "rust" => Some(rust_profile_defaults()),
            "shell" => Some(shell_profile_defaults()),
            "golang" | "go" => Some(golang_profile_defaults()),
            "javascript" | "js" | "typescript" | "ts" => Some(javascript_profile_defaults()),
            "claude" => Some(claude_profile_defaults().to_string()),
            "cursor" => Some(cursor_profile_defaults().to_string()),
            _ => None,
        }
    }

    /// Check if a profile name is valid.
    pub fn is_valid(name: &str) -> bool {
        Self::get(name).is_some()
    }

    /// Check if a profile is an agent profile (vs language profile).
    pub fn is_agent_profile(name: &str) -> bool {
        matches!(name.to_lowercase().as_str(), "claude" | "cursor")
    }

    /// Suggest similar profile names for typos.
    ///
    /// Uses simple prefix matching to suggest corrections.
    pub fn suggest(name: &str) -> Option<&'static str> {
        let lower = name.to_lowercase();

        // Check for common prefixes
        for &profile in Self::available() {
            if profile.starts_with(&lower) || lower.starts_with(profile) {
                return Some(profile);
            }
        }

        // Check for common aliases
        match lower.as_str() {
            "js" | "ts" | "typescript" | "node" => Some("javascript"),
            "go" => Some("golang"),
            "bash" | "zsh" | "sh" => Some("shell"),
            _ => None,
        }
    }
}

// =============================================================================
// DETECTED AGENT SECTIONS
// =============================================================================

use crate::init::{CursorMarker, DetectedAgent};

/// Generate [check.agents] section with detected agents.
///
/// Returns the TOML section with required files based on detected agents.
pub fn agents_detected_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return String::new();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor(CursorMarker::Cursorrules) => ".cursorrules",
            DetectedAgent::Cursor(CursorMarker::CursorRulesDir) => ".cursor/rules/*.mdc",
        })
        .collect();

    format!(
        r#"[check.agents]
check = "error"
required = {:?}
"#,
        required
    )
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

/// Base template without [check.agents] section.
/// The agents section is generated separately to support required field.
pub fn default_template_base() -> &'static str {
    r#"# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

"#
}

/// Portion of template after agents section.
pub fn default_template_suffix() -> &'static str {
    r#"
[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git]
baseline = ".quench/baseline.json"

[git.commit]
check = "off"  # stub in quench v0.3.0

[ratchet]
check = "error"       # error | warn | off
coverage = true       # Coverage can't drop
escapes = true        # Escape counts can't increase
# binary_size = false # Opt-in: binaries can't grow
# build_time_cold = false
# build_time_hot = false
# test_time_total = false

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}

/// Generate [check.agents] section with optional required field.
pub fn agents_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return "[check.agents]\ncheck = \"error\"\n".to_string();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor(CursorMarker::Cursorrules) => ".cursorrules",
            DetectedAgent::Cursor(CursorMarker::CursorRulesDir) => ".cursor/rules/*.mdc",
        })
        .collect();

    format!(
        "[check.agents]\ncheck = \"error\"\nrequired = {:?}\n",
        required
    )
}

/// Full default template for quench init without profiles.
///
/// Matches docs/specs/templates/init.default.toml
pub fn default_template() -> String {
    format!(
        "{}{}{}",
        default_template_base(),
        agents_section(&[]),
        default_template_suffix()
    )
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
