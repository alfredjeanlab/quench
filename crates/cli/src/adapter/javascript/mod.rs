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

use std::fs;
use std::path::Path;

use globset::GlobSet;
use serde_json::Value;

mod policy;
mod suppress;
mod workspace;

pub use policy::{PolicyCheckResult, check_lint_policy};
pub use suppress::{JavaScriptSuppress, SuppressTool, parse_javascript_suppresses};
pub use workspace::JsWorkspace;

use crate::config::JavaScriptPolicyConfig;

use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};

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
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl JavaScriptAdapter {
    /// Create a new JavaScript adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&[
                "**/*.js".to_string(),
                "**/*.jsx".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
                "**/*.mjs".to_string(),
                "**/*.mts".to_string(),
            ]),
            test_patterns: build_glob_set(&[
                "**/*.test.js".to_string(),
                "**/*.test.ts".to_string(),
                "**/*.test.jsx".to_string(),
                "**/*.test.tsx".to_string(),
                "**/*.spec.js".to_string(),
                "**/*.spec.ts".to_string(),
                "**/*.spec.jsx".to_string(),
                "**/*.spec.tsx".to_string(),
                "**/__tests__/**".to_string(),
                "test/**".to_string(),
                "tests/**".to_string(),
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

    /// Check if a path should be ignored (e.g., node_modules/).
    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }

    /// Read package name from package.json at the given root.
    pub fn package_name(root: &Path) -> Option<String> {
        let pkg_json = root.join("package.json");
        let content = fs::read_to_string(&pkg_json).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;
        value.get("name")?.as_str().map(String::from)
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
        &["js", "jsx", "ts", "tsx", "mjs", "mts"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
        if self.should_ignore(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Source patterns
        if self.source_patterns.is_match(path) {
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
