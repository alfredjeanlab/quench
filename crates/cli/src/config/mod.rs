// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Configuration parsing and validation.
//!
//! Handles quench.toml parsing with version validation and unknown key warnings.

mod checks;
pub mod defaults;
pub mod duration;
mod go;
mod javascript;
mod lang_common;
mod python;
mod ratchet;
mod ruby;
mod shell;
mod suppress;
mod test_config;

use std::path::Path;

use serde::Deserialize;

pub use checks::CheckLevel;

use crate::error::{Error, Result};

pub(crate) use checks::{
    ClocConfig, DocsAreaConfig, DocsCommitConfig, DocsConfig, EscapeAction, EscapePattern,
    EscapesConfig, LangClocConfig, LineMetric, SpecsConfig, SpecsSectionsConfig,
};
pub(crate) use go::{GoConfig, GoPolicyConfig, GoSuppressConfig};
pub(crate) use javascript::{JavaScriptConfig, JavaScriptPolicyConfig};
pub(crate) use python::{PythonConfig, PythonPolicyConfig, PythonSuppressConfig};
pub(crate) use ratchet::RatchetConfig;
#[cfg(test)]
pub(crate) use ratchet::RatchetPackageConfig;
pub(crate) use ruby::{RubyConfig, RubyPolicyConfig, RubySuppressConfig};
pub(crate) use shell::{ShellConfig, ShellPolicyConfig, ShellSuppressConfig};
pub(crate) use suppress::{SuppressConfig, SuppressLevel, SuppressScopeConfig};
pub(crate) use test_config::{TestSuiteConfig, TestsCommitConfig, TestsConfig};

pub(crate) use crate::checks::agents::config::{
    AgentsConfig, ContentRule, RequiredSection, SectionsConfig, deserialize_optional_usize,
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

    /// Git configuration.
    #[serde(default)]
    pub git: GitConfig,

    /// Ratcheting configuration.
    #[serde(default)]
    pub ratchet: RatchetConfig,

    /// Rust-specific configuration.
    #[serde(default)]
    pub rust: RustConfig,

    /// Go-specific configuration.
    #[serde(default)]
    pub golang: GoConfig,

    /// JavaScript/TypeScript-specific configuration.
    #[serde(default)]
    pub javascript: JavaScriptConfig,

    /// Python-specific configuration.
    #[serde(default)]
    pub python: PythonConfig,

    /// Ruby-specific configuration.
    #[serde(default)]
    pub ruby: RubyConfig,

    /// Shell-specific configuration.
    #[serde(default)]
    pub shell: ShellConfig,
}

/// Git configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitConfig {
    /// Baseline file path for ratcheting.
    #[serde(default = "GitConfig::default_baseline")]
    pub baseline: String,

    /// Commit message validation settings.
    #[serde(default)]
    pub commit: GitCommitConfig,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            baseline: Self::default_baseline(),
            commit: GitCommitConfig::default(),
        }
    }
}

impl GitConfig {
    fn default_baseline() -> String {
        "notes".to_string()
    }

    /// Check if baseline is configured to use git notes.
    pub fn uses_notes(&self) -> bool {
        self.baseline == "notes"
    }

    /// Get baseline file path (returns None if using notes mode).
    pub fn baseline_path(&self) -> Option<&str> {
        if self.uses_notes() {
            None
        } else {
            Some(&self.baseline)
        }
    }
}

/// Git commit message configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitCommitConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Commit format: "conventional" | "none" (default: "conventional")
    pub format: Option<String>,

    /// Allowed commit types (None = use defaults, Some([]) = any type)
    pub types: Option<Vec<String>>,

    /// Allowed scopes (None = any scope allowed)
    pub scopes: Option<Vec<String>>,

    /// Check that commit format is documented in agent files (default: true)
    pub agents: bool,

    /// Create .gitmessage template with --fix (default: true)
    pub template: bool,

    /// Skip merge commits (e.g., "Merge branch 'x'") (default: true)
    pub skip_merge: bool,
}

impl Default for GitCommitConfig {
    fn default() -> Self {
        Self {
            check: None,
            format: None,
            types: None,
            scopes: None,
            agents: true,
            template: true,
            skip_merge: true,
        }
    }
}

impl GitCommitConfig {
    /// Get effective format (default: "conventional").
    pub fn effective_format(&self) -> &str {
        self.format.as_deref().unwrap_or("conventional")
    }
}

