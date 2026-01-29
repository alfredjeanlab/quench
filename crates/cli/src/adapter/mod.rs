// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Language adapters provide language-specific behavior for checks.
//!
//! See docs/specs/10-language-adapters.md for specification.
//!
//! # Language Adapter Development Guide
//!
//! ## Standard Adapter Pattern
//!
//! All language adapters should follow this structure:
//!
//! ```text
//! adapter/{lang}/
//! ├── mod.rs           # {Lang}Adapter struct + Adapter trait impl
//! ├── mod_tests.rs     # Classification and pattern tests
//! ├── suppress.rs      # Lint directive parsing (language-specific)
//! ├── suppress_tests.rs
//! └── policy.rs        # (optional) Re-export or customize common policy
//! ```
//!
//! ## Required Fields
//!
//! ```ignore
//! pub struct LanguageAdapter {
//!     source_patterns: GlobSet,   // Required (or use fast extension check)
//!     test_patterns: GlobSet,     // Required
//!     exclude_patterns: GlobSet,  // Required
//! }
//! ```
//!
//! ## Required Methods
//!
//! - `new()` - Create adapter with language defaults
//! - `with_patterns(ResolvedPatterns)` - Create adapter from config-resolved patterns
//! - `should_exclude(&Path) -> bool` - MUST use `common::patterns::check_exclude_patterns()`
//!
//! ## Adapter Trait
//!
//! - `name() -> &'static str`
//! - `extensions() -> &'static [&'static str]`
//! - `classify(&Path) -> FileKind`
//! - `default_escapes() -> &'static [EscapePattern]`
//!
//! ## Optimization: Fast Prefixes
//!
//! For languages with common exclude directories, use the `fast_prefixes`
//! parameter of `check_exclude_patterns()` for better performance:
//!
//! ```ignore
//! Some(&["node_modules", "dist", "build"])  // JavaScript
//! Some(&["vendor"])                          // Go
//! None                                       // Languages without common excludes
//! ```
//!
//! ## Checklist for New Adapters
//!
//! - Add language config in `config/<lang>.rs` with `source`, `tests`, `exclude` fields
//! - Implement `LanguageDefaults` trait with default patterns
//! - Create adapter struct with all three pattern fields
//! - Implement `new()`, `with_patterns()`, `should_exclude()`
//! - Use `check_exclude_patterns()` in `should_exclude()`
//! - Add pattern resolution in this file
//! - Write unit tests for config parsing and adapter behavior
//! - Add integration test fixture with custom exclude patterns

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub mod common;
pub mod generic;
pub mod glob;
pub mod go;
pub mod javascript;
pub mod patterns;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod shell;

pub use generic::GenericAdapter;
pub use go::{enumerate_packages, parse_nolint_directives};
pub use javascript::JsWorkspace;
pub use rust::parse_suppress_attrs;

pub(crate) use glob::build_glob_set;
pub(crate) use shell::{ShellAdapter, parse_shellcheck_suppresses};

pub(crate) use go::GoAdapter;
pub(crate) use javascript::{Bundler, JavaScriptAdapter, detect_bundler};
pub(crate) use python::PythonAdapter;
pub(crate) use ruby::{RubyAdapter, parse_ruby_suppresses};
pub(crate) use rust::{CfgTestInfo, RustAdapter};

/// File classification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Production source code.
    Source,
    /// Test code (unit tests, integration tests).
    Test,
    /// Not a source file (config, data, etc.).
    Other,
}

/// A language adapter provides language-specific behavior for checks.
///
/// Adapters are responsible for:
/// - Classifying files as source, test, or other
/// - Providing default escape patterns
/// - TODO(Future): Inline test detection, lint suppression patterns
pub trait Adapter: Send + Sync {
    /// Adapter identifier (e.g., "rust", "shell", "generic").
    fn name(&self) -> &'static str;

    /// File extensions this adapter handles (e.g., ["rs"] for Rust).
    /// Empty slice means this adapter doesn't match by extension (generic fallback).
    fn extensions(&self) -> &'static [&'static str];

    /// Classify a file by its path relative to the project root.
    fn classify(&self, path: &Path) -> FileKind;

    /// Default escape patterns for this language.
    /// Returns empty slice for languages with no default escapes (generic).
    fn default_escapes(&self) -> &'static [EscapePattern] {
        &[]
    }
}

