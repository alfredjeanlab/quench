// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for agent documentation detection.

use super::*;

// =============================================================================
// TYPE PREFIX DETECTION
// =============================================================================

#[test]
fn detects_feat_colon() {
    let content = "Use `feat:` for new features.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_fix_paren() {
    let content = "Use `fix(scope):` for bug fixes.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_type_in_example() {
    let content = "Example: `chore: update deps`";
    assert!(has_commit_documentation(content));
}

#[test]
fn requires_colon_or_paren_after_type() {
    // "feat" alone should not match
    let content = "This project features cool stuff.";
    assert!(!has_commit_documentation(content));
}

#[test]
fn case_insensitive_type_detection() {
    let content = "Use FEAT: for features";
    assert!(has_commit_documentation(content));
}

// =============================================================================
// CONVENTIONAL COMMITS PHRASE
// =============================================================================

#[test]
fn detects_conventional_commits_phrase() {
    let content = "We use Conventional Commits.";
    assert!(has_commit_documentation(content));
}

#[test]
fn detects_conventional_commit_singular() {
    let content = "Follow the conventional commit format.";
    assert!(has_commit_documentation(content));
}

#[test]
fn conventional_commits_case_insensitive() {
    let content = "Use CONVENTIONAL COMMITS format.";
    assert!(has_commit_documentation(content));
}

// =============================================================================
// NEGATIVE CASES
// =============================================================================

#[test]
fn no_detection_in_unrelated_content() {
    let content = "# Project\n\nThis is a project about features.";
    assert!(!has_commit_documentation(content));
}

#[test]
fn no_detection_in_empty_content() {
    let content = "";
    assert!(!has_commit_documentation(content));
}

// =============================================================================
// INTEGRATION WITH FILE CHECKING
// =============================================================================

#[test]
fn check_commit_docs_finds_in_claude_md() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\n## Commits\n\nUse feat: format.\n")
        .unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::Found(file) => assert_eq!(file, "CLAUDE.md"),
        other => panic!("Expected Found, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_not_found_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("CLAUDE.md"), "# Project\n\nNo commit info.\n").unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::NotFound(files) => {
            assert!(files.contains(&"CLAUDE.md".to_string()));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_no_agent_files() {
    let temp = tempfile::tempdir().unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::NoAgentFiles => {}
        other => panic!("Expected NoAgentFiles, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_finds_in_agents_md() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "# Agents\n\n## Commits\n\nUse fix: format.\n")
        .unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::Found(file) => assert_eq!(file, "AGENTS.md"),
        other => panic!("Expected Found, got {:?}", other),
    }
}

#[test]
fn check_commit_docs_finds_in_cursorrules() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join(".cursorrules"),
        "# Cursor Rules\n\nWe use conventional commits.\n",
    )
    .unwrap();

    match check_commit_docs(temp.path()) {
        DocsResult::Found(file) => assert_eq!(file, ".cursorrules"),
        other => panic!("Expected Found, got {:?}", other),
    }
}

#[test]
fn primary_agent_file_returns_first_existing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "content").unwrap();
    std::fs::write(temp.path().join(".cursorrules"), "content").unwrap();

    // CLAUDE.md doesn't exist, should return AGENTS.md
    assert_eq!(primary_agent_file(temp.path()), "AGENTS.md");
}

#[test]
fn primary_agent_file_returns_claude_md_by_default() {
    let temp = tempfile::tempdir().unwrap();

    // No agent files exist, should return CLAUDE.md as default
    assert_eq!(primary_agent_file(temp.path()), "CLAUDE.md");
}
