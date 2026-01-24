//! Shell language-specific configuration.

use serde::Deserialize;

use super::{LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Shell language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    /// Source file patterns.
    #[serde(default = "ShellConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "ShellConfig::default_tests")]
    pub tests: Vec<String>,

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
            source: Self::default_source(),
            tests: Self::default_tests(),
            suppress: ShellSuppressConfig::default(),
            policy: ShellPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

impl ShellConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec!["**/*.sh".to_string(), "**/*.bash".to_string()]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "tests/**/*.bats".to_string(),
            "test/**/*.bats".to_string(),
            "**/*_test.sh".to_string(),
        ]
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        "Can the script be made more concise?\n\
         Look for repetitive patterns that could be extracted into helper functions.\n\
         If not, split into multiple scripts or source helper files."
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

/// Shell lint policy configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellPolicyConfig {
    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "ShellPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl ShellPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![".shellcheckrc".to_string()]
    }
}

impl crate::adapter::common::policy::PolicyConfig for ShellPolicyConfig {
    fn lint_changes(&self) -> super::LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