/// An escape pattern with its action.
#[derive(Debug, Clone)]
pub struct EscapePattern {
    /// Pattern name for reporting (e.g., "unsafe", "unwrap").
    pub name: &'static str,
    /// Regex pattern to match.
    pub pattern: &'static str,
    /// Required action for this escape.
    pub action: EscapeAction,
    /// Required comment pattern (for Comment action).
    pub comment: Option<&'static str>,
    /// Advice to show when pattern is violated.
    pub advice: &'static str,
    /// Override action for test code ("allow" | "forbid" | "comment").
    /// None means default (allow in tests).
    pub in_tests: Option<&'static str>,
}

/// Action required for an escape pattern match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeAction {
    /// Just count occurrences.
    Count,
    /// Require a justification comment.
    Comment,
    /// Never allowed.
    Forbid,
}

/// Registry of available adapters.
pub struct AdapterRegistry {
    /// Adapters by extension (e.g., "rs" -> RustAdapter).
    by_extension: HashMap<&'static str, Arc<dyn Adapter>>,
    /// Fallback adapter for unrecognized extensions.
    fallback: Arc<dyn Adapter>,
}

impl AdapterRegistry {
    /// Create a new registry with the given fallback adapter.
    pub fn new(fallback: Arc<dyn Adapter>) -> Self {
        Self {
            by_extension: HashMap::new(),
            fallback,
        }
    }

    /// Register an adapter for its declared extensions.
    pub fn register(&mut self, adapter: Arc<dyn Adapter>) {
        for ext in adapter.extensions() {
            self.by_extension.insert(ext, Arc::clone(&adapter));
        }
    }

    /// Get the adapter for a file path based on extension.
    pub fn adapter_for(&self, path: &Path) -> &dyn Adapter {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        self.by_extension
            .get(ext)
            .map(|a| a.as_ref())
            .unwrap_or(self.fallback.as_ref())
    }

    /// Classify a file using the appropriate adapter.
    pub fn classify(&self, path: &Path) -> FileKind {
        self.adapter_for(path).classify(path)
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new(Arc::new(GenericAdapter::with_defaults()))
    }
}

// =============================================================================
// PROJECT LANGUAGE DETECTION
// =============================================================================

/// Detect project language from marker files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectLanguage {
    Rust,
    Go,
    JavaScript,
    Python,
    Ruby,
    Shell,
    Generic,
}

/// Detect project language by checking for marker files.
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }

    if root.join("go.mod").exists() {
        return ProjectLanguage::Go;
    }

    // JavaScript detection (before Shell check)
    if root.join("package.json").exists()
        || root.join("tsconfig.json").exists()
        || root.join("jsconfig.json").exists()
    {
        return ProjectLanguage::JavaScript;
    }

    // Python detection (before Ruby and Shell check)
    if has_python_markers(root) {
        return ProjectLanguage::Python;
    }

    // Ruby detection (before Shell check)
    if has_ruby_markers(root) {
        return ProjectLanguage::Ruby;
    }

    // Check for Shell project markers: *.sh in root, bin/, or scripts/
    if has_shell_markers(root) {
        return ProjectLanguage::Shell;
    }

    ProjectLanguage::Generic
}

/// Check if project has Python markers.
/// Detection: pyproject.toml, setup.py, setup.cfg, or requirements.txt
fn has_python_markers(root: &Path) -> bool {
    root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("setup.cfg").exists()
        || root.join("requirements.txt").exists()
}

/// Check if project has Ruby markers.
/// Detection: Gemfile, *.gemspec, config.ru, or config/application.rb (Rails)
fn has_ruby_markers(root: &Path) -> bool {
    root.join("Gemfile").exists()
        || has_gemspec(root)
        || root.join("config.ru").exists()
        || root.join("config/application.rb").exists()
}

/// Check if a directory contains *.gemspec files.
fn has_gemspec(root: &Path) -> bool {
    root.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("gemspec")
            })
        })
        .unwrap_or(false)
}

