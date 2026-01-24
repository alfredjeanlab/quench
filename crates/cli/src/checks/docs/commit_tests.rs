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
    let result = check_commit_has_docs(&commit, &files_with_docs, &areas);
    assert!(result.has_docs);
    assert_eq!(result.matched_areas.len(), 1);
    assert_eq!(result.matched_areas[0].docs_pattern, "docs/api/**");
    assert_eq!(result.matched_areas[0].match_type, AreaMatchType::Scope);

    // Without matching docs
    let files_without_docs = vec!["docs/cli/commands.md".to_string()];
    let result = check_commit_has_docs(&commit, &files_without_docs, &areas);
    assert!(!result.has_docs);
    assert_eq!(result.matched_areas[0].docs_pattern, "docs/api/**");
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
    let result = check_commit_has_docs(&commit, &files_with_docs, &areas);
    assert!(result.has_docs);
    assert!(result.matched_areas.is_empty());

    // Without docs/ changes
    let files_without_docs = vec!["src/lib.rs".to_string()];
    let result = check_commit_has_docs(&commit, &files_without_docs, &areas);
    assert!(!result.has_docs);
    assert!(result.matched_areas.is_empty());
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
    let result = check_commit_has_docs(&commit, &files, &areas);
    assert!(result.has_docs);
    assert!(result.matched_areas.is_empty());
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

// =============================================================================
// SOURCE-BASED AREA DETECTION
// =============================================================================

#[test]
fn finds_areas_from_source_changes() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );
    areas.insert(
        "cli".to_string(),
        DocsAreaConfig {
            docs: "docs/cli/**".to_string(),
            source: Some("src/cli/**".to_string()),
        },
    );

    let files = vec!["src/api/handler.rs".to_string()];
    let matched = find_areas_from_source(&files, &areas);

    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].0, "api");
}

#[test]
fn finds_multiple_areas_from_source_changes() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );
    areas.insert(
        "cli".to_string(),
        DocsAreaConfig {
            docs: "docs/cli/**".to_string(),
            source: Some("src/cli/**".to_string()),
        },
    );

    let files = vec![
        "src/api/handler.rs".to_string(),
        "src/cli/main.rs".to_string(),
    ];
    let matched = find_areas_from_source(&files, &areas);

    assert_eq!(matched.len(), 2);
}

#[test]
fn ignores_areas_without_source_pattern() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: None, // No source pattern
        },
    );

    let files = vec!["src/api/handler.rs".to_string()];
    let matched = find_areas_from_source(&files, &areas);

    assert!(matched.is_empty());
}

#[test]
fn no_areas_matched_when_files_dont_match_patterns() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );

    let files = vec!["src/cli/main.rs".to_string()];
    let matched = find_areas_from_source(&files, &areas);

    assert!(matched.is_empty());
}

// =============================================================================
// SOURCE-BASED COMMIT CHECKING
// =============================================================================

#[test]
fn check_commit_uses_source_matching_when_no_scope() {
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
        scope: None, // No scope
        message: "feat: add api handler".to_string(),
    };

    // Source files match api area, no docs
    let files = vec!["src/api/handler.rs".to_string()];
    let result = check_commit_has_docs(&commit, &files, &areas);
    assert!(!result.has_docs);
    assert_eq!(result.matched_areas.len(), 1);
    assert_eq!(result.matched_areas[0].name, "api");
    assert_eq!(result.matched_areas[0].match_type, AreaMatchType::Source);
}

#[test]
fn check_commit_source_match_passes_with_docs() {
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
        scope: None,
        message: "feat: add api handler".to_string(),
    };

    // Source files match and docs exist
    let files = vec![
        "src/api/handler.rs".to_string(),
        "docs/api/handler.md".to_string(),
    ];
    let result = check_commit_has_docs(&commit, &files, &areas);
    assert!(result.has_docs);
    assert_eq!(result.matched_areas.len(), 1);
    assert!(result.matched_areas[0].has_docs);
}

