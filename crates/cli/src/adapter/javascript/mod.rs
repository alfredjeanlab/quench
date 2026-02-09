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

mod bundler;
mod package_manager;
mod suppress;
mod workspace;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use bundler::{Bundler, detect_bundler};
pub use package_manager::PackageManager;
pub use suppress::{JavaScriptSuppress, SuppressTool, parse_javascript_suppresses};
pub use workspace::JsWorkspace;

use crate::config::JavaScriptPolicyConfig;

use super::common;
use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

/// Common exclude directory prefixes to check before GlobSet.
/// Order: most common first for early exit.
const EXCLUDE_PREFIXES: &[&str] = &["node_modules", "dist", "build", ".next", "coverage"];

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
        in_tests: None,
    },
    EscapePattern {
        name: "ts_ignore",
        pattern: r"@ts-ignore",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "@ts-ignore is forbidden. Use @ts-expect-error instead, which fails if the error is resolved.",
        in_tests: None,
    },
];

/// JavaScript/TypeScript language adapter.
pub struct JavaScriptAdapter {
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl JavaScriptAdapter {
    /// Fast extension check for source files.
    /// Returns Some(true) for JS/TS extensions, None if GlobSet needed.
    #[inline]
    fn is_js_extension(path: &Path) -> Option<bool> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts"))
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
                "**/*_tests/**".to_string(),
                "**/test/**".to_string(),
                "**/tests/**".to_string(),
            ]),
            exclude_patterns: build_glob_set(&[
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
            exclude_patterns: build_glob_set(&patterns.exclude),
        }
    }

    /// Check if a path should be excluded (e.g., node_modules/).
    ///
    /// Uses fast prefix check for common directories before falling back to GlobSet.
    pub fn should_exclude(&self, path: &Path) -> bool {
        common::patterns::check_exclude_patterns(
            path,
            &self.exclude_patterns,
            Some(EXCLUDE_PREFIXES),
        )
    }

    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &JavaScriptPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
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
        // Excluded paths are "Other"
        if self.should_exclude(path) {
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

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
