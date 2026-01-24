// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for conventional commit parsing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// BASIC PARSING TESTS
// =============================================================================

#[test]
fn parses_type_and_description() {
    let result = parse_conventional_commit("fix: handle empty input");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.commit_type, "fix");
    assert_eq!(parsed.scope, None);
    assert_eq!(parsed.description, "handle empty input");
}

#[test]
fn parses_type_scope_and_description() {
    let result = parse_conventional_commit("feat(api): add export endpoint");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.commit_type, "feat");
    assert_eq!(parsed.scope, Some("api".to_string()));
    assert_eq!(parsed.description, "add export endpoint");
}

#[test]
fn rejects_message_without_type_prefix() {
    let result = parse_conventional_commit("update stuff");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn rejects_message_without_colon() {
    let result = parse_conventional_commit("feat add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn rejects_message_with_uppercase_type() {
    // Conventional commits use lowercase types
    let result = parse_conventional_commit("FEAT: add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn handles_description_with_colons() {
    let result = parse_conventional_commit("docs: update README: add examples");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.description, "update README: add examples");
}

#[test]
fn handles_empty_scope_parentheses() {
    // Empty parens should be rejected (no scope)
    let result = parse_conventional_commit("feat(): add feature");
    assert_eq!(result, ParseResult::NonConventional);
}

#[test]
fn handles_scope_with_hyphen() {
    let result = parse_conventional_commit("fix(user-auth): resolve login issue");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope, Some("user-auth".to_string()));
}

#[test]
fn handles_scope_with_underscore() {
    let result = parse_conventional_commit("feat(user_settings): add theme option");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope, Some("user_settings".to_string()));
}

#[test]
fn handles_minimal_description() {
    let result = parse_conventional_commit("fix: x");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.description, "x");
}

#[test]
fn trims_description_whitespace() {
    let result = parse_conventional_commit("fix:   lots of spaces   ");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    // Regex captures everything after colon+space, but leading space is in \s*
    assert!(parsed.description.starts_with("lots"));
}

// =============================================================================
// TYPE VALIDATION TESTS
// =============================================================================

#[test]
fn default_types_accepted() {
    for commit_type in DEFAULT_TYPES {
        let msg = format!("{}: test", commit_type);
        let result = parse_conventional_commit(&msg);
        let ParseResult::Conventional(parsed) = result else {
            panic!("expected Conventional for {}", commit_type);
        };
        assert!(
            parsed.is_type_allowed(None),
            "{} should be in default types",
            commit_type
        );
    }
}

#[test]
fn custom_type_rejected_with_defaults() {
    let result = parse_conventional_commit("custom: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert!(!parsed.is_type_allowed(None));
}

#[test]
fn custom_type_accepted_with_empty_list() {
    let result = parse_conventional_commit("custom: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let empty: Vec<String> = vec![];
    assert!(parsed.is_type_allowed(Some(&empty)));
}

#[test]
fn type_checked_against_custom_list() {
    let result = parse_conventional_commit("feat: add feature");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let allowed = vec!["feat".to_string(), "fix".to_string()];
    assert!(parsed.is_type_allowed(Some(&allowed)));

    let not_allowed = vec!["fix".to_string()];
    assert!(!parsed.is_type_allowed(Some(&not_allowed)));
}

// =============================================================================
// SCOPE VALIDATION TESTS
// =============================================================================

#[test]
fn any_scope_allowed_when_not_configured() {
    let result = parse_conventional_commit("feat(random): something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert!(parsed.is_scope_allowed(None));
}

#[test]
fn no_scope_allowed_when_scopes_configured() {
    let result = parse_conventional_commit("feat: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    let scopes = vec!["api".to_string()];
    assert!(parsed.is_scope_allowed(Some(&scopes)));
}

#[test]
fn scope_checked_against_configured_list() {
    let result = parse_conventional_commit("feat(api): add endpoint");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };

    let allowed = vec!["api".to_string(), "cli".to_string()];
    assert!(parsed.is_scope_allowed(Some(&allowed)));

    let not_allowed = vec!["cli".to_string()];
    assert!(!parsed.is_scope_allowed(Some(&not_allowed)));
}

// =============================================================================
// SCOPE_STR HELPER TESTS
// =============================================================================

#[test]
fn scope_str_returns_scope_when_present() {
    let result = parse_conventional_commit("feat(api): something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope_str(), Some("api"));
}

#[test]
fn scope_str_returns_none_when_absent() {
    let result = parse_conventional_commit("feat: something");
    let ParseResult::Conventional(parsed) = result else {
        panic!("expected Conventional");
    };
    assert_eq!(parsed.scope_str(), None);
}
