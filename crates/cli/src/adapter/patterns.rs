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

    /// Default ignore patterns for this language.
    fn default_ignore() -> Vec<String> {
        vec![]
    }
}

/// Resolved patterns for an adapter.
#[derive(Debug, Clone)]
pub struct ResolvedPatterns {
    pub source: Vec<String>,
    pub test: Vec<String>,
    pub ignore: Vec<String>,
}

// =============================================================================
// TRAIT IMPLEMENTATIONS FOR LANGUAGE CONFIGS
// =============================================================================

impl LanguageDefaults for crate::config::RustConfig {
    fn default_source() -> Vec<String> {
        crate::config::RustConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::RustConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::RustConfig::default_ignore()
    }
}

impl LanguageDefaults for crate::config::GoConfig {
    fn default_source() -> Vec<String> {
        crate::config::GoConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::GoConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::GoConfig::default_ignore()
    }
}

impl LanguageDefaults for crate::config::JavaScriptConfig {
    fn default_source() -> Vec<String> {
        crate::config::JavaScriptConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::JavaScriptConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::JavaScriptConfig::default_ignore()
    }
}

impl LanguageDefaults for crate::config::RubyConfig {
    fn default_source() -> Vec<String> {
        crate::config::RubyConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::RubyConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::RubyConfig::default_ignore()
    }
}

impl LanguageDefaults for crate::config::PythonConfig {
    fn default_source() -> Vec<String> {
        crate::config::PythonConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::PythonConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::PythonConfig::default_ignore()
    }
}

impl LanguageDefaults for crate::config::ShellConfig {
    fn default_source() -> Vec<String> {
        crate::config::ShellConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::ShellConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::ShellConfig::default_ignore()
    }
}

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
    lang_ignore: &[String],
    fallback_test: &[String],
) -> ResolvedPatterns {
    let test = if !lang_tests.is_empty() {
        lang_tests.to_vec()
    } else if !fallback_test.is_empty() {
        fallback_test.to_vec()
    } else {
        C::default_tests()
    };

    let source = if !lang_source.is_empty() {
        lang_source.to_vec()
    } else {
        C::default_source()
    };

    let ignore = if !lang_ignore.is_empty() {
        lang_ignore.to_vec()
    } else {
        C::default_ignore()
    };

    ResolvedPatterns {
        source,
        test,
        ignore,
    }
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
