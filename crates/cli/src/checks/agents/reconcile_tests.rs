#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// HELPER
// =============================================================================

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn write_file(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

// =============================================================================
// ALWAYS-APPLY RECONCILIATION
// =============================================================================

#[test]
fn always_apply_in_sync() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        "CLAUDE.md",
        "# Project\n\n## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );
    write_file(
        root,
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n# General Rules\n\n## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );

    let (violations, _fixes) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::CursorToClaude,
        false,
        false,
    );

    // Preamble will differ (# Project vs stripped header content), but named sections match
    let named_violations: Vec<_> = violations
        .iter()
        .filter(|v| {
            v.section
                .as_deref()
                .map(|s| s != "(preamble)")
                .unwrap_or(true)
        })
        .collect();

    assert!(
        named_violations.is_empty(),
        "expected no violations for named sections, got: {:?}",
        named_violations
            .iter()
            .map(|v| &v.advice)
            .collect::<Vec<_>>()
    );
}

#[test]
fn always_apply_missing_section_in_claude() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        "CLAUDE.md",
        "# Project\n\n## Code Style\n\nUse 4 spaces.\n",
    );
    write_file(
        root,
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::CursorToClaude,
        false,
        false,
    );

    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_missing_in_claude"
                && v.section.as_deref() == Some("Testing")),
        "expected cursor_missing_in_claude for Testing section"
    );
}

#[test]
fn always_apply_missing_section_in_cursor() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        "CLAUDE.md",
        "## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );
    write_file(
        root,
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nUse 4 spaces.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::ClaudeToCursor,
        false,
        false,
    );

    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "claude_missing_in_cursor"
                && v.section.as_deref() == Some("Testing")),
        "expected claude_missing_in_cursor for Testing section"
    );
}

#[test]
fn always_apply_bidirectional() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        "CLAUDE.md",
        "## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );
    write_file(
        root,
        ".cursor/rules/general.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nUse 4 spaces.\n\n## Deployment\n\nUse CI.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    // Deployment missing in CLAUDE.md (cursor → claude)
    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_missing_in_claude"
                && v.section.as_deref() == Some("Deployment"))
    );

    // Testing missing in cursor (claude → cursor)
    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "claude_missing_in_cursor"
                && v.section.as_deref() == Some("Testing"))
    );
}

#[test]
fn always_apply_aggregate_multiple_mdc_files() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        "CLAUDE.md",
        "## Code Style\n\nUse 4 spaces.\n\n## Testing\n\nRun cargo test.\n",
    );
    write_file(
        root,
        ".cursor/rules/style.mdc",
        "---\nalwaysApply: true\n---\n\n## Code Style\n\nUse 4 spaces.\n",
    );
    write_file(
        root,
        ".cursor/rules/testing.mdc",
        "---\nalwaysApply: true\n---\n\n## Testing\n\nRun cargo test.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    // Both directions should be satisfied with aggregate coverage
    let named_violations: Vec<_> = violations
        .iter()
        .filter(|v| {
            v.section
                .as_deref()
                .map(|s| s != "(preamble)")
                .unwrap_or(true)
        })
        .collect();

    assert!(
        named_violations.is_empty(),
        "expected no violations for named sections with aggregate coverage, got: {:?}",
        named_violations
            .iter()
            .map(|v| &v.advice)
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// DIRECTORY-SCOPED RECONCILIATION
// =============================================================================

#[test]
fn directory_scoped_no_agent_file() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/api.mdc",
        "---\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n",
    );
    // No src/api/CLAUDE.md exists

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_no_agent_file")
    );
}

#[test]
fn directory_scoped_in_sync() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/api.mdc",
        "---\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n",
    );
    write_file(
        root,
        "src/api/CLAUDE.md",
        "## API Conventions\n\nUse REST.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::CursorToClaude,
        false,
        false,
    );

    assert!(
        violations.is_empty(),
        "expected no violations, got: {:?}",
        violations.iter().map(|v| &v.advice).collect::<Vec<_>>()
    );
}

#[test]
fn directory_scoped_missing_section() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/api.mdc",
        "---\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n\n## Authentication\n\nUse JWT.\n",
    );
    write_file(
        root,
        "src/api/CLAUDE.md",
        "## API Conventions\n\nUse REST.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::CursorToClaude,
        false,
        false,
    );

    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_dir_missing_in_agent"
                && v.section.as_deref() == Some("Authentication"))
    );
}

// =============================================================================
// FIX MODE
// =============================================================================

#[test]
fn fix_creates_missing_agent_file() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/api.mdc",
        "---\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n",
    );

    let (violations, fixes) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        true,
        false,
    );

    // Fix should have created the file
    assert!(!fixes.is_empty(), "expected at least one fix");
    let agent_path = root.join("src/api/CLAUDE.md");
    assert!(agent_path.exists(), "expected agent file to be created");

    let content = std::fs::read_to_string(agent_path).unwrap();
    assert!(content.contains("## API Conventions"));

    // Violation still reported even when fix applied
    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_no_agent_file")
    );
}

