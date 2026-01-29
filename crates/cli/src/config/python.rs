// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python language-specific configuration.

use serde::Deserialize;

use super::lang_common::{LanguageDefaults, define_policy_config};
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

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

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: PythonSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: PythonPolicyConfig,

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
            suppress: PythonSuppressConfig::default(),
            policy: PythonPolicyConfig::default(),
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
            "**/tests/**/*.py".to_string(),
            "test/**/*.py".to_string(),
            "**/test/**/*.py".to_string(),
            "**/test_*.py".to_string(),
            "**/*_test.py".to_string(),
            "**/conftest.py".to_string(),
        ]
    }

    fn default_ignore() -> Vec<String> {
        vec![
            ".venv/**".to_string(),
            "venv/**".to_string(),
            ".env/**".to_string(),
            "env/**".to_string(),
            "__pycache__/**".to_string(),
            ".mypy_cache/**".to_string(),
            ".pytest_cache/**".to_string(),
            ".ruff_cache/**".to_string(),
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
         Consider using Python's built-in functions and comprehensions for cleaner code.\n\n\
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

/// Python suppress configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "PythonSuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "PythonSuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl Default for PythonSuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: SuppressScopeConfig::default(),
            test: Self::default_test(),
        }
    }
}

impl PythonSuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Comment // Python defaults to comment (require justification)
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

// Python lint config files detection.
//
// Detects configuration for common Python linting tools:
// - Ruff (modern, fast): ruff.toml, .ruff.toml, pyproject.toml [tool.ruff]
// - Black (formatter): pyproject.toml [tool.black]
// - Flake8 (legacy): .flake8, setup.cfg [flake8]
// - Pylint (comprehensive): .pylintrc, pylintrc, pyproject.toml [tool.pylint]
// - Mypy (type checker): mypy.ini, .mypy.ini, pyproject.toml [tool.mypy]
//
// Note: pyproject.toml and setup.cfg are included because they often contain
// lint tool configuration sections. The policy check uses filename matching,
// so any change to these files triggers the standalone requirement when
// lint_changes = "standalone" is set.
define_policy_config!(
    PythonPolicyConfig,
    [
        // Ruff
        "ruff.toml",
        ".ruff.toml",
        // Flake8
        ".flake8",
        // Pylint
        ".pylintrc",
        "pylintrc",
        // Mypy
        "mypy.ini",
        ".mypy.ini",
        // Multi-tool config files (pyproject.toml contains [tool.ruff/black/mypy/pylint])
        "pyproject.toml",
        // Legacy multi-tool config (setup.cfg contains [flake8], [mypy] sections)
        "setup.cfg",
    ]
);
