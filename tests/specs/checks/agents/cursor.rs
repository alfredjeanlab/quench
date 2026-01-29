// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Behavioral specs for cursor rule reconciliation.
//!
//! Tests that quench correctly reconciles `.cursor/rules/*.mdc` files
//! with CLAUDE.md / AGENTS.md agent files.
//!
//! Reference: docs/specs/checks/agents.cursor.md

use crate::prelude::*;

// =============================================================================
// ALWAYS-APPLY RECONCILIATION
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#alwaysapply-rules
///
/// > alwaysApply rules in sync with CLAUDE.md should pass
#[test]
fn always_apply_in_sync_passes() {
    check("agents").on("agents/cursor-always-apply").passes();
}

/// Spec: docs/specs/checks/agents.cursor.md#alwaysapply-rules
///
/// > alwaysApply rule with section not in CLAUDE.md generates violation
#[test]
fn always_apply_out_of_sync_fails() {
    let result = check("agents")
        .on("agents/cursor-out-of-sync")
        .json()
        .fails();
    assert!(
        result.has_violation("cursor_missing_in_claude"),
        "should detect section missing from CLAUDE.md"
    );
}

/// Spec: docs/specs/checks/agents.cursor.md#alwaysapply-rules
///
/// > Out-of-sync violation identifies the missing section
#[test]
fn always_apply_out_of_sync_identifies_section() {
    let result = check("agents")
        .on("agents/cursor-out-of-sync")
        .json()
        .fails();
    let violations = result.violations();

    let has_testing_section = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("cursor_missing_in_claude")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Testing"))
                .unwrap_or(false)
    });

    assert!(
        has_testing_section,
        "should identify 'Testing' as the missing section"
    );
}

// =============================================================================
// DIRECTORY-SCOPED RECONCILIATION
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#directory-scoped-rules
///
/// > Directory-scoped rule with matching agent file should pass
#[test]
fn directory_scoped_in_sync_passes() {
    check("agents").on("agents/cursor-dir-scope").passes();
}

/// Spec: docs/specs/checks/agents.cursor.md#directory-scoped-rules
///
/// > Directory-scoped rule with no agent file generates violation
#[test]
fn directory_scoped_no_agent_file_fails() {
    let result = check("agents").on("agents/cursor-no-claude").json().fails();
    assert!(
        result.has_violation("cursor_no_agent_file"),
        "should detect missing agent file for directory-scoped rule"
    );
}

/// Spec: docs/specs/checks/agents.cursor.md#directory-scoped-rules
///
/// > No-agent-file violation identifies the target directory
#[test]
fn directory_scoped_no_agent_file_identifies_directory() {
    let result = check("agents").on("agents/cursor-no-claude").json().fails();
    let violations = result.violations();

    let has_src_api = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("cursor_no_agent_file")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("src/api"))
                .unwrap_or(false)
    });

    assert!(
        has_src_api,
        "should identify src/api as the target directory"
    );
}

// =============================================================================
// MIXED RULES
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#rule-scope-classification
///
/// > File-pattern rules are not reconciled
#[test]
fn mixed_rules_file_pattern_not_reconciled() {
    // The cursor-mixed-rules fixture has all three types; only
    // alwaysApply and directory-scoped are reconciled. File-pattern
    // (.tsx) rules should not cause violations.
    check("agents").on("agents/cursor-mixed-rules").passes();
}

// =============================================================================
// PARSE ERRORS
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > Malformed .mdc frontmatter generates cursor_parse_error
#[test]
fn malformed_mdc_generates_parse_error() {
    let result = check("agents")
        .on("agents/cursor-mdc-invalid")
        .json()
        .fails();
    assert!(
        result.has_violation("cursor_parse_error"),
        "should detect malformed .mdc frontmatter"
    );
}

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#disabling-reconciliation
///
/// > reconcile_cursor = false disables all cursor reconciliation
#[test]
fn reconcile_cursor_false_disables_reconciliation() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
sections.required = []
max_lines = false
max_tokens = false
reconcile_cursor = false
"#,
    );
    temp.file("CLAUDE.md", "# Project\n\n## Code Style\n\nContent.\n");
    temp.file(
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Different Section\n\nDifferent content.\n",
    );

    // Should pass because reconciliation is disabled
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.cursor.md#one-way-reconciliation
///
/// > cursor_to_claude direction only checks cursor → agent
#[test]
fn cursor_to_claude_direction_only_checks_forward() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
sections.required = []
max_lines = false
max_tokens = false
reconcile_cursor = true
reconcile_direction = "cursor_to_claude"
"#,
    );
    // CLAUDE.md has a section NOT in cursor - should not be flagged
    temp.file(
        "CLAUDE.md",
        "## Code Style\n\nContent.\n\n## Extra Section\n\nClaude-only.\n",
    );
    temp.file(
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nContent.\n",
    );

    // Should pass - extra CLAUDE.md sections not flagged in cursor_to_claude mode
    check("agents").pwd(temp.path()).passes();
}

// =============================================================================
// EDGE CASES
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#mdc-frontmatter
///
/// > .mdc with no frontmatter is treated as non-always-apply
#[test]
fn no_frontmatter_not_reconciled() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
sections.required = []
max_lines = false
max_tokens = false
reconcile_cursor = true
"#,
    );
    temp.file("CLAUDE.md", "# Project\n");
    temp.file(
        ".cursor/rules/plain.mdc",
        "## Just Markdown\n\nNo frontmatter, not reconciled.\n",
    );

    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.cursor.md#content-comparison
///
/// > Empty .mdc body has no sections to reconcile
#[test]
fn empty_body_not_reconciled() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
sections.required = []
max_lines = false
max_tokens = false
reconcile_cursor = true
reconcile_direction = "cursor_to_claude"
"#,
    );
    temp.file("CLAUDE.md", "## Code Style\n\nContent.\n");
    temp.file(".cursor/rules/empty.mdc", "---\nalwaysApply: true\n---\n");

    check("agents").pwd(temp.path()).passes();
}
