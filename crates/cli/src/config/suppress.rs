// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared suppress configuration types.
//!
//! Used by Rust, Go, and Shell language adapters.

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Lint suppression configuration for #[allow(...)] and #[expect(...)].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "SuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    /// Example: "// JUSTIFIED:" or "// REASON:"
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default, deserialize_with = "deserialize_source_scope")]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(
        default = "SuppressScopeConfig::default_for_test",
        deserialize_with = "deserialize_test_scope"
    )]
    pub test: SuppressScopeConfig,
}

impl Default for SuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: SuppressScopeConfig::default(),
            test: SuppressScopeConfig::default_for_test(),
        }
    }
}

impl SuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Comment
    }
}

/// Custom deserializer for source scope that merges with defaults.
fn deserialize_source_scope<'de, D>(deserializer: D) -> Result<SuppressScopeConfig, D::Error>
where
    D: Deserializer<'de>,
{
    let user_config = SuppressScopeConfig::deserialize(deserializer)?;
    Ok(merge_with_defaults(
        user_config,
        SuppressScopeConfig::default_for_source(),
    ))
}

/// Custom deserializer for test scope that merges with defaults.
fn deserialize_test_scope<'de, D>(deserializer: D) -> Result<SuppressScopeConfig, D::Error>
where
    D: Deserializer<'de>,
{
    let user_config = SuppressScopeConfig::deserialize(deserializer)?;
    Ok(merge_with_defaults(
        user_config,
        SuppressScopeConfig::default_for_test(),
    ))
}

/// Merge user config with defaults: user patterns override, but defaults are preserved.
fn merge_with_defaults(
    user: SuppressScopeConfig,
    mut defaults: SuppressScopeConfig,
) -> SuppressScopeConfig {
    // User-provided patterns override defaults
    for (lint_code, patterns) in user.patterns {
        defaults.patterns.insert(lint_code, patterns);
    }

    SuppressScopeConfig {
        check: user.check.or(defaults.check),
        allow: if user.allow.is_empty() {
            defaults.allow
        } else {
            user.allow
        },
        forbid: if user.forbid.is_empty() {
            defaults.forbid
        } else {
            user.forbid
        },
        patterns: defaults.patterns,
    }
}

/// Scope-specific suppress configuration.
///
/// NOTE: Uses custom deserializer to accept arbitrary lint codes as fields
/// (e.g., `dead_code = "// REASON:"`), which are parsed into the `patterns` map.
#[derive(Debug, Clone)]
pub struct SuppressScopeConfig {
    /// Override check level for this scope.
    pub check: Option<SuppressLevel>,

    /// Lint codes that don't require comments (per-code allow list).
    pub allow: Vec<String>,

    /// Lint codes that are never allowed to be suppressed (per-code forbid list).
    pub forbid: Vec<String>,

    /// Per-lint-code comment patterns. Maps lint code to list of valid comment prefixes.
    /// Any of the patterns is accepted.
    /// Example: {"dead_code" => ["// KEEP UNTIL:", "// NOTE(compat):"]}
    pub patterns: std::collections::HashMap<String, Vec<String>>,
}

impl<'de> Deserialize<'de> for SuppressScopeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SuppressScopeVisitor;

        impl<'de> Visitor<'de> for SuppressScopeVisitor {
            type Value = SuppressScopeConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a suppress scope configuration")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut check = None;
                let mut allow = None;
                let mut forbid = None;
                let mut patterns: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                let mut explicit_patterns: Option<std::collections::HashMap<String, Vec<String>>> =
                    None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "check" => {
                            if check.is_some() {
                                return Err(de::Error::duplicate_field("check"));
                            }
                            check = Some(map.next_value()?);
                        }
                        "allow" => {
                            if allow.is_some() {
                                return Err(de::Error::duplicate_field("allow"));
                            }
                            allow = Some(map.next_value()?);
                        }
                        "forbid" => {
                            if forbid.is_some() {
                                return Err(de::Error::duplicate_field("forbid"));
                            }
                            forbid = Some(map.next_value()?);
                        }
                        "patterns" => {
                            if explicit_patterns.is_some() {
                                return Err(de::Error::duplicate_field("patterns"));
                            }
                            explicit_patterns = Some(map.next_value()?);
                        }
                        lint_code => {
                            // Parse lint code pattern: string, array, or subsection with comment field
                            #[derive(Deserialize)]
                            #[serde(untagged)]
                            enum PatternValue {
                                Single(String),
                                Multiple(Vec<String>),
                                Subsection { comment: String },
                            }

                            let value: PatternValue = map.next_value()?;
                            let pattern_list = match value {
                                PatternValue::Single(s) => vec![s],
                                PatternValue::Multiple(v) => v,
                                PatternValue::Subsection { comment } => vec![comment],
                            };

                            patterns.insert(lint_code.to_string(), pattern_list);
                        }
                    }
                }

                // Merge explicit patterns map with inline lint codes
                if let Some(explicit) = explicit_patterns {
                    patterns.extend(explicit);
                }

                Ok(SuppressScopeConfig {
                    check,
                    allow: allow.unwrap_or_default(),
                    forbid: forbid.unwrap_or_default(),
                    patterns,
                })
            }
        }

        deserializer.deserialize_map(SuppressScopeVisitor)
    }
}

impl Default for SuppressScopeConfig {
    fn default() -> Self {
        Self::default_for_source()
    }
}

impl SuppressScopeConfig {
    /// Default for source code: requires specific comments for common lint suppressions.
    pub(crate) fn default_for_source() -> Self {
        use std::collections::HashMap;
        let patterns: HashMap<String, Vec<String>> = [
            // dead_code requires KEEP UNTIL, NOTE(compat), or NOTE(lifetime) comment
            (
                "dead_code",
                vec![
                    "// KEEP UNTIL:",
                    "// NOTE(compat):",
                    "// NOTE(compatibility):",
                    "// NOTE(lifetime):",
                ],
            ),
            // too_many_arguments requires TODO(refactor) comment
            ("clippy::too_many_arguments", vec!["// TODO(refactor):"]),
            // casts require CORRECTNESS or SAFETY comment
            (
                "clippy::cast_possible_truncation",
                vec!["// CORRECTNESS:", "// SAFETY:"],
            ),
            // deprecated requires TODO(refactor) or NOTE(compat) comment
            (
                "deprecated",
                vec![
                    "// TODO(refactor):",
                    "// NOTE(compat):",
                    "// NOTE(compatibility):",
                ],
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
        .collect();

        Self {
            check: None,
            allow: Vec::new(),
            forbid: Vec::new(),
            patterns,
        }
    }

    /// Default for test code: allow suppressions freely.
    pub(crate) fn default_for_test() -> Self {
        Self {
            check: Some(SuppressLevel::Allow),
            allow: Vec::new(),
            forbid: Vec::new(),
            patterns: std::collections::HashMap::new(),
        }
    }
}

/// Suppress check level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuppressLevel {
    /// Never allowed - any suppression fails.
    Forbid,
    /// Requires justification comment (default).
    #[default]
    Comment,
    /// Always allowed - no check.
    Allow,
}
