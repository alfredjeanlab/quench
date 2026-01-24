// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Lint policy checking for the escapes check.

use std::path::Path;

use crate::adapter::{
    GoAdapter, JavaScriptAdapter, ProjectLanguage, RustAdapter, ShellAdapter, detect_language,
};
use crate::check::{CheckContext, Violation};
use crate::config::{
    CheckLevel, GoConfig, JavaScriptConfig, LintChangesPolicy, RustConfig, ShellConfig,
};

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
        ProjectLanguage::Rust => check_rust_lint_policy(ctx, &ctx.config.rust),
        ProjectLanguage::Go => check_go_lint_policy(ctx, &ctx.config.golang),
        ProjectLanguage::Shell => check_shell_lint_policy(ctx, &ctx.config.shell),
        ProjectLanguage::JavaScript => check_javascript_lint_policy(ctx, &ctx.config.javascript),
        ProjectLanguage::Generic => PolicyCheckResult {
            violations: Vec::new(),
            check_level: CheckLevel::Off,
        },
    }
}

/// Check Rust lint policy and generate violations.
fn check_rust_lint_policy(ctx: &CheckContext, rust_config: &RustConfig) -> PolicyCheckResult {
    let check_level = ctx.config.policy_check_level_for_language("rust");

    // If policy check is off, skip entirely
    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if rust_config.policy.lint_changes != LintChangesPolicy::Standalone {
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

    let adapter = RustAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &rust_config.policy);
    PolicyCheckResult {
        violations: make_policy_violation(
            result.standalone_violated,
            &result.changed_lint_config,
            &result.changed_source,
        ),
        check_level,
    }
}

/// Check Go lint policy and generate violations.
fn check_go_lint_policy(ctx: &CheckContext, go_config: &GoConfig) -> PolicyCheckResult {
    let check_level = ctx.config.policy_check_level_for_language("go");

    // If policy check is off, skip entirely
    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if go_config.policy.lint_changes != LintChangesPolicy::Standalone {
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

    let adapter = GoAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &go_config.policy);
    PolicyCheckResult {
        violations: make_policy_violation(
            result.standalone_violated,
            &result.changed_lint_config,
            &result.changed_source,
        ),
        check_level,
    }
}

/// Check Shell lint policy and generate violations.
fn check_shell_lint_policy(ctx: &CheckContext, shell_config: &ShellConfig) -> PolicyCheckResult {
    let check_level = ctx.config.policy_check_level_for_language("shell");

    // If policy check is off, skip entirely
    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if shell_config.policy.lint_changes != LintChangesPolicy::Standalone {
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

    let adapter = ShellAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &shell_config.policy);
    PolicyCheckResult {
        violations: make_policy_violation(
            result.standalone_violated,
            &result.changed_lint_config,
            &result.changed_source,
        ),
        check_level,
    }
}

/// Check JavaScript lint policy and generate violations.
fn check_javascript_lint_policy(
    ctx: &CheckContext,
    js_config: &JavaScriptConfig,
) -> PolicyCheckResult {
    let check_level = ctx.config.policy_check_level_for_language("javascript");

    // If policy check is off, skip entirely
    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if js_config.policy.lint_changes != LintChangesPolicy::Standalone {
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

    let adapter = JavaScriptAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &js_config.policy);
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
