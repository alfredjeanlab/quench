// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Configuration for the agents check.

use serde::Deserialize;
use serde::de::{self, Deserializer};

use crate::config::CheckLevel;

/// Custom deserializer for optional usize that accepts false to mean None.
fn deserialize_optional_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionalUsize {
        Number(usize),
        Bool(bool),
    }
    match OptionalUsize::deserialize(deserializer)? {
        OptionalUsize::Number(n) => Ok(Some(n)),
        OptionalUsize::Bool(false) => Ok(None),
        OptionalUsize::Bool(true) => Err(de::Error::custom("expected a number or false, not true")),
    }
}

/// Content rule enforcement level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContentRule {
    /// Allow this content type.
    Allow,
    /// Forbid this content type (generate violation).
    #[default]
    Forbid,
}

impl ContentRule {
    /// Returns ContentRule::Allow (for serde defaults).
    pub fn allow() -> Self {
        ContentRule::Allow
    }
}

impl<'de> Deserialize<'de> for ContentRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "allow" => Ok(ContentRule::Allow),
            "forbid" => Ok(ContentRule::Forbid),
            _ => Err(serde::de::Error::custom(format!(
                "invalid content rule: {}, expected 'allow' or 'forbid'",
                s
            ))),
        }
    }
}

/// Configuration for the agents check.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsConfig {
    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Agent files to check (default: all recognized).
    #[serde(default = "AgentsConfig::default_files")]
    pub files: Vec<String>,

    /// Files that must exist (root scope). Use "*" to require any one file.
    #[serde(default = "AgentsConfig::default_required")]
    pub required: Vec<String>,

    /// Files checked if present (root scope).
    #[serde(default)]
    pub optional: Vec<String>,

    /// Files that must not exist (root scope).
    #[serde(default)]
    pub forbid: Vec<String>,

    /// Enable file synchronization checking (default: true).
    #[serde(default = "AgentsConfig::default_sync")]
    pub sync: bool,

    /// Source file for synchronization (other files should match this).
    #[serde(default)]
    pub sync_source: Option<String>,

    /// Section validation configuration.
    #[serde(default)]
    pub sections: SectionsConfig,

    /// Markdown table enforcement (default: forbid).
    #[serde(default)]
    pub tables: ContentRule,

    /// Box diagram enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub box_diagrams: ContentRule,

    /// Mermaid block enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub mermaid: ContentRule,

    /// Maximum lines per file (root scope, default: 500, None to disable).
    #[serde(
        default = "AgentsConfig::default_max_lines",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_lines: Option<usize>,

    /// Maximum tokens per file (root scope, default: 20000, None to disable).
    #[serde(
        default = "AgentsConfig::default_max_tokens",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_tokens: Option<usize>,

    /// Root scope settings (overrides flat config).
    #[serde(default)]
    pub root: Option<AgentsScopeConfig>,

    /// Package scope settings.
    #[serde(default)]
    pub package: Option<AgentsScopeConfig>,

    /// Module scope settings.
    #[serde(default)]
    pub module: Option<AgentsScopeConfig>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            check: CheckLevel::default(),
            files: Self::default_files(),
            required: Self::default_required(),
            optional: Vec::new(),
            forbid: Vec::new(),
            sync: Self::default_sync(),
            sync_source: None,
            sections: SectionsConfig::default(),
            tables: ContentRule::default(),
            box_diagrams: ContentRule::allow(),
            mermaid: ContentRule::allow(),
            max_lines: Self::default_max_lines(),
            max_tokens: Self::default_max_tokens(),
            root: None,
            package: None,
            module: None,
        }
    }
}

impl AgentsConfig {
    /// Default agent files to detect.
    pub fn default_files() -> Vec<String> {
        vec![
            "CLAUDE.md".to_string(),
            "AGENTS.md".to_string(),
            ".cursorrules".to_string(),
            ".cursorignore".to_string(),
            ".cursor/rules/*.md".to_string(),
            ".cursor/rules/*.mdc".to_string(),
        ]
    }

    /// Default required files ("*" = at least one agent file must exist).
    fn default_required() -> Vec<String> {
        vec!["*".to_string()]
    }

    /// Default sync setting (true - keep agent files in sync).
    fn default_sync() -> bool {
        true
    }

    /// Default max lines per file (500).
    fn default_max_lines() -> Option<usize> {
        Some(500)
    }

    /// Default max tokens per file (20000).
    fn default_max_tokens() -> Option<usize> {
        Some(20000)
    }
}

/// Per-scope configuration for agent files.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentsScopeConfig {
    /// Files that must exist at this scope.
    #[serde(default)]
    pub required: Vec<String>,

    /// Files checked if present at this scope.
    #[serde(default)]
    pub optional: Vec<String>,

    /// Files that must not exist at this scope.
    #[serde(default)]
    pub forbid: Vec<String>,

    /// Maximum lines per file at this scope.
    #[serde(default, deserialize_with = "deserialize_optional_usize")]
    pub max_lines: Option<usize>,

    /// Maximum tokens per file at this scope.
    #[serde(default, deserialize_with = "deserialize_optional_usize")]
    pub max_tokens: Option<usize>,
}

/// Section validation configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionsConfig {
    /// Required sections (simple form: names only, or extended form with advice).
    #[serde(default = "SectionsConfig::default_required")]
    pub required: Vec<RequiredSection>,

    /// Forbidden sections (supports globs like "Test*").
    #[serde(default)]
    pub forbid: Vec<String>,
}

impl Default for SectionsConfig {
    fn default() -> Self {
        Self {
            required: Self::default_required(),
            forbid: Vec::new(),
        }
    }
}

impl SectionsConfig {
    /// Default required sections for agent files.
    fn default_required() -> Vec<RequiredSection> {
        vec![
            RequiredSection {
                name: "Directory Structure".to_string(),
                advice: Some("Overview of project layout and key directories".to_string()),
            },
            RequiredSection {
                name: "Landing the Plane".to_string(),
                advice: Some("Checklist for AI agents before completing work".to_string()),
            },
        ]
    }
}

/// A required section with optional advice.
#[derive(Debug, Clone)]
pub struct RequiredSection {
    /// Section name (case-insensitive matching).
    pub name: String,
    /// Advice shown when section is missing.
    pub advice: Option<String>,
}

impl<'de> Deserialize<'de> for RequiredSection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged, deny_unknown_fields)]
        enum RequiredSectionRepr {
            Simple(String),
            Extended {
                name: String,
                advice: Option<String>,
            },
        }

        match RequiredSectionRepr::deserialize(deserializer)? {
            RequiredSectionRepr::Simple(name) => Ok(RequiredSection { name, advice: None }),
            RequiredSectionRepr::Extended { name, advice } => Ok(RequiredSection { name, advice }),
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
