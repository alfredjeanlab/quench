//! Ruby language-specific configuration.

use serde::Deserialize;

use super::lang_common::{LanguageDefaults, define_policy_config};
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Ruby language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyConfig {
    /// Source file patterns.
    #[serde(default = "RubyDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "RubyDefaults::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "RubyDefaults::default_ignore")]
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
            source: RubyDefaults::default_source(),
            tests: RubyDefaults::default_tests(),
            ignore: RubyDefaults::default_ignore(),
            suppress: RubySuppressConfig::default(),
            policy: RubyPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// Ruby language defaults.
pub struct RubyDefaults;

impl LanguageDefaults for RubyDefaults {
    fn default_source() -> Vec<String> {
        vec![
            "**/*.rb".to_string(),
            "**/*.rake".to_string(),
            "Rakefile".to_string(),
            "Gemfile".to_string(),
            "*.gemspec".to_string(),
        ]
    }

    fn default_tests() -> Vec<String> {
        vec![
            "spec/**/*_spec.rb".to_string(),
            "test/**/*_test.rb".to_string(),
            "test/**/test_*.rb".to_string(),
            "features/**/*.rb".to_string(),
        ]
    }

    fn default_ignore() -> Vec<String> {
        vec![
            "vendor/".to_string(),
            "tmp/".to_string(),
            "log/".to_string(),
            "coverage/".to_string(),
        ]
    }

    fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\
         Look for repetitive patterns that could be extracted into helper methods.\n\
         Consider using Ruby's built-in enumerable methods for cleaner code.\n\
         If not, split into smaller classes or modules."
    }
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        RubyDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        RubyDefaults::default_tests()
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        RubyDefaults::default_ignore()
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        RubyDefaults::default_cloc_advice()
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

define_policy_config!(
    RubyPolicyConfig,
    [".rubocop.yml", ".rubocop_todo.yml", ".standard.yml",]
);
