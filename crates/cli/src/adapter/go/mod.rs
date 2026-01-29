// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Go language adapter.
//!
//! Provides Go-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Go projects
//! - Go-specific escape patterns (unsafe.Pointer, go:linkname, go:noescape)
//!
//! See docs/specs/langs/golang.md for specification.

use std::path::Path;

use globset::GlobSet;

mod suppress;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use suppress::{NolintDirective, parse_nolint_directives};

use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::GoPolicyConfig;

/// Default escape patterns for Go.
///
/// These patterns require justification comments to explain why
/// potentially dangerous constructs are being used.
const GO_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe_pointer",
        pattern: r"unsafe\.Pointer",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining pointer validity.",
        in_tests: None,
    },
    EscapePattern {
        name: "go_linkname",
        pattern: r"//go:linkname",
        action: EscapeAction::Comment,
        comment: Some("// LINKNAME:"),
        advice: "Add a // LINKNAME: comment explaining the external symbol dependency.",
        in_tests: None,
    },
    EscapePattern {
        name: "go_noescape",
        pattern: r"//go:noescape",
        action: EscapeAction::Comment,
        comment: Some("// NOESCAPE:"),
        advice: "Add a // NOESCAPE: comment explaining why escape analysis should be bypassed.",
        in_tests: None,
    },
];

/// Go language adapter.
pub struct GoAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl GoAdapter {
    /// Create a new Go adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.go".to_string()]),
            test_patterns: build_glob_set(&["**/*_test.go".to_string()]),
            exclude_patterns: build_glob_set(&["vendor/**".to_string()]),
        }
    }

    /// Create a Go adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            exclude_patterns: build_glob_set(&patterns.exclude),
        }
    }

    /// Check if a path should be excluded (e.g., vendor/).
    pub fn should_exclude(&self, path: &Path) -> bool {
        self.exclude_patterns.is_match(path)
    }
}

impl Default for GoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for GoAdapter {
    fn name(&self) -> &'static str {
        "go"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Excluded paths are "Other"
        if self.should_exclude(path) {
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
        GO_ESCAPE_PATTERNS
    }
}

impl GoAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &GoPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
    }
}

/// Parse go.mod to extract module name.
pub fn parse_go_mod(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            return Some(trimmed.strip_prefix("module ")?.trim().to_string());
        }
    }
    None
}

/// Enumerate packages from directory structure.
/// Returns paths relative to the module root that contain .go files.
pub fn enumerate_packages(root: &Path) -> Vec<String> {
    let mut packages = Vec::new();
    enumerate_packages_recursive(root, root, &mut packages);
    packages
}

fn enumerate_packages_recursive(root: &Path, current: &Path, packages: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    let mut has_go_files = false;
    let mut subdirs = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip vendor directory
        if file_name_str == "vendor" {
            continue;
        }

        if path.is_dir() {
            subdirs.push(path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("go") {
            // Skip _test.go files when checking for package (they're still in the package)
            has_go_files = true;
        }
    }

    if has_go_files {
        let relative = current
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| {
                if s.is_empty() {
                    ".".to_string()
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| ".".to_string());
        packages.push(relative);
    }

    for subdir in subdirs {
        enumerate_packages_recursive(root, &subdir, packages);
    }
}

#[cfg(test)]
#[path = "../go_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
