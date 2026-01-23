//! Language adapters provide language-specific behavior for checks.
//!
//! See docs/specs/10-language-adapters.md for specification.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub mod generic;
pub mod rust;

pub use generic::GenericAdapter;
pub use rust::{CfgTestInfo, PolicyCheckResult, RustAdapter, parse_suppress_attrs};

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
/// - (Future) Inline test detection, lint suppression patterns
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
    Generic,
}

/// Detect project language by checking for marker files.
pub fn detect_language(root: &Path) -> ProjectLanguage {
    if root.join("Cargo.toml").exists() {
        return ProjectLanguage::Rust;
    }
    ProjectLanguage::Generic
}

impl AdapterRegistry {
    /// Create a registry pre-populated with detected adapters.
    pub fn for_project(root: &Path) -> Self {
        let mut registry = Self::new(Arc::new(GenericAdapter::with_defaults()));

        match detect_language(root) {
            ProjectLanguage::Rust => {
                registry.register(Arc::new(RustAdapter::new()));
            }
            ProjectLanguage::Generic => {}
        }

        registry
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
