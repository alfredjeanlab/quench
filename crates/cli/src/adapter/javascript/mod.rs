// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript/TypeScript language adapter.
//!
//! Provides JS/TS-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for JS/TS projects
//! - JS/TS-specific escape patterns (Phase 495)
//!
//! See docs/specs/langs/javascript.md for specification.

use std::path::Path;

use globset::GlobSet;

mod policy;
mod suppress;
mod workspace;

pub use policy::{PolicyCheckResult, check_lint_policy};
pub use suppress::{JavaScriptSuppress, SuppressTool, parse_javascript_suppresses};
pub use workspace::JsWorkspace;

use crate::config::JavaScriptPolicyConfig;

use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Common ignore directory prefixes to check before GlobSet.
/// Order: most common first for early exit.
const IGNORE_PREFIXES: &[&str] = &["node_modules", "dist", "build", ".next", "coverage"];

/// Default escape patterns for JavaScript/TypeScript.
///
/// These patterns detect common type safety escapes that require justification.
const JS_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "as_unknown",
        pattern: r"as\s+unknown",
        action: EscapeAction::Comment,
        comment: Some("// CAST:"),
        advice: "Add a // CAST: comment explaining why the type assertion is necessary.",
    },
    EscapePattern {
        name: "ts_ignore",
        pattern: r"@ts-ignore",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "@ts-ignore is forbidden. Use @ts-expect-error instead, which fails if the error is resolved.",
    },
];

/// JavaScript/TypeScript language adapter.
pub struct JavaScriptAdapter {
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl JavaScriptAdapter {
    /// Fast extension check for source files.
    /// Returns Some(true) for JS/TS extensions, None if GlobSet needed.
    #[inline]
    fn is_js_extension(path: &Path) -> Option<bool> {
        path.extension().and_then(|ext| ext.to_str()).map(|ext| {
            matches!(
                ext,
                "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts"
            )
        })
    }

    /// Create a new JavaScript adapter with default patterns.
    pub fn new() -> Self {
        Self {
            test_patterns: build_glob_set(&[
                "**/*.test.*".to_string(),
                "**/*.spec.*".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/test_*.*".to_string(),
                "**/__tests__/**".to_string(),
                "**/test/**".to_string(),
                "**/tests/**".to_string(),
            ]),
            ignore_patterns: build_glob_set(&[
                "node_modules/**".to_string(),
                "dist/**".to_string(),
                "build/**".to_string(),
                ".next/**".to_string(),
                "coverage/**".to_string(),
            ]),
        }
    }

    /// Create a JavaScript adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&patterns.ignore),
        }
    }

    /// Check if a path should be ignored (e.g., node_modules/).
    ///
    /// Uses fast prefix check for common directories before falling back to GlobSet.
    pub fn should_ignore(&self, path: &Path) -> bool {
        // Fast path: check common prefixes in first path component
        if let Some(first_component) = path.components().next()
            && let std::path::Component::Normal(name) = first_component
            && let Some(name_str) = name.to_str()
        {
            for prefix in IGNORE_PREFIXES {
                if name_str == *prefix {
                    return true;
                }
            }
        }

        // Fallback: GlobSet for edge cases (patterns in subdirectories)
        self.ignore_patterns.is_match(path)
    }

    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &JavaScriptPolicyConfig,
    ) -> PolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}

impl Default for JavaScriptAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for JavaScriptAdapter {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "ts", "tsx", "mjs", "mts", "cjs", "cts"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
        if self.should_ignore(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence (must check before source)
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Fast path: extension check instead of GlobSet for source files
        if Self::is_js_extension(path).unwrap_or(false) {
            return FileKind::Source;
        }

        FileKind::Other
    }

    fn default_escapes(&self) -> &'static [EscapePattern] {
        JS_ESCAPE_PATTERNS
    }
}

#[cfg(test)]
#[path = "../javascript_tests.rs"]
mod tests;
