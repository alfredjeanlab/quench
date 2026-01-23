//! Generic language adapter (fallback).
//!
//! Uses patterns from [project] config for file classification.
//! Has no default escape patterns.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::{Adapter, FileKind};

/// Generic adapter that uses project config patterns.
///
/// This is the fallback adapter for files that don't match
/// any language-specific adapter.
pub struct GenericAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
}

impl GenericAdapter {
    /// Create a new generic adapter from config patterns.
    pub fn new(source_patterns: &[String], test_patterns: &[String]) -> Self {
        Self {
            source_patterns: build_glob_set(source_patterns),
            test_patterns: build_glob_set(test_patterns),
        }
    }

    /// Create with default patterns (no source filter, common test patterns).
    pub fn with_defaults() -> Self {
        Self::new(
            &[],
            &[
                "**/tests/**".to_string(),
                "**/test/**".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.test.*".to_string(),
                "**/*.spec.*".to_string(),
            ],
        )
    }
}

impl Adapter for GenericAdapter {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn extensions(&self) -> &'static [&'static str] {
        // Generic adapter doesn't match by extension
        // It's selected as fallback when no other adapter matches
        &[]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // If source patterns are configured, file must match
        if !self.source_patterns.is_empty() {
            if self.source_patterns.is_match(path) {
                return FileKind::Source;
            }
            return FileKind::Other;
        }

        // No source patterns configured = all non-test files are source
        FileKind::Source
    }

    // No default escapes for generic adapter
    // fn default_escapes() uses trait default: &[]
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        } else {
            tracing::warn!("invalid glob pattern: {}", pattern);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

#[cfg(test)]
#[path = "generic_tests.rs"]
mod tests;
