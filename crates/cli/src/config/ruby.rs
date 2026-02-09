// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

    /// Exclude patterns (walker-level: prevents I/O on subtrees).
    #[serde(default = "RubyDefaults::default_exclude", alias = "ignore")]
    pub exclude: Vec<String>,

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
            exclude: RubyDefaults::default_exclude(),
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

    fn default_exclude() -> Vec<String> {
        vec!["vendor/".to_string(), "tmp/".to_string(), "log/".to_string(), "coverage/".to_string()]
    }

    fn default_cloc_advice(threshold: usize) -> String {
        let range = super::defaults::advice::target_range(threshold);
        format!(
            "First, look for repetitive patterns that could be extracted into helper \
methods. Consider using built-in enumerable methods for cleaner code.\n\
\n\
Then split into smaller classes or modules by semantic concern \
(target {range} each).\n\
\n\
Avoid removing individual lines to satisfy the linter; \
prefer extracting testable code blocks."
        )
    }
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        RubyDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        RubyDefaults::default_tests()
    }

    pub(crate) fn default_exclude() -> Vec<String> {
        RubyDefaults::default_exclude()
    }

    pub(crate) fn default_cloc_advice(threshold: usize) -> String {
        RubyDefaults::default_cloc_advice(threshold)
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

define_policy_config!(RubyPolicyConfig, [".rubocop.yml", ".rubocop_todo.yml", ".standard.yml",]);
