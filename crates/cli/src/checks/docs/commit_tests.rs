//! Unit tests for commit checking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// CONVENTIONAL COMMIT PARSING
// =============================================================================

#[test]
fn parses_feat_without_scope() {
    let result = parse_commit_line("abc1234567890 feat: add new feature");
    let commit = result.unwrap();
    assert_eq!(commit.hash, "abc1234");
    assert_eq!(commit.commit_type, "feat");
    assert!(commit.scope.is_none());
    assert_eq!(commit.message, "feat: add new feature");
}

#[test]
fn parses_feat_with_scope() {
    let result = parse_commit_line("def4567890123 feat(api): add endpoint");
    let commit = result.unwrap();
    assert_eq!(commit.hash, "def4567");
    assert_eq!(commit.commit_type, "feat");
    assert_eq!(commit.scope.as_deref(), Some("api"));
    assert_eq!(commit.message, "feat(api): add endpoint");
}

#[test]
fn parses_uppercase_type_as_lowercase() {
    let result = parse_commit_line("abc1234567890 FEAT: uppercase type");
    let commit = result.unwrap();
    assert_eq!(commit.commit_type, "feat");
}

#[test]
fn rejects_non_conventional_commit() {
    let result = parse_commit_line("abc1234567890 Add feature without prefix");
    assert!(result.is_none());
}

#[test]
fn rejects_missing_colon() {
    let result = parse_commit_line("abc1234567890 feat add feature");
    assert!(result.is_none());
}

#[test]
fn parses_breaking_type() {
    let result = parse_commit_line("abc1234567890 breaking: remove api");
    let commit = result.unwrap();
    assert_eq!(commit.commit_type, "breaking");
}

#[test]
fn parses_fix_type() {
    let result = parse_commit_line("abc1234567890 fix: bug in code");
    let commit = result.unwrap();
    assert_eq!(commit.commit_type, "fix");
}

// =============================================================================
// PATTERN MATCHING
// =============================================================================

#[test]
fn matches_docs_wildcard() {
    let files = vec![
        "docs/api/endpoints.md".to_string(),
        "src/lib.rs".to_string(),
    ];
    assert!(has_changes_matching(&files, "docs/**"));
}

#[test]
fn matches_specific_docs_path() {
    let files = vec![
        "docs/api/endpoints.md".to_string(),
        "src/lib.rs".to_string(),
    ];
    assert!(has_changes_matching(&files, "docs/api/**"));
}

#[test]
fn no_match_when_no_docs() {
    let files = vec!["src/lib.rs".to_string(), "tests/test.rs".to_string()];
    assert!(!has_changes_matching(&files, "docs/**"));
}

#[test]
fn no_match_wrong_area() {
    let files = vec!["docs/cli/commands.md".to_string()];
    assert!(!has_changes_matching(&files, "docs/api/**"));
}

// =============================================================================
// COMMIT VALIDATION
// =============================================================================

#[test]
fn check_commit_has_docs_with_area_mapping() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );

    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: Some("api".to_string()),
        message: "feat(api): add endpoint".to_string(),
    };

    // With matching docs
    let files_with_docs = vec!["docs/api/endpoints.md".to_string()];
    let (has_docs, pattern) = check_commit_has_docs(&commit, &files_with_docs, &areas);
    assert!(has_docs);
    assert_eq!(pattern.as_deref(), Some("docs/api/**"));

    // Without matching docs
    let files_without_docs = vec!["docs/cli/commands.md".to_string()];
    let (has_docs, pattern) = check_commit_has_docs(&commit, &files_without_docs, &areas);
    assert!(!has_docs);
    assert_eq!(pattern.as_deref(), Some("docs/api/**"));
}

#[test]
fn check_commit_has_docs_without_scope_uses_default() {
    let areas = HashMap::new();

    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: add feature".to_string(),
    };

    // With docs/ changes
    let files_with_docs = vec!["docs/guide.md".to_string()];
    let (has_docs, pattern) = check_commit_has_docs(&commit, &files_with_docs, &areas);
    assert!(has_docs);
    assert!(pattern.is_none());

    // Without docs/ changes
    let files_without_docs = vec!["src/lib.rs".to_string()];
    let (has_docs, pattern) = check_commit_has_docs(&commit, &files_without_docs, &areas);
    assert!(!has_docs);
    assert!(pattern.is_none());
}

#[test]
fn check_commit_with_unknown_scope_uses_default() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: None,
        },
    );

    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: Some("unknown".to_string()),
        message: "feat(unknown): something".to_string(),
    };

    // With generic docs/ changes
    let files = vec!["docs/guide.md".to_string()];
    let (has_docs, pattern) = check_commit_has_docs(&commit, &files, &areas);
    assert!(has_docs);
    assert!(pattern.is_none());
}

// =============================================================================
// VIOLATION CREATION
// =============================================================================

#[test]
fn creates_violation_with_expected_docs() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: Some("api".to_string()),
        message: "feat(api): add endpoint".to_string(),
    };

    let v = create_violation(&commit, Some("docs/api/**"));
    assert_eq!(v.commit.as_deref(), Some("abc1234"));
    assert_eq!(v.message.as_deref(), Some("feat(api): add endpoint"));
    assert_eq!(v.violation_type, "missing_docs");
    assert_eq!(v.expected_docs.as_deref(), Some("docs/api/**"));
    assert!(v.advice.contains("docs/api/**"));
}

#[test]
fn creates_violation_without_expected_docs() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: add feature".to_string(),
    };

    let v = create_violation(&commit, None);
    assert_eq!(v.commit.as_deref(), Some("abc1234"));
    assert!(v.expected_docs.is_none());
    assert!(v.advice.contains("docs/"));
}
