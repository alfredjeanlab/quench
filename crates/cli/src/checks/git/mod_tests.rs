// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the git check validation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::field_reassign_with_default
)]

use super::*;
use crate::config::GitCommitConfig;
use crate::git::Commit;

/// Create a test commit with the given hash and message.
fn test_commit(hash: &str, message: &str) -> Commit {
    Commit {
        hash: hash.to_string(),
        message: message.to_string(),
    }
}

// =============================================================================
// BASIC CHECK TESTS
// =============================================================================

#[test]
fn git_check_name() {
    let check = GitCheck;
    assert_eq!(check.name(), "git");
}

#[test]
fn git_check_description() {
    let check = GitCheck;
    assert_eq!(check.description(), "Commit message format");
}

#[test]
fn git_check_default_disabled() {
    let check = GitCheck;
    assert!(!check.default_enabled());
}

// =============================================================================
// FORMAT VALIDATION TESTS
// =============================================================================

#[test]
fn validates_conventional_format() {
    let commit = test_commit("abc1234", "feat: add feature");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn validates_conventional_format_with_scope() {
    let commit = test_commit("abc1234", "feat(api): add endpoint");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_non_conventional_format() {
    let commit = test_commit("abc1234", "update stuff");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_format");
    assert_eq!(violations[0].commit, Some("abc1234".to_string()));
    assert_eq!(violations[0].message, Some("update stuff".to_string()));
}

#[test]
fn rejects_missing_colon() {
    let commit = test_commit("abc1234", "feat add feature");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_format");
}

// =============================================================================
// TYPE VALIDATION TESTS
// =============================================================================

#[test]
fn accepts_default_type() {
    let commit = test_commit("abc1234", "feat: add feature");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn accepts_all_default_types() {
    let config = GitCommitConfig::default();
    let default_types = [
        "feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style",
    ];

    for commit_type in default_types {
        let commit = test_commit("abc1234", &format!("{}: do something", commit_type));
        let mut violations = Vec::new();
        validate_commit(&commit, &config, &mut violations);
        assert!(
            violations.is_empty(),
            "type '{}' should be allowed by default",
            commit_type
        );
    }
}

#[test]
fn rejects_invalid_type_with_defaults() {
    let commit = test_commit("abc1234", "custom: do something");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_type");
    assert!(violations[0].advice.contains("Allowed types:"));
}

#[test]
fn accepts_custom_type_when_configured() {
    let commit = test_commit("abc1234", "custom: do something");
    let mut config = GitCommitConfig::default();
    config.types = Some(vec!["custom".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn any_type_allowed_with_empty_list() {
    let commit = test_commit("abc1234", "anything: do something");
    let mut config = GitCommitConfig::default();
    config.types = Some(vec![]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_type_not_in_custom_list() {
    let commit = test_commit("abc1234", "chore: do something");
    let mut config = GitCommitConfig::default();
    config.types = Some(vec!["feat".to_string(), "fix".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_type");
    assert!(violations[0].advice.contains("feat"));
    assert!(violations[0].advice.contains("fix"));
}

// =============================================================================
// SCOPE VALIDATION TESTS
// =============================================================================

#[test]
fn any_scope_allowed_when_not_configured() {
    let commit = test_commit("abc1234", "feat(random): add feature");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn accepts_configured_scope() {
    let commit = test_commit("abc1234", "feat(api): add endpoint");
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string(), "cli".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn rejects_invalid_scope() {
    let commit = test_commit("abc1234", "feat(unknown): add feature");
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string(), "cli".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_scope");
    assert!(violations[0].advice.contains("api"));
    assert!(violations[0].advice.contains("cli"));
}

#[test]
fn no_scope_allowed_when_scopes_configured() {
    // Commits without scope are allowed even when scopes are configured
    let commit = test_commit("abc1234", "feat: add feature");
    let mut config = GitCommitConfig::default();
    config.scopes = Some(vec!["api".to_string()]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

// =============================================================================
// CONFIG TESTS
// =============================================================================

#[test]
fn effective_format_defaults_to_conventional() {
    let config = GitCommitConfig::default();
    assert_eq!(config.effective_format(), "conventional");
}

#[test]
fn effective_format_respects_config() {
    let mut config = GitCommitConfig::default();
    config.format = Some("none".to_string());
    assert_eq!(config.effective_format(), "none");
}

#[test]
fn agents_defaults_to_true() {
    let config = GitCommitConfig::default();
    assert!(config.agents);
}

#[test]
fn template_defaults_to_true() {
    let config = GitCommitConfig::default();
    assert!(config.template);
}

// =============================================================================
// ADVICE FORMATTING TESTS
// =============================================================================

#[test]
fn format_type_advice_with_defaults() {
    let advice = format_type_advice(None);
    assert!(advice.contains("Allowed types:"));
    assert!(advice.contains("feat"));
    assert!(advice.contains("fix"));
}

#[test]
fn format_type_advice_with_empty_list() {
    let advice = format_type_advice(Some(&[]));
    assert_eq!(advice, "Any type allowed (check format only)");
}

#[test]
fn format_type_advice_with_custom_list() {
    let types = vec!["custom".to_string(), "special".to_string()];
    let advice = format_type_advice(Some(&types));
    assert!(advice.contains("custom"));
    assert!(advice.contains("special"));
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

// =============================================================================
// MERGE COMMIT HANDLING TESTS
// =============================================================================

#[test]
fn skips_merge_commit_by_default() {
    let commit = test_commit("abc1234", "Merge branch 'feature' into main");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    let validated = validate_commit(&commit, &config, &mut violations);

    assert!(!validated, "merge commit should be skipped");
    assert!(violations.is_empty());
}

#[test]
fn validates_merge_commit_when_skip_disabled() {
    let commit = test_commit("abc1234", "Merge branch 'feature' into main");
    let mut config = GitCommitConfig::default();
    config.skip_merge = false;
    let mut violations = Vec::new();

    let validated = validate_commit(&commit, &config, &mut violations);

    assert!(validated, "merge commit should be validated");
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].violation_type, "invalid_format");
}

#[test]
fn validates_breaking_change_marker() {
    // Breaking change marker `!` should be valid
    let commit = test_commit("abc1234", "feat!: breaking change");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(
        violations.is_empty(),
        "Breaking change marker should be valid"
    );
}

#[test]
fn validates_breaking_change_with_scope() {
    // Breaking change marker with scope should be valid
    let commit = test_commit("abc1234", "feat(api)!: breaking API change");
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(
        violations.is_empty(),
        "Breaking change with scope should be valid"
    );
}

#[test]
fn validates_revert_commit() {
    // Revert commits using conventional format
    let commit = test_commit("abc1234", "revert: undo previous change");
    let mut config = GitCommitConfig::default();
    // Add "revert" to allowed types
    config.types = Some(vec![
        "feat".to_string(),
        "fix".to_string(),
        "revert".to_string(),
    ]);
    let mut violations = Vec::new();

    validate_commit(&commit, &config, &mut violations);

    assert!(violations.is_empty());
}

#[test]
fn validates_multiple_commits_with_different_violations() {
    // Test that violations are counted per commit
    let commits = vec![
        test_commit("abc1234", "feat: valid commit"),
        test_commit("def5678", "invalid commit"),
        test_commit("ghi9012", "unknown: wrong type"),
    ];
    let config = GitCommitConfig::default();
    let mut violations = Vec::new();

    for commit in &commits {
        validate_commit(commit, &config, &mut violations);
    }

    // First commit: valid (0 violations)
    // Second commit: invalid_format (1 violation)
    // Third commit: invalid_type (1 violation)
    assert_eq!(violations.len(), 2);
}
