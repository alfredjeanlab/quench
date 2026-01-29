// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Check-specific configuration structures.

use std::collections::HashMap;

use serde::Deserialize;
use serde::de::{self, Deserializer};

use crate::config::{ContentRule, RequiredSection, deserialize_optional_usize};

/// Documentation check configuration.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// TOC validation settings.
    #[serde(default)]
    pub toc: TocConfig,

    /// Link validation settings.
    #[serde(default)]
    pub links: LinksConfig,

    /// Specs directory validation settings.
    #[serde(default)]
    pub specs: SpecsConfig,

    /// Commit checking configuration (CI mode).
    #[serde(default)]
    pub commit: DocsCommitConfig,

    /// Area mappings for scoped commit requirements.
    #[serde(default)]
    pub area: HashMap<String, DocsAreaConfig>,
}

/// Configuration for commit checking in CI mode.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocsCommitConfig {
    /// Check level: "error" | "warn" | "off" (default: "off")
    #[serde(default = "DocsCommitConfig::default_check")]
    pub check: String,

    /// Commit types that require documentation.
    /// Default: ["feat", "feature", "story", "breaking"]
    #[serde(default = "DocsCommitConfig::default_types")]
    pub types: Vec<String>,
}

impl Default for DocsCommitConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            types: Self::default_types(),
        }
    }
}

impl DocsCommitConfig {
    fn default_check() -> String {
        "off".to_string()
    }

    fn default_types() -> Vec<String> {
        vec![
            "feat".to_string(),
            "feature".to_string(),
            "story".to_string(),
            "breaking".to_string(),
        ]
    }
}

/// Area mapping for scoped commits.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsAreaConfig {
    /// Required docs pattern (glob).
    pub docs: String,

    /// Source files that trigger this area (optional glob).
    #[serde(default)]
    pub source: Option<String>,
}

/// Configuration for TOC validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TocConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Include patterns for markdown files.
    #[serde(default = "TocConfig::default_include")]
    pub include: Vec<String>,

    /// Exclude patterns (plans, etc.).
    #[serde(default = "TocConfig::default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            check: None,
            include: Self::default_include(),
            exclude: Self::default_exclude(),
        }
    }
}

impl TocConfig {
    pub(super) fn default_include() -> Vec<String> {
        vec!["**/*.md".to_string(), "**/*.mdc".to_string()]
    }

    pub(super) fn default_exclude() -> Vec<String> {
        vec![
            "plans/**".to_string(),
            "plan.md".to_string(),
            "*_plan.md".to_string(),
            "plan_*".to_string(),
            "**/fixtures/**".to_string(),
            "**/testdata/**".to_string(),
        ]
    }
}

/// Configuration for link validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LinksConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Include patterns for markdown files.
    #[serde(default = "LinksConfig::default_include")]
    pub include: Vec<String>,

    /// Exclude patterns (plans, etc.).
    #[serde(default = "LinksConfig::default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            check: None,
            include: Self::default_include(),
            exclude: Self::default_exclude(),
        }
    }
}

impl LinksConfig {
    pub(super) fn default_include() -> Vec<String> {
        vec!["**/*.md".to_string(), "**/*.mdc".to_string()]
    }

    pub(super) fn default_exclude() -> Vec<String> {
        vec![
            "plans/**".to_string(),
            "plan.md".to_string(),
            "*_plan.md".to_string(),
            "plan_*".to_string(),
            "**/fixtures/**".to_string(),
            "**/testdata/**".to_string(),
        ]
    }
}

/// Configuration for specs directory validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpecsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Specs directory path (default: "docs/specs").
    #[serde(default = "SpecsConfig::default_path")]
    pub path: String,

    /// File extension for spec files (default: ".md").
    #[serde(default = "SpecsConfig::default_extension")]
    pub extension: String,

    /// Index mode: "auto" | "toc" | "linked" | "exists" (default: "exists" for this phase).
    #[serde(default = "SpecsConfig::default_index")]
    pub index: String,

    /// Override index file path (auto-detect if not specified).
    pub index_file: Option<String>,

    /// Section validation configuration.
    #[serde(default)]
    pub sections: SpecsSectionsConfig,

    /// Markdown table enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub tables: ContentRule,

    /// Box diagram enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub box_diagrams: ContentRule,

    /// Mermaid block enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub mermaid: ContentRule,

    /// Maximum lines per spec file (default: 1000, None to disable).
    #[serde(
        default = "SpecsConfig::default_max_lines",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_lines: Option<usize>,

    /// Maximum tokens per spec file (default: 20000, None to disable).
    #[serde(
        default = "SpecsConfig::default_max_tokens",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_tokens: Option<usize>,
}

impl Default for SpecsConfig {
    fn default() -> Self {
        Self {
            check: None,
            path: Self::default_path(),
            extension: Self::default_extension(),
            index: Self::default_index(),
            index_file: None,
            sections: SpecsSectionsConfig::default(),
            tables: ContentRule::allow(),
            box_diagrams: ContentRule::allow(),
            mermaid: ContentRule::allow(),
            max_lines: Self::default_max_lines(),
            max_tokens: Self::default_max_tokens(),
        }
    }
}

impl SpecsConfig {
    pub(super) fn default_path() -> String {
        "docs/specs".to_string()
    }

    pub(super) fn default_extension() -> String {
        ".md".to_string()
    }

    pub(super) fn default_index() -> String {
        "exists".to_string()
    }

    /// Default max lines per spec file (1000).
    pub(super) fn default_max_lines() -> Option<usize> {
        Some(super::defaults::size::MAX_LINES_SPEC)
    }