impl Config {
    /// Get effective cloc check level for a language.
    ///
    /// Resolution order:
    /// 1. {lang}.cloc.check if set
    /// 2. check.cloc.check (global default)
    ///
    /// The `language` parameter can be either an adapter name (e.g., "rust")
    /// or a file extension (e.g., "rs"). This allows per-file language detection
    /// even in mixed-language projects where only the primary adapter is registered.
    pub fn cloc_check_level_for_language(&self, language: &str) -> CheckLevel {
        let lang_level = match language {
            // Adapter names and file extensions combined
            // (Go uses "go" for both adapter name and file extension)
            "rust" | "rs" => self.rust.cloc.as_ref().and_then(|c| c.check),
            "go" => self.golang.cloc.as_ref().and_then(|c| c.check),
            "javascript" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts" => {
                self.javascript.cloc.as_ref().and_then(|c| c.check)
            }
            "python" | "py" => self.python.cloc.as_ref().and_then(|c| c.check),
            "shell" | "sh" | "bash" | "zsh" | "fish" | "bats" => {
                self.shell.cloc.as_ref().and_then(|c| c.check)
            }
            "ruby" | "rb" | "rake" => self.ruby.cloc.as_ref().and_then(|c| c.check),
            _ => None,
        };
        lang_level.unwrap_or(self.check.cloc.check)
    }

    /// Get cloc advice for source files, checking user override then language default.
    ///
    /// Resolution order:
    /// 1. {lang}.cloc.advice if set
    /// 2. {lang}.cloc_advice if set (deprecated)
    /// 3. check.cloc.advice (global) if different from default
    /// 4. Language-specific default advice
    ///
    /// The `language` parameter can be either an adapter name or a file extension.
    pub fn cloc_advice_for_language(&self, language: &str) -> &str {
        // Check language-specific advice first
        let lang_advice = match language {
            "rust" | "rs" => self
                .rust
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.rust.cloc_advice.as_deref()),
            "go" => self
                .golang
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.golang.cloc_advice.as_deref()),
            "javascript" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts" => self
                .javascript
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.javascript.cloc_advice.as_deref()),
            "python" | "py" => self
                .python
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.python.cloc_advice.as_deref()),
            "ruby" | "rb" | "rake" => self
                .ruby
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.ruby.cloc_advice.as_deref()),
            "shell" | "sh" | "bash" | "zsh" | "fish" | "bats" => self
                .shell
                .cloc
                .as_ref()
                .and_then(|c| c.advice.as_deref())
                .or(self.shell.cloc_advice.as_deref()),
            _ => None,
        };

        // If language-specific advice is set, use it
        if let Some(advice) = lang_advice {
            return advice;
        }

        // Check if global advice differs from default (user customized it)
        let default_advice = ClocConfig::default_advice();
        if self.check.cloc.advice != default_advice {
            return &self.check.cloc.advice;
        }

        // Use language-specific defaults
        match language {
            "rust" | "rs" => RustConfig::default_cloc_advice(),
            "go" => GoConfig::default_cloc_advice(),
            "python" | "py" => PythonConfig::default_cloc_advice(),
            "ruby" | "rb" | "rake" => RubyConfig::default_cloc_advice(),
            "shell" | "sh" | "bash" | "zsh" | "fish" | "bats" => ShellConfig::default_cloc_advice(),
            _ => &self.check.cloc.advice,
        }
    }

    /// Get effective policy check level for a language.
    ///
    /// Resolution order:
    /// 1. {lang}.policy.check if set
    /// 2. CheckLevel::Error (default - policy violations fail)
    ///
    /// The `language` parameter should be an adapter name (e.g., "rust", "go").
    pub fn policy_check_level_for_language(&self, language: &str) -> CheckLevel {
        let lang_level = match language {
            "rust" => self.rust.policy.check,
            "go" | "golang" => self.golang.policy.check,
            "javascript" | "js" => self.javascript.policy.check,
            "python" | "py" => self.python.policy.check,
            "ruby" | "rb" => self.ruby.policy.check,
            "shell" | "sh" => self.shell.policy.check,
            _ => None,
        };
        lang_level.unwrap_or(CheckLevel::Error)
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

use lang_common::{LanguageDefaults, define_policy_config};

/// Rust language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustConfig {
    /// Source file patterns.
    #[serde(default = "RustDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "RustDefaults::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "RustDefaults::default_ignore")]
    pub ignore: Vec<String>,

    /// How to handle #[cfg(test)] blocks (default: "count").
    #[serde(default)]
    pub cfg_test_split: CfgTestSplitMode,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: SuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RustPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for RustConfig {
    fn default() -> Self {
        Self {
            source: RustDefaults::default_source(),
            tests: RustDefaults::default_tests(),
            ignore: RustDefaults::default_ignore(),
            cfg_test_split: CfgTestSplitMode::default(),
            suppress: SuppressConfig::default(),
            policy: RustPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// Rust language defaults.
pub struct RustDefaults;

impl LanguageDefaults for RustDefaults {
    fn default_source() -> Vec<String> {
        vec!["**/*.rs".to_string()]
    }

    fn default_tests() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**/*.rs".to_string(),
            "**/benches/**".to_string(),
            "**/*_test.rs".to_string(),
            "**/*_tests.rs".to_string(),
        ]
    }

    fn default_ignore() -> Vec<String> {
        vec!["target/**".to_string()]
    }

    fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         Look for repetitive patterns that could be extracted into helper functions\n\
         or consider refactoring to be more unit testable.\n\n\
         If not, split large source files into sibling modules or submodules in a folder,\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}

impl RustConfig {
    pub(crate) fn default_source() -> Vec<String> {
        RustDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        RustDefaults::default_tests()
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        RustDefaults::default_ignore()
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        RustDefaults::default_cloc_advice()
    }
}

define_policy_config!(
    RustPolicyConfig,
    [
        "rustfmt.toml",
        ".rustfmt.toml",
        "clippy.toml",
        ".clippy.toml",
    ]
);

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

    /// Build check configuration.
    #[serde(default)]
    pub build: BuildConfig,
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

    /// File patterns to check, organized by language.
    /// Example: { rust = ["**/*.rs"], shell = ["**/*.sh"] }
    #[serde(default)]
    pub patterns: std::collections::HashMap<String, Vec<String>>,

    /// Exclude patterns (files matching these won't be checked).
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Build check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Explicit targets to measure (auto-detected if not specified).
    #[serde(default)]
    pub targets: Vec<String>,

    /// Global maximum binary size (e.g., "10 MB").
    pub size_max: Option<String>,

    /// Maximum cold build time (e.g., "60s").
    pub time_cold_max: Option<String>,

    /// Maximum hot build time (e.g., "5s").
    pub time_hot_max: Option<String>,

    /// Per-target configuration.
    #[serde(default)]
    pub target: std::collections::HashMap<String, BuildTargetConfig>,
}

/// Per-target build configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BuildTargetConfig {
    /// Maximum binary size for this target.
    pub size_max: Option<String>,
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

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod suppress_tests;
