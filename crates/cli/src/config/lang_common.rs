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
/// test, and ignore file matching.
pub trait LanguageDefaults {
    /// Default source file patterns.
    fn default_source() -> Vec<String>;

    /// Default test file patterns.
    fn default_tests() -> Vec<String>;

    /// Default ignore patterns.
    fn default_ignore() -> Vec<String> {
        vec![]
    }

    /// Default advice for cloc violations.
    fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\
         Look for repetitive patterns that could be extracted into helper functions."
    }
}
