// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python language-specific configuration.

use serde::Deserialize;

use super::LangClocConfig;
use super::lang_common::LanguageDefaults;

/// Python language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonConfig {
    /// Source file patterns.
    #[serde(default = "PythonDefaults::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "PythonDefaults::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "PythonDefaults::default_ignore")]
    pub ignore: Vec<String>,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    /// Custom cloc advice for source files (None = use generic default).
    /// Note: Deprecated in favor of cloc.advice.
    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            source: PythonDefaults::default_source(),
            tests: PythonDefaults::default_tests(),
            ignore: PythonDefaults::default_ignore(),
            cloc: None,
            cloc_advice: None,
        }
    }
}

/// Python language defaults.
pub struct PythonDefaults;

impl LanguageDefaults for PythonDefaults {
    fn default_source() -> Vec<String> {
        vec!["**/*.py".to_string()]
    }

    fn default_tests() -> Vec<String> {
        vec![
            "tests/**/*.py".to_string(),
            "**/test_*.py".to_string(),
            "**/*_test.py".to_string(),
            "**/conftest.py".to_string(),
        ]
    }

    fn default_ignore() -> Vec<String> {
        vec![
            ".venv/**".to_string(),
            "venv/**".to_string(),
            "__pycache__/**".to_string(),
            ".mypy_cache/**".to_string(),
            ".pytest_cache/**".to_string(),
            "dist/**".to_string(),
            "build/**".to_string(),
            "*.egg-info/**".to_string(),
            ".tox/**".to_string(),
            ".nox/**".to_string(),
        ]
    }

    fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         Look for repetitive patterns that could be extracted into helper functions.\n\n\
         If not, split large modules into submodules using packages (directories with __init__.py).\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}

impl PythonConfig {
    pub(crate) fn default_source() -> Vec<String> {
        PythonDefaults::default_source()
    }

    pub(crate) fn default_tests() -> Vec<String> {
        PythonDefaults::default_tests()
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        PythonDefaults::default_ignore()
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        PythonDefaults::default_cloc_advice()
    }
}
