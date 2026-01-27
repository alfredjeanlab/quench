//! JavaScript/TypeScript language-specific configuration.

use serde::Deserialize;

use super::lang_common::{LanguageDefaults, define_policy_config};
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressConfig};

/// JavaScript/TypeScript language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaScriptConfig {
    /// Source file patterns.
    #[serde(default = "JavaScriptDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "JavaScriptDefaults::default_tests")]
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
            source: JavaScriptDefaults::default_source(),
            tests: JavaScriptDefaults::default_tests(),
            suppress: SuppressConfig::default(),
            policy: JavaScriptPolicyConfig::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// JavaScript/TypeScript language defaults.
pub struct JavaScriptDefaults;

impl LanguageDefaults for JavaScriptDefaults {
    fn default_source() -> Vec<String> {
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

    fn default_tests() -> Vec<String> {
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

    fn default_ignore() -> Vec<String> {
        vec![
            "node_modules/**".to_string(),
            "dist/**".to_string(),
            "build/**".to_string(),
            ".next/**".to_string(),
            "coverage/**".to_string(),
        ]
    }
}

impl JavaScriptConfig {
    pub(crate) fn default_source() -> Vec<String> {
        JavaScriptDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        JavaScriptDefaults::default_tests()
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        JavaScriptDefaults::default_ignore()
    }
}

define_policy_config!(
    JavaScriptPolicyConfig,
    [
        ".eslintrc",
        ".eslintrc.js",
        ".eslintrc.json",
        ".eslintrc.yml",
        "eslint.config.js",
        "eslint.config.mjs",
        "tsconfig.json",
        ".prettierrc",
        ".prettierrc.json",
        "prettier.config.js",
        "biome.json",
        "biome.jsonc",
    ]
);
