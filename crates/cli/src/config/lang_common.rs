// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Common language configuration utilities.
//!
//! Provides macros and traits to reduce duplication across language-specific
//! configuration modules (rust, go, javascript, ruby, shell).

/// Macro to define a language-specific policy config struct.
///
/// Generates a struct with `check`, `lint_changes`, and `lint_config` fields,
/// along with Default impl and PolicyConfig trait impl.
///
/// # Example
/// ```ignore
/// define_policy_config!(GoPolicyConfig, [
///     ".golangci.yml",
///     ".golangci.yaml",
///     ".golangci.toml",
/// ]);
/// ```
macro_rules! define_policy_config {
    ($name:ident, [$($config_file:expr),* $(,)?]) => {
        /// Lint policy configuration.
        #[derive(Debug, Clone, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        pub struct $name {
            /// Check level: "error" | "warn" | "off" (default: inherits from global).
            pub check: Option<CheckLevel>,

            /// Lint config changes policy: "standalone" requires separate PRs.
            pub lint_changes: LintChangesPolicy,

            /// Files that trigger the standalone requirement.
            pub lint_config: Vec<String>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    check: None,
                    lint_changes: LintChangesPolicy::default(),
                    lint_config: Self::default_lint_config(),
                }
            }
        }

        impl $name {
            pub(crate) fn default_lint_config() -> Vec<String> {
                vec![$($config_file.to_string()),*]
            }
        }

        impl crate::adapter::common::policy::PolicyConfig for $name {
            fn lint_changes(&self) -> LintChangesPolicy {
                self.lint_changes
            }

            fn lint_config(&self) -> &[String] {
                &self.lint_config
            }
        }
    };
}

pub(crate) use define_policy_config;

/// Trait for language-specific configuration defaults.
///
/// Languages implement this to provide their default patterns for source,
/// test, and exclude file matching.
pub trait LanguageDefaults {
    /// Default source file patterns.
    fn default_source() -> Vec<String>;

    /// Default test file patterns.
    fn default_tests() -> Vec<String>;

    /// Default exclude patterns (walker-level: prevents I/O on subtrees).
    fn default_exclude() -> Vec<String> {
        vec![]
    }

    /// Default advice for cloc violations.
    fn default_cloc_advice(threshold: usize) -> String {
        let range = super::defaults::advice::target_range(threshold);
        format!(
            "First, look for repetitive patterns that could be extracted \
into helper functions.\n\
\n\
Then split by semantic concern (target {range} each).\n\
\n\
Avoid removing individual lines to satisfy the linter; \
prefer extracting testable code blocks."
        )
    }
}
