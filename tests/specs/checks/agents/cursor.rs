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
    let result = check("agents").on("agents/cursor-out-of-sync").json().fails();
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
    let result = check("agents").on("agents/cursor-out-of-sync").json().fails();
    let violations = result.violations();

    let has_testing_section = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("cursor_missing_in_claude")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Testing"))
                .unwrap_or(false)
    });

    assert!(has_testing_section, "should identify 'Testing' as the missing section");
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

    assert!(has_src_api, "should identify src/api as the target directory");
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
    let result = check("agents").on("agents/cursor-mdc-invalid").json().fails();
    assert!(result.has_violation("cursor_parse_error"), "should detect malformed .mdc frontmatter");
}

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#disabling-reconciliation
///
/// > sync = false disables all cursor reconciliation
#[test]
fn sync_false_disables_reconciliation() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = false
sections.required = []
max_lines = false
max_tokens = false
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
/// > .mdc sync_from only checks cursor → agent (CursorToClaude)
#[test]
fn mdc_sync_source_only_checks_cursor_to_claude() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = true
files = [".cursor/rules/*.mdc", "CLAUDE.md"]
sync_from = ".cursor/rules/general.mdc"
sections.required = []
max_lines = false
max_tokens = false
"#,
    );
    // CLAUDE.md has extra section - should NOT be flagged in CursorToClaude mode
    temp.file("CLAUDE.md", "## Code Style\n\nContent.\n\n## Extra Section\n\nClaude-only.\n");
    temp.file(
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nContent.\n",
    );

    // Should pass - extra CLAUDE.md sections not flagged in CursorToClaude mode
    check("agents").pwd(temp.path()).passes();
}

/// Spec: docs/specs/checks/agents.cursor.md#one-way-reconciliation
///
/// > CLAUDE.md sync_from checks agent → cursor (ClaudeToCursor)
#[test]
fn claude_sync_source_checks_claude_to_cursor() {
    let temp = Project::empty();
    temp.config(
        r#"[check.agents]
required = []
sync = true
sync_from = "CLAUDE.md"
sections.required = []
max_lines = false
max_tokens = false
"#,
    );
    // CLAUDE.md has section not in cursor - should be flagged
    temp.file("CLAUDE.md", "## Code Style\n\nContent.\n\n## Testing\n\nTest guidelines.\n");
    temp.file(
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nContent.\n",
    );

    let result = check("agents").pwd(temp.path()).json().fails();
    assert!(result.has_violation("claude_missing_in_cursor"));
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
sections.required = []
max_lines = false
max_tokens = false
"#,
    );
    temp.file("CLAUDE.md", "# Project\n");
    temp.file(".cursor/rules/plain.mdc", "## Just Markdown\n\nNo frontmatter, not reconciled.\n");

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
sync = true
sync_from = ".cursor/rules/empty.mdc"
sections.required = []
max_lines = false
max_tokens = false
"#,
    );
    temp.file("CLAUDE.md", "## Code Style\n\nContent.\n");
    temp.file(".cursor/rules/empty.mdc", "---\nalwaysApply: true\n---\n");

    check("agents").pwd(temp.path()).passes();
}

// =============================================================================
// OUTPUT FORMAT VALIDATION
// =============================================================================

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_missing_in_claude text output format
#[test]
fn cursor_missing_in_claude_text_format() {
    check("agents")
        .on("agents/cursor-out-of-sync")
        .exits(1)
        .stdout_eq(
            "agents: FAIL
  .cursor/rules/general.mdc: cursor_missing_in_claude: CLAUDE.md
    Section \"Testing\" exists in .cursor/rules/general.mdc (alwaysApply) but not in CLAUDE.md. Use --fix to add missing sections.
FAIL: agents
",
        );
}

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_no_agent_file text output format
#[test]
fn cursor_no_agent_file_text_format() {
    check("agents")
        .on("agents/cursor-no-claude")
        .exits(1)
        .stdout_eq(
            "agents: FAIL
  .cursor/rules/api.mdc: cursor_no_agent_file: src/api/CLAUDE.md
    Rule scoped to src/api/ but no CLAUDE.md found there. Use --fix to create src/api/CLAUDE.md from rule content.
FAIL: agents
",
        );
}

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_parse_error text output format
#[test]
fn cursor_parse_error_text_format() {
    check("agents").on("agents/cursor-mdc-invalid").exits(1).stdout_eq(
        "agents: FAIL
  .cursor/rules/bad.mdc: cursor_parse_error
    Malformed .mdc frontmatter: unterminated frontmatter (missing closing ---)
FAIL: agents
",
    );
}

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_missing_in_claude JSON output format
#[test]
fn cursor_missing_in_claude_json_format() {
    let result = check("agents").on("agents/cursor-out-of-sync").json().fails();

    let violation = result.require_violation("cursor_missing_in_claude");
    assert_eq!(violation.get("file").and_then(|f| f.as_str()), Some(".cursor/rules/general.mdc"));
    assert_eq!(violation.get("type").and_then(|f| f.as_str()), Some("cursor_missing_in_claude"));
    assert_eq!(violation.get("target").and_then(|f| f.as_str()), Some("CLAUDE.md"));
    assert!(
        violation.get("advice").and_then(|a| a.as_str()).unwrap().contains("Section \"Testing\"")
    );
}

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_no_agent_file JSON output format
#[test]
fn cursor_no_agent_file_json_format() {
    let result = check("agents").on("agents/cursor-no-claude").json().fails();

    let violation = result.require_violation("cursor_no_agent_file");
    assert_eq!(violation.get("file").and_then(|f| f.as_str()), Some(".cursor/rules/api.mdc"));
    assert_eq!(violation.get("type").and_then(|f| f.as_str()), Some("cursor_no_agent_file"));
    assert_eq!(violation.get("target").and_then(|f| f.as_str()), Some("src/api/CLAUDE.md"));
    assert!(violation.get("advice").and_then(|a| a.as_str()).unwrap().contains("src/api/"));
}

/// Spec: docs/specs/checks/agents.cursor.md#violation-types
///
/// > cursor_parse_error JSON output format
#[test]
fn cursor_parse_error_json_format() {
    let result = check("agents").on("agents/cursor-mdc-invalid").json().fails();

    let violation = result.require_violation("cursor_parse_error");
    assert_eq!(violation.get("file").and_then(|f| f.as_str()), Some(".cursor/rules/bad.mdc"));
    assert_eq!(violation.get("type").and_then(|f| f.as_str()), Some("cursor_parse_error"));
    assert!(
        violation
            .get("advice")
            .and_then(|a| a.as_str())
            .unwrap()
            .contains("unterminated frontmatter")
    );
}
