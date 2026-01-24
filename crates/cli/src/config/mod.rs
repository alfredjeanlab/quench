// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Configuration parsing and validation.
//!
//! Handles quench.toml parsing with version validation and unknown key warnings.

mod go;
mod parse;
mod shell;
mod suppress;

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

pub use go::{GoConfig, GoPolicyConfig, GoSuppressConfig};
pub use shell::{ShellConfig, ShellPolicyConfig, ShellSuppressConfig};
pub use suppress::{SuppressConfig, SuppressLevel, SuppressScopeConfig};

use crate::error::{Error, Result};
use parse::{
    parse_agents_config, parse_cloc_config, parse_escapes_config, parse_go_config,
    parse_rust_config, parse_shell_config, warn_unknown_key,
};

pub use crate::checks::agents::config::{AgentsConfig, AgentsScopeConfig};

/// Minimum config structure for version checking.
#[derive(Deserialize)]
struct VersionOnly {
    version: Option<i64>,
}

/// Config with flexible parsing that captures unknown keys.
#[derive(Deserialize)]
struct FlexibleConfig {
    version: i64,

    #[serde(default)]
    project: Option<toml::Value>,

    #[serde(default)]
    workspace: Option<toml::Value>,

    #[serde(default)]
    check: Option<toml::Value>,

    #[serde(default)]
    rust: Option<toml::Value>,

    #[serde(default)]
    go: Option<toml::Value>,

    #[serde(default)]
    golang: Option<toml::Value>,

    #[serde(default)]
    shell: Option<toml::Value>,

    #[serde(flatten)]
    unknown: std::collections::BTreeMap<String, toml::Value>,
}

/// Full configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Config file version (must be 1).
    pub version: i64,

    /// Project configuration.
    #[serde(default)]
    pub project: ProjectConfig,

    /// Workspace configuration (for monorepos).
    #[serde(default)]
    pub workspace: WorkspaceConfig,

    /// Check configurations.
    #[serde(default)]
    pub check: CheckConfig,

    /// Rust-specific configuration.
    #[serde(default)]
    pub rust: RustConfig,

    /// Go-specific configuration.
    #[serde(default)]
    pub go: GoConfig,

    /// Shell-specific configuration.
    #[serde(default)]
    pub shell: ShellConfig,
}

impl Config {
    /// Get cloc advice for source files, checking user override then language default.
    pub fn cloc_advice_for_language(&self, language: &str) -> &str {
        match language {
            "rust" => self
                .rust
                .cloc_advice
                .as_deref()
                .unwrap_or(RustConfig::default_cloc_advice()),
            "go" => self
                .go
                .cloc_advice
                .as_deref()
                .unwrap_or(GoConfig::default_cloc_advice()),
            "shell" => self
                .shell
                .cloc_advice
                .as_deref()
                .unwrap_or(ShellConfig::default_cloc_advice()),
            _ => &self.check.cloc.advice,
        }
    }
}

/// Rust language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RustConfig {
    /// Split #[cfg(test)] blocks from source LOC (default: true).
    #[serde(default = "RustConfig::default_cfg_test_split")]
    pub cfg_test_split: bool,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: SuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RustPolicyConfig,

    /// Custom cloc advice for source files (None = use generic default).
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for RustConfig {
    fn default() -> Self {
        Self {
            cfg_test_split: Self::default_cfg_test_split(),
            suppress: SuppressConfig::default(),
            policy: RustPolicyConfig::default(),
            cloc_advice: None,
        }
    }
}

impl RustConfig {
    pub(crate) fn default_cfg_test_split() -> bool {
        true
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         If not, split large source files into sibling modules or submodules in a folder;\n\
         consider refactoring to be more unit testable.\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}

/// Rust lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RustPolicyConfig {
    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "RustPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for RustPolicyConfig {
    fn default() -> Self {
        Self {
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl RustPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            "rustfmt.toml".to_string(),
            ".rustfmt.toml".to_string(),
            "clippy.toml".to_string(),
            ".clippy.toml".to_string(),
        ]
    }
}

impl crate::adapter::common::policy::PolicyConfig for RustPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}

/// Lint changes policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintChangesPolicy {
    /// No policy - mixed changes allowed.
    #[default]
    None,
    /// Lint config changes must be in standalone PRs.
    Standalone,
}

/// Workspace configuration for monorepos.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Package directories within the workspace.
    #[serde(default)]
    pub packages: Vec<String>,

    /// Package name lookup (path -> name).
    /// Auto-populated when detecting Rust workspaces.
    #[serde(default)]
    pub package_names: std::collections::HashMap<String, String>,
}

