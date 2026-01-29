// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell language-specific configuration.

use serde::Deserialize;

use super::lang_common::{LanguageDefaults, define_policy_config};
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Shell language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    /// Source file patterns.
    #[serde(default = "ShellDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "ShellDefaults::default_tests")]
    pub tests: Vec<String>,

    /// Exclude patterns (walker-level: prevents I/O on subtrees).
    #[serde(default = "ShellDefaults::default_exclude", alias = "ignore")]
    pub exclude: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: ShellSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: ShellPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            source: ShellDefaults::default_source(),
            tests: ShellDefaults::default_tests(),
            exclude: ShellDefaults::default_exclude(),
            suppress: ShellSuppressConfig::default(),
            policy: ShellPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// Shell language defaults.
pub struct ShellDefaults;

impl LanguageDefaults for ShellDefaults {
    fn default_source() -> Vec<String> {
        vec!["**/*.sh".to_string(), "**/*.bash".to_string()]
    }

    fn default_tests() -> Vec<String> {
        vec![
            "**/tests/**/*.bats".to_string(),
            "**/test/**/*.bats".to_string(),
            "**/*_test.sh".to_string(),
        ]
    }

    fn default_exclude() -> Vec<String> {
        vec![]
    }

    fn default_cloc_advice(threshold: usize) -> String {
        let range = super::defaults::advice::target_range(threshold);
        format!(
            "First, look for repetitive patterns that could be extracted \
into helper functions.\n\
\n\
Then split into multiple scripts or source helper files by semantic concern \
(target {range} each).\n\
\n\
Avoid removing individual lines to satisfy the linter; \
prefer extracting testable code blocks."
        )
    }
}

impl ShellConfig {
    pub(crate) fn default_source() -> Vec<String> {
        ShellDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        ShellDefaults::default_tests()
    }

    pub(crate) fn default_exclude() -> Vec<String> {
        ShellDefaults::default_exclude()
    }

    pub(crate) fn default_cloc_advice(threshold: usize) -> String {
        ShellDefaults::default_cloc_advice(threshold)
    }
}

/// Shell suppress configuration (defaults to "forbid" unlike Rust's "comment").
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellSuppressConfig {
    /// Check level: forbid, comment, or allow (default: "forbid").
    #[serde(default = "ShellSuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "ShellSuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl Default for ShellSuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: Self::default_source(),
            test: Self::default_test(),
        }
    }
}

impl ShellSuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Forbid // Shell defaults to forbid, not comment
    }

    pub(crate) fn default_source() -> SuppressScopeConfig {
        // Shell has no default patterns (defaults to forbid anyway)
        SuppressScopeConfig {
            check: None,
            allow: Vec::new(),
            forbid: Vec::new(),
            patterns: std::collections::HashMap::new(),
        }
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

define_policy_config!(ShellPolicyConfig, [".shellcheckrc",]);
