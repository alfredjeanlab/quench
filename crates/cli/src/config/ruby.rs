//! Ruby language-specific configuration.

use serde::Deserialize;

use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Ruby language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyConfig {
    /// Source file patterns.
    #[serde(default = "RubyConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "RubyConfig::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "RubyConfig::default_ignore")]
    pub ignore: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: RubySuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RubyPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for RubyConfig {
    fn default() -> Self {
        Self {
            source: Self::default_source(),
            tests: Self::default_tests(),
            ignore: Self::default_ignore(),
            suppress: RubySuppressConfig::default(),
            policy: RubyPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec![
            "**/*.rb".to_string(),
            "**/*.rake".to_string(),
            "Rakefile".to_string(),
            "Gemfile".to_string(),
            "*.gemspec".to_string(),
        ]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "spec/**/*_spec.rb".to_string(),
            "test/**/*_test.rb".to_string(),
            "test/**/test_*.rb".to_string(),
            "features/**/*.rb".to_string(),
        ]
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        vec![
            "vendor/".to_string(),
            "tmp/".to_string(),
            "log/".to_string(),
            "coverage/".to_string(),
        ]
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\
         Look for repetitive patterns that could be extracted into helper methods.\n\
         Consider using Ruby's built-in enumerable methods for cleaner code.\n\
         If not, split into smaller classes or modules."
    }
}

/// Ruby suppress configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubySuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "RubySuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "RubySuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl Default for RubySuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: SuppressScopeConfig::default(),
            test: Self::default_test(),
        }
    }
}

impl RubySuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Comment // Ruby defaults to comment (require justification)
    }

    pub(crate) fn default_test() -> SuppressScopeConfig {
        SuppressScopeConfig {
            check: Some(SuppressLevel::Allow),
            allow: Vec::new(),
            forbid: Vec::new(),
            patterns: std::collections::HashMap::new(),
        }
    }
}

/// Ruby lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyPolicyConfig {
    /// Check level: "error" | "warn" | "off" (default: inherits from global).
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "RubyPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for RubyPolicyConfig {
    fn default() -> Self {
        Self {
            check: None,
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl RubyPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            ".rubocop.yml".to_string(),
            ".rubocop_todo.yml".to_string(),
            ".standard.yml".to_string(),
        ]
    }
}

impl crate::adapter::common::policy::PolicyConfig for RubyPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