#[test]
fn fix_dry_run_does_not_write() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/api.mdc",
        "---\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n",
    );

    let (_violations, fixes) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        true,
        true,
    );

    assert!(!fixes.is_empty(), "expected at least one fix preview");
    let agent_path = root.join("src/api/CLAUDE.md");
    assert!(!agent_path.exists(), "dry run should not create files");
}

// =============================================================================
// PARSE ERRORS
// =============================================================================

#[test]
fn malformed_mdc_produces_parse_error() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/bad.mdc",
        "---\nalwaysApply: true\nNo closing delimiter\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    assert!(
        violations
            .iter()
            .any(|v| v.violation_type == "cursor_parse_error")
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn no_mdc_files_no_violations() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(root, "CLAUDE.md", "# Project\n\nContent.\n");

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    assert!(violations.is_empty());
}

#[test]
fn file_pattern_rules_not_reconciled() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/tsx.mdc",
        "---\nglobs: \"**/*.tsx\"\nalwaysApply: false\n---\n\n## TSX Rules\n\nUse functional components.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    // FilePattern rules should not produce reconciliation violations
    assert!(
        violations.is_empty(),
        "file-pattern rules should not be reconciled"
    );
}

#[test]
fn on_demand_rules_not_reconciled() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(
        root,
        ".cursor/rules/manual.mdc",
        "---\ndescription: \"Use when needed\"\nalwaysApply: false\n---\n\n## Manual Rule\n\nContent.\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::Bidirectional,
        false,
        false,
    );

    assert!(
        violations.is_empty(),
        "on-demand rules should not be reconciled"
    );
}

#[test]
fn empty_body_no_sections_to_reconcile() {
    let dir = temp_dir();
    let root = dir.path();

    write_file(root, "CLAUDE.md", "## Code Style\n\nUse 4 spaces.\n");
    write_file(
        root,
        ".cursor/rules/empty.mdc",
        "---\nalwaysApply: true\n---\n",
    );

    let (violations, _) = check_cursor_reconciliation(
        root,
        &["CLAUDE.md".to_string()],
        &ReconcileDirection::CursorToClaude,
        false,
        false,
    );

    // Empty body has no sections to check in cursor→claude direction
    assert!(
        violations.is_empty(),
        "empty body should not produce cursor→claude violations"
    );
}

// =============================================================================
// DERIVE DIRECTION FROM SYNC
// =============================================================================

#[test]
fn derive_direction_claude_source() {
    let dir = derive_direction_from_sync(true, Some("CLAUDE.md"), &["CLAUDE.md".to_string()]);
    assert_eq!(dir, Some(ReconcileDirection::ClaudeToCursor));
}

#[test]
fn derive_direction_agents_source() {
    let dir = derive_direction_from_sync(true, Some("AGENTS.md"), &["AGENTS.md".to_string()]);
    assert_eq!(dir, Some(ReconcileDirection::ClaudeToCursor));
}

#[test]
fn derive_direction_mdc_source() {
    let dir = derive_direction_from_sync(
        true,
        Some(".cursor/rules/api.mdc"),
        &["CLAUDE.md".to_string()],
    );
    assert_eq!(dir, Some(ReconcileDirection::CursorToClaude));
}

#[test]
fn derive_direction_mdc_source_ends_with_mdc() {
    let dir = derive_direction_from_sync(true, Some("custom.mdc"), &["CLAUDE.md".to_string()]);
    assert_eq!(dir, Some(ReconcileDirection::CursorToClaude));
}

#[test]
fn derive_direction_sync_disabled() {
    let dir = derive_direction_from_sync(false, Some("CLAUDE.md"), &[]);
    assert_eq!(dir, None);
}

#[test]
fn derive_direction_bidirectional_default() {
    let dir = derive_direction_from_sync(true, None, &["CLAUDE.md".to_string()]);
    assert_eq!(dir, Some(ReconcileDirection::Bidirectional));
}

#[test]
fn derive_direction_cursorrules_bidirectional() {
    let dir = derive_direction_from_sync(true, Some(".cursorrules"), &["CLAUDE.md".to_string()]);
    assert_eq!(dir, Some(ReconcileDirection::Bidirectional));
}

#[test]
fn derive_direction_no_source_is_bidirectional() {
    // When sync_source is not provided, default to bidirectional
    let dir = derive_direction_from_sync(
        true,
        None,
        &["CLAUDE.md".to_string(), "AGENTS.md".to_string()],
    );
    assert_eq!(dir, Some(ReconcileDirection::Bidirectional));
}
