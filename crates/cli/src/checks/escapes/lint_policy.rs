// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Lint policy checking for the escapes check.

use std::path::Path;

use crate::adapter::{
    GoAdapter, JavaScriptAdapter, ProjectLanguage, RustAdapter, ShellAdapter, detect_language,
};
use crate::check::{CheckContext, Violation};
use crate::config::{GoConfig, JavaScriptConfig, LintChangesPolicy, RustConfig, ShellConfig};

/// Check lint policy and return any violations.
pub fn check_lint_policy(ctx: &CheckContext) -> Vec<Violation> {
    match detect_language(ctx.root) {
        ProjectLanguage::Rust => check_rust_lint_policy(ctx, &ctx.config.rust),
        ProjectLanguage::Go => check_go_lint_policy(ctx, &ctx.config.golang),
        ProjectLanguage::Shell => check_shell_lint_policy(ctx, &ctx.config.shell),
        ProjectLanguage::JavaScript => check_javascript_lint_policy(ctx, &ctx.config.javascript),
        ProjectLanguage::Generic => Vec::new(),
    }
}

/// Check Rust lint policy and generate violations.
fn check_rust_lint_policy(ctx: &CheckContext, rust_config: &RustConfig) -> Vec<Violation> {
    if rust_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }
    let Some(changed_files) = ctx.changed_files else {
        return Vec::new();
    };

    let adapter = RustAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &rust_config.policy);
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
    )
}

/// Check Go lint policy and generate violations.
fn check_go_lint_policy(ctx: &CheckContext, go_config: &GoConfig) -> Vec<Violation> {
    if go_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }
    let Some(changed_files) = ctx.changed_files else {
        return Vec::new();
    };

    let adapter = GoAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &go_config.policy);
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
    )
}

/// Check Shell lint policy and generate violations.
fn check_shell_lint_policy(ctx: &CheckContext, shell_config: &ShellConfig) -> Vec<Violation> {
    if shell_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }
    let Some(changed_files) = ctx.changed_files else {
        return Vec::new();
    };

    let adapter = ShellAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &shell_config.policy);
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
    )
}

/// Check JavaScript lint policy and generate violations.
fn check_javascript_lint_policy(
    ctx: &CheckContext,
    js_config: &JavaScriptConfig,
) -> Vec<Violation> {
    if js_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }
    let Some(changed_files) = ctx.changed_files else {
        return Vec::new();
    };

    let adapter = JavaScriptAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &js_config.policy);
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
    )
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
