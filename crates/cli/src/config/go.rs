// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go language-specific configuration.

use serde::Deserialize;

use super::lang_common::{LanguageDefaults, define_policy_config};
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Go language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoConfig {
    /// Source file patterns.
    #[serde(default = "GoDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "GoDefaults::default_tests")]
    pub tests: Vec<String>,

    /// Exclude patterns (walker-level: prevents I/O on subtrees).
    #[serde(default = "GoDefaults::default_exclude", alias = "ignore")]
    pub exclude: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: GoSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: GoPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for GoConfig {
    fn default() -> Self {
        Self {
            source: GoDefaults::default_source(),
            tests: GoDefaults::default_tests(),
            exclude: GoDefaults::default_exclude(),
            suppress: GoSuppressConfig::default(),
            policy: GoPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// Go language defaults.
pub struct GoDefaults;

impl LanguageDefaults for GoDefaults {
    fn default_source() -> Vec<String> {
        vec!["**/*.go".to_string()]
    }

    fn default_tests() -> Vec<String> {
        vec!["**/*_test.go".to_string()]
    }

    fn default_exclude() -> Vec<String> {
        vec!["vendor/**".to_string()]
    }

    fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         Look for repetitive patterns that could be extracted into helper functions.\n\n\
         If not, split large files into multiple files in the same package,\n\
         or extract reusable logic into internal packages.\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}

impl GoConfig {
    pub(crate) fn default_source() -> Vec<String> {
        GoDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        GoDefaults::default_tests()
    }

    pub(crate) fn default_exclude() -> Vec<String> {
        GoDefaults::default_exclude()
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        GoDefaults::default_cloc_advice()
    }
}

/// Go suppress configuration (defaults to "comment" like Rust).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoSuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "GoSuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "GoSuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl Default for GoSuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: Self::default_source(),
            test: Self::default_test(),
        }
    }
}

impl GoSuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Comment // Go defaults to comment, like Rust
    }

    pub(crate) fn default_source() -> SuppressScopeConfig {
        // Go has no default per-lint patterns yet
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

define_policy_config!(
    GoPolicyConfig,
    [".golangci.yml", ".golangci.yaml", ".golangci.toml",]
);
