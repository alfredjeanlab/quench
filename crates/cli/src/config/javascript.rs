//! JavaScript/TypeScript language-specific configuration.

use serde::Deserialize;

use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressConfig};

/// JavaScript/TypeScript language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaScriptConfig {
    /// Source file patterns.
    #[serde(default = "JavaScriptConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "JavaScriptConfig::default_tests")]
    pub tests: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: SuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: JavaScriptPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for JavaScriptConfig {
    fn default() -> Self {
        Self {
            source: Self::default_source(),
            tests: Self::default_tests(),
            suppress: SuppressConfig::default(),
            policy: JavaScriptPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

impl JavaScriptConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec![
            "**/*.js".to_string(),
            "**/*.jsx".to_string(),
            "**/*.ts".to_string(),
            "**/*.tsx".to_string(),
            "**/*.mjs".to_string(),
            "**/*.mts".to_string(),
            "**/*.cjs".to_string(),
            "**/*.cts".to_string(),
        ]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/__tests__/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            "**/test_*.*".to_string(),
        ]
    }
}

/// JavaScript/TypeScript lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaScriptPolicyConfig {
    /// Check level: "error" | "warn" | "off" (default: inherits from global).
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Lint changes policy.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Lint config files (default: ESLint, TSConfig, Biome, Prettier).
    #[serde(default = "JavaScriptPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for JavaScriptPolicyConfig {
    fn default() -> Self {
        Self {
            check: None,
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl JavaScriptPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            ".eslintrc".to_string(),
            ".eslintrc.js".to_string(),
            ".eslintrc.json".to_string(),
            ".eslintrc.yml".to_string(),
            "eslint.config.js".to_string(),
            "eslint.config.mjs".to_string(),
            "tsconfig.json".to_string(),
            ".prettierrc".to_string(),
            ".prettierrc.json".to_string(),
            "prettier.config.js".to_string(),
            "biome.json".to_string(),
            "biome.jsonc".to_string(),
        ]
    }
}

impl crate::adapter::common::policy::PolicyConfig for JavaScriptPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