/// Check if project has Shell markers.
/// Detection: *.sh files in root, bin/, or scripts/
fn has_shell_markers(root: &Path) -> bool {
    // Check root directory
    if has_sh_files(root) {
        return true;
    }

    // Check bin/ directory
    let bin_dir = root.join("bin");
    if bin_dir.is_dir() && has_sh_files(&bin_dir) {
        return true;
    }

    // Check scripts/ directory
    let scripts_dir = root.join("scripts");
    if scripts_dir.is_dir() && has_sh_files(&scripts_dir) {
        return true;
    }

    false
}

/// Check if a directory contains *.sh files.
fn has_sh_files(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sh")
            })
        })
        .unwrap_or(false)
}

impl AdapterRegistry {
    /// Create a registry pre-populated with detected adapters.
    ///
    /// Uses default patterns for all adapters. For config-aware pattern resolution,
    /// use `for_project_with_config` instead.
    pub fn for_project(root: &Path) -> Self {
        let mut registry = Self::new(Arc::new(GenericAdapter::with_defaults()));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                registry.register(Arc::new(RustAdapter::new()));
            }
            ProjectLanguage::Go => {
                registry.register(Arc::new(GoAdapter::new()));
            }
            ProjectLanguage::JavaScript => {
                registry.register(Arc::new(JavaScriptAdapter::new()));
            }
            ProjectLanguage::Python => {
                registry.register(Arc::new(PythonAdapter::new()));
            }
            ProjectLanguage::Ruby => {
                registry.register(Arc::new(RubyAdapter::new()));
            }
            ProjectLanguage::Shell => {
                registry.register(Arc::new(ShellAdapter::new()));
            }
            ProjectLanguage::Generic => {}
        }

        registry
    }

    /// Create a registry with config-aware pattern resolution.
    ///
    /// Pattern resolution hierarchy:
    /// 1. `[<language>].tests` - Language-specific override (most specific)
    /// 2. `[project].tests` - Project-wide patterns
    /// 3. Adapter defaults - Built-in convention (zero-config)
    pub fn for_project_with_config(root: &Path, config: &crate::config::Config) -> Self {
        // Resolve fallback patterns: project config or generic defaults
        let fallback_test_patterns = if !config.project.tests.is_empty() {
            config.project.tests.clone()
        } else {
            GenericAdapter::default_test_patterns()
        };

        let fallback_source_patterns = if !config.project.source.is_empty() {
            config.project.source.clone()
        } else {
            vec![] // Empty = all non-test files are source
        };

        let mut registry = Self::new(Arc::new(GenericAdapter::new(
            &fallback_source_patterns,
            &fallback_test_patterns,
        )));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                let patterns = resolve_rust_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(RustAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::Go => {
                let patterns = resolve_go_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(GoAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::JavaScript => {
                let patterns = resolve_javascript_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(JavaScriptAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::Python => {
                let patterns = resolve_python_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(PythonAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::Ruby => {
                let patterns = resolve_ruby_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(RubyAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::Shell => {
                let patterns = resolve_shell_patterns(config, &fallback_test_patterns);
                registry.register(Arc::new(ShellAdapter::with_patterns(patterns)));
            }
            ProjectLanguage::Generic => {}
        }

        registry
    }
}

// Re-export ResolvedPatterns from the patterns module.
pub use patterns::ResolvedPatterns;

/// Macro to define a resolve_*_patterns function.
///
/// Generates a function that resolves patterns from config with the standard
/// fallback hierarchy: language config -> project config -> language defaults.
macro_rules! define_resolve_patterns {
    ($fn_name:ident, $config_field:ident, $config_type:ty) => {
        fn $fn_name(config: &crate::config::Config, fallback_test: &[String]) -> ResolvedPatterns {
            patterns::resolve_patterns::<$config_type>(
                &config.$config_field.source,
                &config.$config_field.tests,
                &config.$config_field.exclude,
                fallback_test,
            )
        }
    };
}

define_resolve_patterns!(resolve_rust_patterns, rust, crate::config::RustConfig);
define_resolve_patterns!(resolve_go_patterns, golang, crate::config::GoConfig);
define_resolve_patterns!(
    resolve_javascript_patterns,
    javascript,
    crate::config::JavaScriptConfig
);
define_resolve_patterns!(resolve_python_patterns, python, crate::config::PythonConfig);
define_resolve_patterns!(resolve_ruby_patterns, ruby, crate::config::RubyConfig);
define_resolve_patterns!(resolve_shell_patterns, shell, crate::config::ShellConfig);

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
