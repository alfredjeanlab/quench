// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Ruby language adapter.
//!
//! Provides Ruby-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Ruby files
//! - Default escape patterns (debuggers, metaprogramming)
//! - RuboCop/Standard suppress directive parsing
//!
//! See docs/specs/langs/ruby.md for specification.

use std::path::Path;

use globset::GlobSet;

mod suppress;

pub use crate::adapter::common::policy::PolicyCheckResult;
pub use suppress::{RubySuppress, RubySuppressKind, parse_ruby_suppresses};

use super::common::patterns::normalize_ignore_patterns;
use super::glob::build_glob_set;
use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::RubyPolicyConfig;

/// Default escape patterns for Ruby.
const RUBY_ESCAPE_PATTERNS: &[EscapePattern] = &[
    // Debugger patterns - forbidden even in tests
    EscapePattern {
        name: "binding_pry",
        pattern: r"binding\.pry",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove debugger statement before committing.",
        in_tests: Some("forbid"),
    },
    EscapePattern {
        name: "byebug",
        pattern: r"\bbyebug\b",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove debugger statement before committing.",
        in_tests: Some("forbid"),
    },
    EscapePattern {
        name: "debugger",
        pattern: r"\bdebugger\b",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Remove debugger statement before committing.",
        in_tests: Some("forbid"),
    },
    // Metaprogramming patterns - allowed in tests by default
    EscapePattern {
        name: "eval",
        pattern: r"\beval\s*\(",
        action: EscapeAction::Comment,
        comment: Some("# METAPROGRAMMING:"),
        advice: "Add a # METAPROGRAMMING: comment explaining why eval is necessary.",
        in_tests: None,
    },
    EscapePattern {
        name: "instance_eval",
        pattern: r"\.instance_eval\b",
        action: EscapeAction::Comment,
        comment: Some("# METAPROGRAMMING:"),
        advice: "Add a # METAPROGRAMMING: comment explaining the DSL or metaprogramming use case.",
        in_tests: None,
    },
    EscapePattern {
        name: "class_eval",
        pattern: r"\.class_eval\b",
        action: EscapeAction::Comment,
        comment: Some("# METAPROGRAMMING:"),
        advice: "Add a # METAPROGRAMMING: comment explaining the metaprogramming use case.",
        in_tests: None,
    },
];

/// Ruby language adapter.
pub struct RubyAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RubyAdapter {
    /// Create a new Ruby adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&[
                "**/*.rb".to_string(),
                "**/*.rake".to_string(),
                "Rakefile".to_string(),
                "Gemfile".to_string(),
                "*.gemspec".to_string(),
            ]),
            test_patterns: build_glob_set(&[
                "spec/**/*_spec.rb".to_string(),
                "test/**/*_test.rb".to_string(),
                "test/**/test_*.rb".to_string(),
                "features/**/*.rb".to_string(),
            ]),
            ignore_patterns: build_glob_set(&[
                "vendor/**".to_string(),
                "tmp/**".to_string(),
                "log/**".to_string(),
                "coverage/**".to_string(),
            ]),
        }
    }

    /// Create a Ruby adapter with resolved patterns from config.
    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        let ignore_globs = normalize_ignore_patterns(&patterns.ignore);

        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&ignore_globs),
        }
    }

    /// Check if a path matches ignore patterns.
    pub fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check explicit ignore patterns
        if self.ignore_patterns.is_match(path) {
            return true;
        }

        // Also check for common ignored directories by path prefix
        // This handles cases where the path starts with these directories
        let parts: Vec<&str> = path_str.split('/').collect();
        if !parts.is_empty() {
            let first = parts[0];
            if first == "vendor" || first == "tmp" || first == "log" || first == "coverage" {
                return true;
            }
        }

        false
    }
}

impl Default for RubyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for RubyAdapter {
    fn name(&self) -> &'static str {
        "ruby"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rb", "rake"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Check ignore patterns first
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
        RUBY_ESCAPE_PATTERNS
    }
}

impl RubyAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &RubyPolicyConfig,
    ) -> PolicyCheckResult {
        crate::adapter::common::policy::check_lint_policy(changed_files, policy, |p| {
            self.classify(p)
        })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