/// Check-specific configurations.
#[derive(Debug, Default, Deserialize)]
pub struct CheckConfig {
    /// Cloc (count lines of code) check configuration.
    #[serde(default)]
    pub cloc: ClocConfig,

    /// Escapes (escape hatches) check configuration.
    #[serde(default)]
    pub escapes: EscapesConfig,

    /// Agents (agent context files) check configuration.
    #[serde(default)]
    pub agents: AgentsConfig,
}

/// Escapes check configuration.
#[derive(Debug, Default, Deserialize)]
pub struct EscapesConfig {
    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Patterns to detect (overrides defaults).
    #[serde(default)]
    pub patterns: Vec<EscapePattern>,
}

/// A single escape hatch pattern definition.
#[derive(Debug, Clone, Deserialize)]
pub struct EscapePattern {
    /// Unique name for this pattern (e.g., "unwrap", "unsafe").
    pub name: String,

    /// Regex pattern to match.
    pub pattern: String,

    /// Action to take: count, comment, or forbid.
    #[serde(default)]
    pub action: EscapeAction,

    /// Required comment pattern for action = "comment".
    #[serde(default)]
    pub comment: Option<String>,

    /// Count threshold for action = "count" (default: 0).
    #[serde(default)]
    pub threshold: usize,

    /// Custom advice message for violations.
    #[serde(default)]
    pub advice: Option<String>,
}

/// Action to take when pattern is matched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EscapeAction {
    #[default]
    Forbid,
    Comment,
    Count,
}

/// Which line metric to use for size thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineMetric {
    /// Total lines (matches `wc -l`).
    #[default]
    Lines,
    /// Non-blank lines only.
    Nonblank,
}

/// Cloc check configuration.
#[derive(Debug, Deserialize)]
pub struct ClocConfig {
    /// Maximum lines per file (default: 750).
    #[serde(default = "ClocConfig::default_max_lines")]
    pub max_lines: usize,

    /// Maximum lines per test file (default: 1100).
    #[serde(default = "ClocConfig::default_max_lines_test")]
    pub max_lines_test: usize,

    /// Which line metric to compare against max_lines (default: lines).
    /// - "lines": total lines (matches `wc -l`)
    /// - "nonblank": non-blank lines only
    #[serde(default)]
    pub metric: LineMetric,

    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ClocConfig::default_test_patterns")]
    pub test_patterns: Vec<String>,

    /// Patterns to exclude from size limit checks.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Maximum tokens per file (default: 20000, None = disabled).
    #[serde(default = "ClocConfig::default_max_tokens")]
    pub max_tokens: Option<usize>,

    /// Advice message for source file violations.
    #[serde(default = "ClocConfig::default_advice")]
    pub advice: String,

    /// Advice message for test file violations.
    #[serde(default = "ClocConfig::default_advice_test")]
    pub advice_test: String,
}

impl Default for ClocConfig {
    fn default() -> Self {
        Self {
            max_lines: Self::default_max_lines(),
            max_lines_test: Self::default_max_lines_test(),
            metric: LineMetric::default(),
            check: CheckLevel::default(),
            test_patterns: Self::default_test_patterns(),
            exclude: Vec::new(),
            max_tokens: Self::default_max_tokens(),
            advice: Self::default_advice(),
            advice_test: Self::default_advice_test(),
        }
    }
}

impl ClocConfig {
    fn default_max_lines() -> usize {
        750
    }

    fn default_max_lines_test() -> usize {
        1100
    }

    fn default_max_tokens() -> Option<usize> {
        Some(20000)
    }

    fn default_test_patterns() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            "**/test_*.*".to_string(),
        ]
    }

    fn default_advice() -> String {
        "Can the code be made more concise?\n\n\
         If not, split large source files into sibling modules or submodules in a folder;\n\
         consider refactoring to be more unit testable.\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
            .to_string()
    }

    fn default_advice_test() -> String {
        "Can tests be parameterized or use shared fixtures to be more concise?\n\n\
         If not, split large test files into a folder."
            .to_string()
    }
}

/// Check level: error, warn, or off.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    #[default]
    Error,
    Warn,
    Off,
}

/// Project-level configuration.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: Option<String>,

    /// Source file patterns (default: empty = all non-test files are source).
    #[serde(default)]
    pub source: Vec<String>,

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ProjectConfig::default_test_patterns")]
    pub tests: Vec<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,
}

impl ProjectConfig {
    /// Default test patterns matching common conventions.
    fn default_test_patterns() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/benches/**".to_string(),
            "**/test_utils.*".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
        ]
    }
}

/// Ignore pattern configuration.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct IgnoreConfig {
    /// Glob patterns to ignore (e.g., "*.snapshot", "testdata/", "**/fixtures/**").
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// Currently supported config version.
pub const SUPPORTED_VERSION: i64 = 1;