    /// Default max tokens per spec file (20000).
    pub(super) fn default_max_tokens() -> Option<usize> {
        Some(super::defaults::size::MAX_TOKENS)
    }
}

/// Section validation for specs (separate from agents to allow different defaults).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecsSectionsConfig {
    /// Required sections (simple form: names only, or extended form with advice).
    #[serde(default)]
    pub required: Vec<RequiredSection>,

    /// Forbidden sections (supports globs like "Draft*").
    #[serde(default)]
    pub forbid: Vec<String>,
}

/// Escapes check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EscapesConfig {
    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Patterns to detect (overrides defaults).
    #[serde(default)]
    pub patterns: Vec<EscapePattern>,
}

/// A single escape hatch pattern definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EscapePattern {
    /// Unique name for this pattern (e.g., "unwrap", "unsafe").
    /// If not provided, uses the pattern itself as the name.
    #[serde(default)]
    pub name: Option<String>,

    /// Regex pattern to match.
    pub pattern: String,

    /// Action to take: count, comment, or forbid.
    #[serde(default)]
    pub action: EscapeAction,

    /// Required comment pattern for action = "comment".
    #[serde(default)]
    pub comment: Option<String>,

    /// Count threshold for action = "count" (default: 0).
    #[serde(default)]
    pub threshold: usize,

    /// Custom advice message for violations.
    #[serde(default)]
    pub advice: Option<String>,

    /// Source file patterns (optional, applies to specific files).
    #[serde(default)]
    pub source: Vec<String>,

    /// Test file patterns (optional, applies to specific files).
    #[serde(default)]
    pub tests: Vec<String>,

    /// Override action for test code ("allow" | "comment" | "forbid").
    #[serde(default)]
    pub in_tests: Option<String>,
}

impl EscapePattern {
    /// Get the effective name for this pattern (uses name if present, otherwise pattern).
    pub fn effective_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.pattern)
    }
}

/// Action to take when pattern is matched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EscapeAction {
    #[default]
    Forbid,
    Comment,
    Count,
}

/// Which line metric to use for size thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineMetric {
    /// Total lines (matches `wc -l`).
    #[default]
    Lines,
    /// Non-blank lines only.
    Nonblank,
}

/// Cloc check configuration.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClocConfig {
    /// Maximum lines per file (default: 750).
    #[serde(default = "ClocConfig::default_max_lines")]
    pub max_lines: usize,

    /// Maximum lines per test file (default: 1000).
    #[serde(default = "ClocConfig::default_max_lines_test")]
    pub max_lines_test: usize,

    /// Which line metric to compare against max_lines (default: lines).
    /// - "lines": total lines (matches `wc -l`)
    /// - "nonblank": non-blank lines only
    #[serde(default)]
    pub metric: LineMetric,

    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ClocConfig::default_test_patterns")]
    pub test_patterns: Vec<String>,

    /// Patterns to exclude from size limit checks.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Maximum tokens per file (default: 20000, None = disabled).
    /// Accepts either a number or `false` to disable.
    #[serde(
        default = "ClocConfig::default_max_tokens",
        deserialize_with = "deserialize_max_tokens"
    )]
    pub max_tokens: Option<usize>,

    /// Advice message for source file violations.
    #[serde(default = "ClocConfig::default_advice")]
    pub advice: String,

    /// Advice message for test file violations.
    #[serde(default = "ClocConfig::default_advice_test")]
    pub advice_test: String,
}

impl Default for ClocConfig {
    fn default() -> Self {
        Self {
            max_lines: Self::default_max_lines(),
            max_lines_test: Self::default_max_lines_test(),
            metric: LineMetric::default(),
            check: CheckLevel::default(),
            test_patterns: Self::default_test_patterns(),
            exclude: Vec::new(),
            max_tokens: Self::default_max_tokens(),
            advice: Self::default_advice(),
            advice_test: Self::default_advice_test(),
        }
    }
}

impl ClocConfig {
    pub(super) fn default_max_lines() -> usize {
        super::defaults::size::MAX_LINES
    }

    pub(super) fn default_max_lines_test() -> usize {
        super::defaults::size::MAX_LINES_TEST
    }

    pub(super) fn default_max_tokens() -> Option<usize> {
        Some(super::defaults::size::MAX_TOKENS)
    }

    pub(super) fn default_test_patterns() -> Vec<String> {
        super::defaults::test_patterns::generic()
    }

    pub(super) fn default_advice() -> String {
        super::defaults::advice::cloc_source(super::defaults::size::MAX_LINES)
    }

    pub(crate) fn default_advice_test() -> String {
        super::defaults::advice::cloc_test(super::defaults::size::MAX_LINES_TEST)
    }
}

/// Check level: error, warn, or off.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    #[default]
    Error,
    Warn,
    Off,
}

/// Custom deserializer for max_tokens that accepts either a number or `false`.
fn deserialize_max_tokens<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MaxTokens {
        Number(usize),
        Bool(bool),
    }

    match MaxTokens::deserialize(deserializer)? {
        MaxTokens::Number(n) => Ok(Some(n)),
        MaxTokens::Bool(false) => Ok(None),
        MaxTokens::Bool(true) => Err(de::Error::custom(
            "max_tokens must be a number or false, not true",
        )),
    }
}

/// Per-language cloc configuration.
///
/// Allows overriding the global cloc.check level and advice per language.
/// Unset fields inherit from [check.cloc].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LangClocConfig {
    /// Check level: error, warn, or off.
    /// If None, inherits from check.cloc.check.
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Custom advice for violations.
    /// If None, uses language-specific default or check.cloc.advice.
    #[serde(default)]
    pub advice: Option<String>,
}
