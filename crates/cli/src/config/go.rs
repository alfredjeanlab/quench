//! Go language-specific configuration.

use serde::Deserialize;

use super::{LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Go language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GoConfig {
    /// Source file patterns.
    #[serde(default = "GoConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "GoConfig::default_tests")]
    pub tests: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: GoSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: GoPolicyConfig,
}

impl Default for GoConfig {
    fn default() -> Self {
        Self {
            source: Self::default_source(),
            tests: Self::default_tests(),
            suppress: GoSuppressConfig::default(),
            policy: GoPolicyConfig::default(),
        }
    }
}

impl GoConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec!["**/*.go".to_string()]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec!["**/*_test.go".to_string()]
    }
}

/// Go suppress configuration (defaults to "comment" like Rust).
#[derive(Debug, Clone, Deserialize)]
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

/// Go lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GoPolicyConfig {
    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "GoPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for GoPolicyConfig {
    fn default() -> Self {
        Self {
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl GoPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            ".golangci.yml".to_string(),
            ".golangci.yaml".to_string(),
            ".golangci.toml".to_string(),
        ]
    }
}

impl crate::adapter::common::policy::PolicyConfig for GoPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