/// Known top-level keys in the config.
const KNOWN_KEYS: &[&str] = &[
    "version",
    "project",
    "workspace",
    "check",
    "rust",
    "go",
    "golang",
    "shell",
];

/// Known project keys in the config.
const KNOWN_PROJECT_KEYS: &[&str] = &["name", "source", "tests", "ignore"];

/// Load and validate config from a file path.
pub fn load(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    parse(&content, path)
}

/// Load config with warnings for unknown keys.
pub fn load_with_warnings(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    parse_with_warnings(&content, path)
}

/// Parse config from string content (strict mode).
pub fn parse(content: &str, path: &Path) -> Result<Config> {
    // First check version
    let version_check: VersionOnly = toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })?;

    let version = version_check.version.ok_or_else(|| Error::Config {
        message: "missing required field: version".to_string(),
        path: Some(path.to_path_buf()),
    })?;

    if version != SUPPORTED_VERSION {
        return Err(Error::Config {
            message: format!(
                "unsupported config version {} (supported: {})\n  Upgrade quench to use this config.",
                version, SUPPORTED_VERSION
            ),
            path: Some(path.to_path_buf()),
        });
    }

    // Parse full config
    toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })
}

/// Parse config, warning on unknown keys.
pub fn parse_with_warnings(content: &str, path: &Path) -> Result<Config> {
    // First validate version
    let flexible: FlexibleConfig = toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })?;

    if flexible.version != SUPPORTED_VERSION {
        return Err(Error::Config {
            message: format!(
                "unsupported config version {} (supported: {})",
                flexible.version, SUPPORTED_VERSION
            ),
            path: Some(path.to_path_buf()),
        });
    }

    // Collect unknown keys
    let mut unknown_keys = BTreeSet::new();

    // Check top-level unknown keys
    for key in flexible.unknown.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            unknown_keys.insert(key.clone());
        }
    }

    // Warn about unknown keys
    for key in &unknown_keys {
        warn_unknown_key(path, key);
    }

    // Return a valid config with known fields
    let project = match flexible.project {
        Some(toml::Value::Table(t)) => {
            let name = t
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse source patterns
            let source = t
                .get("source")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Parse test patterns
            let tests = t
                .get("tests")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(ProjectConfig::default_test_patterns);

            // Parse ignore patterns
            let ignore = match t.get("ignore") {
                Some(toml::Value::Table(ignore_table)) => {
                    let patterns = ignore_table
                        .get("patterns")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    // Warn about unknown ignore fields
                    for key in ignore_table.keys() {
                        if key != "patterns" {
                            warn_unknown_key(path, &format!("project.ignore.{}", key));
                        }
                    }

                    IgnoreConfig { patterns }
                }
                _ => IgnoreConfig::default(),
            };

            // Warn about unknown project fields
            for key in t.keys() {
                if !KNOWN_PROJECT_KEYS.contains(&key.as_str()) {
                    warn_unknown_key(path, &format!("project.{}", key));
                }
            }

            ProjectConfig {
                name,
                source,
                tests,
                ignore,
            }
        }
        _ => ProjectConfig::default(),
    };

    // Parse workspace config
    let workspace = match flexible.workspace {
        Some(toml::Value::Table(t)) => {
            let packages = t
                .get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            WorkspaceConfig {
                packages,
                package_names: std::collections::HashMap::new(),
            }
        }
        _ => WorkspaceConfig::default(),
    };

    // Parse check config
    let check = match flexible.check {
        Some(toml::Value::Table(t)) => {
            // Known check types
            const KNOWN_CHECKS: &[&str] = &[
                "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
            ];

            // Warn about unknown check types
            for key in t.keys() {
                if !KNOWN_CHECKS.contains(&key.as_str()) {
                    warn_unknown_key(path, &format!("check.{}", key));
                }
            }

            // Parse cloc config
            let cloc = parse_cloc_config(t.get("cloc"));

            // Parse escapes config
            let escapes = parse_escapes_config(t.get("escapes"));

            // Parse agents config
            let agents = parse_agents_config(t.get("agents"));

            CheckConfig {
                cloc,
                escapes,
                agents,
            }
        }
        _ => CheckConfig::default(),
    };

    // Parse rust config
    let rust = parse_rust_config(flexible.rust.as_ref());

    // Parse go config (supports both "go" and "golang" keys)
    let go = parse_go_config(flexible.go.as_ref().or(flexible.golang.as_ref()));

    // Parse shell config
    let shell = parse_shell_config(flexible.shell.as_ref());

    Ok(Config {
        version: flexible.version,
        project,
        workspace,
        check,
        rust,
        go,
        shell,
    })
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
