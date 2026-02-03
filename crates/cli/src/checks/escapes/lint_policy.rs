// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Lint policy checking for the escapes check.

use std::path::Path;

use crate::adapter::common::policy::{self, PolicyConfig};
use crate::adapter::{
    GoAdapter, JavaScriptAdapter, ProjectLanguage, PythonAdapter, RubyAdapter, RustAdapter,
    ShellAdapter, detect_language,
};
use crate::check::{CheckContext, Violation};
use crate::config::{CheckLevel, LintChangesPolicy};

/// Result of lint policy check with violations and their check level.
pub struct PolicyCheckResult {
    /// Violations found (empty if check is off or no violations).
    pub violations: Vec<Violation>,
    /// The check level for these violations (determines if warnings or errors).
    pub check_level: CheckLevel,
}

/// Check lint policy and return violations with their check level.
pub fn check_lint_policy(ctx: &CheckContext) -> PolicyCheckResult {
    match detect_language(ctx.root) {
        ProjectLanguage::Rust => check_language_lint_policy(
            ctx,
            "rust",
            &ctx.config.rust.policy,
            ctx.config.rust.policy.lint_changes,
            RustAdapter::new,
        ),
        ProjectLanguage::Go => check_language_lint_policy(
            ctx,
            "go",
            &ctx.config.golang.policy,
            ctx.config.golang.policy.lint_changes,
            GoAdapter::new,
        ),
        ProjectLanguage::Python => check_language_lint_policy(
            ctx,
            "python",
            &ctx.config.python.policy,
            ctx.config.python.policy.lint_changes,
            PythonAdapter::new,
        ),
        ProjectLanguage::Ruby => check_language_lint_policy(
            ctx,
            "ruby",
            &ctx.config.ruby.policy,
            ctx.config.ruby.policy.lint_changes,
            RubyAdapter::new,
        ),
        ProjectLanguage::Shell => check_language_lint_policy(
            ctx,
            "shell",
            &ctx.config.shell.policy,
            ctx.config.shell.policy.lint_changes,
            ShellAdapter::new,
        ),
        ProjectLanguage::JavaScript => check_language_lint_policy(
            ctx,
            "javascript",
            &ctx.config.javascript.policy,
            ctx.config.javascript.policy.lint_changes,
            JavaScriptAdapter::new,
        ),
        ProjectLanguage::Generic => PolicyCheckResult {
            violations: Vec::new(),
            check_level: CheckLevel::Off,
        },
    }
}

/// Generic lint policy check for any language adapter.
fn check_language_lint_policy<P, A, F>(
    ctx: &CheckContext,
    language: &str,
    policy_config: &P,
    lint_changes: LintChangesPolicy,
    make_adapter: F,
) -> PolicyCheckResult
where
    P: PolicyConfig,
    A: crate::adapter::Adapter,
    F: FnOnce() -> A,
{
    let check_level = ctx.config.policy_check_level_for_language(language);

    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if lint_changes != LintChangesPolicy::Standalone {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    let Some(changed_files) = ctx.changed_files else {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    };

    let adapter = make_adapter();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = policy::check_lint_policy(&file_refs, policy_config, |p| adapter.classify(p));
    PolicyCheckResult {
        violations: make_policy_violation(
            result.standalone_violated,
            &result.changed_lint_config,
            &result.changed_source,
        ),
        check_level,
    }
}

/// Create policy violation if standalone policy was violated.
fn make_policy_violation(
    violated: bool,
    lint_config: &[String],
    source: &[String],
) -> Vec<Violation> {
    if !violated {
        return Vec::new();
    }
    vec![Violation {
        file: None,
        line: None,
        violation_type: "lint_policy".to_string(),
        advice: format!(
            "Changed lint config: {}\nAlso changed source: {}\nSubmit lint config changes in a separate PR.",
            lint_config.join(", "),
            truncate_list(source, 3),
        ),
        value: None,
        threshold: None,
        pattern: Some("lint_changes = standalone".to_string()),
        lines: None,
        nonblank: None,
        other_file: None,
        section: None,
        commit: None,
        message: None,
        expected_docs: None,
        area: None,
        area_match: None,
        path: None,
        target: None,
        change_type: None,
        lines_changed: None,
        scope: None,
        expected: None,
        found: None,
    }]
}

/// Truncate a list for display, showing "and N more" if needed.
fn truncate_list(items: &[String], max: usize) -> String {
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<_> = items.iter().take(max).cloned().collect();
        format!("{} and {} more", shown.join(", "), items.len() - max)
    }
}