#[test]
fn check_commit_multiple_source_areas_require_all_docs() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );
    areas.insert(
        "cli".to_string(),
        DocsAreaConfig {
            docs: "docs/cli/**".to_string(),
            source: Some("src/cli/**".to_string()),
        },
    );

    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: refactor".to_string(),
    };

    // Changes both areas but only has api docs
    let files = vec![
        "src/api/handler.rs".to_string(),
        "src/cli/main.rs".to_string(),
        "docs/api/handler.md".to_string(),
    ];
    let result = check_commit_has_docs(&commit, &files, &areas);
    assert!(!result.has_docs); // Missing cli docs
    assert_eq!(result.matched_areas.len(), 2);
}

#[test]
fn check_commit_scope_takes_priority_over_source() {
    let mut areas = HashMap::new();
    areas.insert(
        "api".to_string(),
        DocsAreaConfig {
            docs: "docs/api/**".to_string(),
            source: Some("src/api/**".to_string()),
        },
    );
    areas.insert(
        "cli".to_string(),
        DocsAreaConfig {
            docs: "docs/cli/**".to_string(),
            source: Some("src/cli/**".to_string()),
        },
    );

    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: Some("api".to_string()), // Has scope
        message: "feat(api): add handler".to_string(),
    };

    // Changes both areas source files, but commit has api scope
    // Should only require api docs (scope takes priority)
    let files = vec![
        "src/api/handler.rs".to_string(),
        "src/cli/main.rs".to_string(),
        "docs/api/handler.md".to_string(),
    ];
    let result = check_commit_has_docs(&commit, &files, &areas);
    assert!(result.has_docs); // Only api docs needed due to scope
    assert_eq!(result.matched_areas.len(), 1);
    assert_eq!(result.matched_areas[0].name, "api");
    assert_eq!(result.matched_areas[0].match_type, AreaMatchType::Scope);
}

// =============================================================================
// AREA VIOLATION CREATION
// =============================================================================

#[test]
fn creates_area_violation_with_scope_match() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: Some("api".to_string()),
        message: "feat(api): add endpoint".to_string(),
    };

    let area = MatchedArea {
        name: "api".to_string(),
        docs_pattern: "docs/api/**".to_string(),
        match_type: AreaMatchType::Scope,
        has_docs: false,
    };

    let v = create_area_violation(&commit, &area);
    assert_eq!(v.area.as_deref(), Some("api"));
    assert_eq!(v.area_match.as_deref(), Some("scope"));
    assert!(v.advice.contains("feat(api):"));
}

#[test]
fn creates_area_violation_with_source_match() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: add handler".to_string(),
    };

    let area = MatchedArea {
        name: "api".to_string(),
        docs_pattern: "docs/api/**".to_string(),
        match_type: AreaMatchType::Source,
        has_docs: false,
    };

    let v = create_area_violation(&commit, &area);
    assert_eq!(v.area.as_deref(), Some("api"));
    assert_eq!(v.area_match.as_deref(), Some("source"));
    assert!(v.advice.contains("changes in api area"));
}

#[test]
fn creates_multiple_violations_for_multiple_areas() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: refactor".to_string(),
    };

    let result = DocCheckResult {
        has_docs: false,
        matched_areas: vec![
            MatchedArea {
                name: "api".to_string(),
                docs_pattern: "docs/api/**".to_string(),
                match_type: AreaMatchType::Source,
                has_docs: false,
            },
            MatchedArea {
                name: "cli".to_string(),
                docs_pattern: "docs/cli/**".to_string(),
                match_type: AreaMatchType::Source,
                has_docs: false,
            },
        ],
    };

    let violations = create_violations_for_commit(&commit, &result);
    assert_eq!(violations.len(), 2);
}

#[test]
fn creates_only_violations_for_missing_docs() {
    let commit = ConventionalCommit {
        hash: "abc1234".to_string(),
        commit_type: "feat".to_string(),
        scope: None,
        message: "feat: refactor".to_string(),
    };

    let result = DocCheckResult {
        has_docs: false,
        matched_areas: vec![
            MatchedArea {
                name: "api".to_string(),
                docs_pattern: "docs/api/**".to_string(),
                match_type: AreaMatchType::Source,
                has_docs: true, // This one has docs
            },
            MatchedArea {
                name: "cli".to_string(),
                docs_pattern: "docs/cli/**".to_string(),
                match_type: AreaMatchType::Source,
                has_docs: false, // Missing docs
            },
        ],
    };

    let violations = create_violations_for_commit(&commit, &result);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].area.as_deref(), Some("cli"));
}
