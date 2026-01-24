// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Configuration parsing and validation.
//!
//! Handles quench.toml parsing with version validation and unknown key warnings.

mod checks;
mod go;
mod javascript;
mod shell;
mod suppress;

use std::path::Path;

use serde::Deserialize;

pub use checks::{
    CheckLevel, ClocConfig, DocsAreaConfig, DocsCommitConfig, DocsConfig, EscapeAction,
    EscapePattern, EscapesConfig, LineMetric, LinksConfig, SpecsConfig, SpecsSectionsConfig,
    TocConfig,
};
pub use go::{GoConfig, GoPolicyConfig, GoSuppressConfig};
pub use javascript::{JavaScriptConfig, JavaScriptPolicyConfig};
pub use shell::{ShellConfig, ShellPolicyConfig, ShellSuppressConfig};
pub use suppress::{SuppressConfig, SuppressLevel, SuppressScopeConfig};

use crate::error::{Error, Result};

pub use crate::checks::agents::config::{
    AgentsConfig, AgentsScopeConfig, ContentRule, RequiredSection, SectionsConfig,
    deserialize_optional_usize,
};

/// Minimum config structure for version checking.
#[derive(Deserialize)]
struct VersionOnly {
    version: Option<i64>,
}

/// Full configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config file version (must be 1).
    pub version: i64,

    /// Project configuration.
    #[serde(default)]
    pub project: ProjectConfig,

    /// Check configurations.
    #[serde(default)]
    pub check: CheckConfig,

    /// Rust-specific configuration.
    #[serde(default)]
    pub rust: RustConfig,

    /// Go-specific configuration.
    #[serde(default)]
    pub golang: GoConfig,

    /// JavaScript/TypeScript-specific configuration.
    #[serde(default)]
    pub javascript: JavaScriptConfig,

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
                .golang
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

/// Mode for handling #[cfg(test)] blocks in Rust files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CfgTestSplitMode {
    /// Split #[cfg(test)] blocks into test LOC (default).
    #[default]
    Count,
    /// Fail if source files contain inline #[cfg(test)] blocks.
    Require,
    /// Count all lines as source LOC, don't parse for #[cfg(test)].
    Off,
}

/// Rust language-specific configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustConfig {
    /// How to handle #[cfg(test)] blocks (default: "count").
    #[serde(default)]
    pub cfg_test_split: CfgTestSplitMode,

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

impl RustConfig {
    pub(crate) fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         Look for repetitive patterns that could be extracted into helper functions\n\
         or consider refactoring to be more unit testable.\n\n\
         If not, split large source files into sibling modules or submodules in a folder,\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}

/// Rust lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Check-specific configurations.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
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

    /// Docs (documentation validation) check configuration.
    #[serde(default)]
    pub docs: DocsConfig,

    /// Tests check configuration.
    #[serde(default)]
    pub tests: TestsConfig,

    /// License check configuration.
    #[serde(default)]
    pub license: LicenseConfig,
}

/// Tests check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Commit message validation settings.
    #[serde(default)]
    pub commit: TestsCommitConfig,
}

/// Tests commit check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsCommitConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,
}

/// License check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LicenseConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// License identifier (e.g., "MIT", "Apache-2.0").
    pub license: Option<String>,

    /// Copyright holder.
    pub copyright: Option<String>,
}

/// Project-level configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Project name.
    pub name: Option<String>,

    /// Source file patterns (default: empty = all non-test files are source).
    #[serde(default)]
    pub source: Vec<String>,

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ProjectConfig::default_test_patterns")]
    pub tests: Vec<String>,

    /// Package directories for multi-package projects (e.g., workspace members).
    #[serde(default)]
    pub packages: Vec<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,

    /// Package name lookup (path -> name).
    /// Auto-populated when detecting workspaces; not user-configurable.
    #[serde(default, skip_serializing)]
    pub package_names: std::collections::HashMap<String, String>,
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
///
/// Accepts either shorthand or full form:
/// - `ignore = ["pattern1", "pattern2"]`
/// - `ignore = { patterns = ["pattern1", "pattern2"] }`
#[derive(Debug, Default, Clone)]
pub struct IgnoreConfig {
    pub patterns: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum IgnoreConfigHelper {
    Short(Vec<String>),
    Full { patterns: Vec<String> },
}

impl<'de> serde::Deserialize<'de> for IgnoreConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match IgnoreConfigHelper::deserialize(deserializer)? {
            IgnoreConfigHelper::Short(patterns) | IgnoreConfigHelper::Full { patterns } => {
                Self { patterns }
            }
        })
    }
}

/// Currently supported config version.
pub const SUPPORTED_VERSION: i64 = 1;

/// Load and validate config from a file path.
pub fn load(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    parse(&content, path)
}

/// Load config with warnings for unknown keys.
///
/// NOTE: Unknown keys now cause hard errors (via #[serde(deny_unknown_fields)]).
/// This function exists for backward compatibility but behaves identically to `load()`.
pub fn load_with_warnings(path: &Path) -> Result<Config> {
    load(path)
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

/// Parse config with warnings for unknown keys.
///
/// NOTE: Unknown keys now cause hard errors (via #[serde(deny_unknown_fields)]).
/// This function exists for backward compatibility but behaves identically to `parse()`.
pub fn parse_with_warnings(content: &str, path: &Path) -> Result<Config> {
    parse(content, path)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
