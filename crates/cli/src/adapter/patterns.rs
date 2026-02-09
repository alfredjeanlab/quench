// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Generic pattern resolution for language adapters.
//!
//! Provides a trait-based approach to resolving source, test, and ignore patterns
//! with a consistent fallback hierarchy.

/// Trait for language configurations that provide default patterns.
///
/// Implement this trait to participate in generic pattern resolution.
pub trait LanguageDefaults {
    /// Default source file patterns for this language.
    fn default_source() -> Vec<String>;

    /// Default test file patterns for this language.
    fn default_tests() -> Vec<String>;

    /// Default exclude patterns for this language (walker-level: prevents I/O on subtrees).
    fn default_exclude() -> Vec<String> {
        vec![]
    }
}

/// Resolved patterns for an adapter.
#[derive(Debug, Clone)]
pub struct ResolvedPatterns {
    pub source: Vec<String>,
    pub test: Vec<String>,
    pub exclude: Vec<String>,
}

// =============================================================================
// TRAIT IMPLEMENTATIONS FOR LANGUAGE CONFIGS
// =============================================================================

/// Implements `LanguageDefaults` for config types that have the same-named inherent methods.
macro_rules! impl_language_defaults {
    ($($config:ty),* $(,)?) => {
        $(
            impl LanguageDefaults for $config {
                fn default_source() -> Vec<String> {
                    <$config>::default_source()
                }

                fn default_tests() -> Vec<String> {
                    <$config>::default_tests()
                }

                fn default_exclude() -> Vec<String> {
                    <$config>::default_exclude()
                }
            }
        )*
    };
}

impl_language_defaults!(
    crate::config::RustConfig,
    crate::config::GoConfig,
    crate::config::JavaScriptConfig,
    crate::config::PythonConfig,
    crate::config::RubyConfig,
    crate::config::ShellConfig,
);

// =============================================================================
// PATTERN RESOLUTION
// =============================================================================

/// Generic pattern resolution.
///
/// Resolution hierarchy:
/// 1. Language-specific config (most specific)
/// 2. Project-wide fallback
/// 3. Language defaults (zero-config)
pub fn resolve_patterns<C: LanguageDefaults>(
    lang_source: &[String],
    lang_tests: &[String],
    lang_exclude: &[String],
    fallback_test: &[String],
) -> ResolvedPatterns {
    let test = if !lang_tests.is_empty() {
        lang_tests.to_vec()
    } else if !fallback_test.is_empty() {
        fallback_test.to_vec()
    } else {
        C::default_tests()
    };

    let source = if !lang_source.is_empty() { lang_source.to_vec() } else { C::default_source() };

    let exclude =
        if !lang_exclude.is_empty() { lang_exclude.to_vec() } else { C::default_exclude() };

    ResolvedPatterns { source, test, exclude }
}

// =============================================================================
// CORRELATION EXCLUDE DEFAULTS
// =============================================================================

/// Language-aware default exclude patterns for correlation checks.
///
/// These exclude files that typically don't need dedicated tests
/// (entry points, module declarations, generated code).
pub fn correlation_exclude_defaults(lang: super::ProjectLanguage) -> Vec<String> {
    // Universal: generated code is never test-required
    let mut patterns = vec!["**/generated/**".to_string()];

    // Language-specific entry points and declarations
    match lang {
        super::ProjectLanguage::Rust => {
            patterns.extend(["**/mod.rs", "**/lib.rs", "**/main.rs"].map(String::from));
        }
        super::ProjectLanguage::Go => {
            patterns.push("**/main.go".to_string());
        }
        super::ProjectLanguage::Python => {
            patterns.push("**/__init__.py".to_string());
        }
        super::ProjectLanguage::JavaScript => {
            patterns.extend(
                ["**/index.js", "**/index.ts", "**/index.jsx", "**/index.tsx"].map(String::from),
            );
        }
        super::ProjectLanguage::Ruby => {}
        super::ProjectLanguage::Shell => {}
        super::ProjectLanguage::Generic => {}
    }
    patterns
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
