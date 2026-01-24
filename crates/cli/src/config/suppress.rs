//! Shared suppress configuration types.
//!
//! Used by Rust, Go, and Shell language adapters.

use serde::Deserialize;

/// Lint suppression configuration for #[allow(...)] and #[expect(...)].
#[derive(Debug, Clone, Deserialize)]
pub struct SuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "SuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    /// Example: "// JUSTIFIED:" or "// REASON:"
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default)]
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

/// Scope-specific suppress configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SuppressScopeConfig {
    /// Override check level for this scope.
    #[serde(default)]
    pub check: Option<SuppressLevel>,

    /// Lint codes that don't require comments (per-code allow list).
    #[serde(default)]
    pub allow: Vec<String>,

    /// Lint codes that are never allowed to be suppressed (per-code forbid list).
    #[serde(default)]
    pub forbid: Vec<String>,

    /// Per-lint-code comment patterns. Maps lint code to list of valid comment prefixes.
    /// Any of the patterns is accepted.
    /// Example: {"dead_code" => ["// KEEP UNTIL:", "// NOTE(compat):"]}
    #[serde(default)]
    pub patterns: std::collections::HashMap<String, Vec<String>>,
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
            // dead_code requires KEEP UNTIL or NOTE(compat) comment
            (
                "dead_code",
                vec![
                    "// KEEP UNTIL:",
                    "// NOTE(compat):",
                    "// NOTE(compatibility):",
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
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
