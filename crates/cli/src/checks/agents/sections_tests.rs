// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn validate_finds_missing_required_section() {
    let content = r#"# Project

## Development

Some content.
"#;
    let config = SectionsConfig {
        required: vec![RequiredSection {
            name: "Landing the Plane".to_string(),
            advice: Some("Checklist before work".to_string()),
        }],
        forbid: Vec::new(),
    };

    let result = validate_sections(content, &config);

    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0].name, "Landing the Plane");
    assert_eq!(result.missing[0].advice, Some("Checklist before work".to_string()));
}

#[test]
fn validate_passes_when_required_section_exists() {
    let content = r#"# Project

## Landing the Plane

- [ ] Run tests
"#;
    let config = SectionsConfig {
        required: vec![RequiredSection { name: "Landing the Plane".to_string(), advice: None }],
        forbid: Vec::new(),
    };

    let result = validate_sections(content, &config);

    assert!(result.missing.is_empty());
}

#[test]
fn validate_required_section_case_insensitive() {
    let content = r#"# Project

## LANDING THE PLANE

- [ ] Run tests
"#;
    let config = SectionsConfig {
        required: vec![RequiredSection { name: "Landing the Plane".to_string(), advice: None }],
        forbid: Vec::new(),
    };

    let result = validate_sections(content, &config);

    assert!(result.missing.is_empty());
}

#[test]
fn validate_finds_forbidden_section() {
    let content = r#"# Project

## Secrets

DO NOT put secrets here!
"#;
    let config = SectionsConfig { required: Vec::new(), forbid: vec!["Secrets".to_string()] };

    let result = validate_sections(content, &config);

    assert_eq!(result.forbidden.len(), 1);
    assert_eq!(result.forbidden[0].heading, "Secrets");
    assert_eq!(result.forbidden[0].matched_pattern, "Secrets");
}

#[test]
fn validate_forbidden_section_case_insensitive() {
    let content = r#"# Project

## SECRETS

DO NOT put secrets here!
"#;
    let config = SectionsConfig { required: Vec::new(), forbid: vec!["Secrets".to_string()] };

    let result = validate_sections(content, &config);

    assert_eq!(result.forbidden.len(), 1);
}

#[test]
fn validate_forbidden_glob_star() {
    let content = r#"# Project

## Testing Plan

This is a test plan.
"#;
    let config = SectionsConfig { required: Vec::new(), forbid: vec!["Test*".to_string()] };

    let result = validate_sections(content, &config);

    assert_eq!(result.forbidden.len(), 1);
    assert_eq!(result.forbidden[0].heading, "Testing Plan");
    assert_eq!(result.forbidden[0].matched_pattern, "Test*");
}

#[test]
fn validate_forbidden_glob_question() {
    let content = r#"# Project

## API Key

The API key.
"#;
    let config = SectionsConfig { required: Vec::new(), forbid: vec!["API?Key".to_string()] };

    let result = validate_sections(content, &config);

    assert_eq!(result.forbidden.len(), 1);
}

#[test]
fn validate_glob_no_match() {
    let content = r#"# Project

## Development

Some content.
"#;
    let config = SectionsConfig { required: Vec::new(), forbid: vec!["Test*".to_string()] };

    let result = validate_sections(content, &config);

    assert!(result.forbidden.is_empty());
}

// Glob pattern unit tests
mod glob {
    use super::super::glob_match;

    #[test]
    fn exact_match() {
        assert!(glob_match("test", "test"));
        assert!(!glob_match("test", "testing"));
    }

    #[test]
    fn trailing_star() {
        assert!(glob_match("test*", "test"));
        assert!(glob_match("test*", "testing"));
        assert!(glob_match("test*", "test plan"));
        assert!(!glob_match("test*", "atest"));
    }

    #[test]
    fn leading_star() {
        assert!(glob_match("*test", "test"));
        assert!(glob_match("*test", "mytest"));
        assert!(!glob_match("*test", "testing"));
    }

    #[test]
    fn middle_star() {
        assert!(glob_match("te*st", "test"));
        assert!(glob_match("te*st", "teXXst"));
        assert!(!glob_match("te*st", "testing"));
    }

    #[test]
    fn question_mark() {
        assert!(glob_match("te?t", "test"));
        assert!(glob_match("te?t", "text"));
        assert!(!glob_match("te?t", "tet"));
        assert!(!glob_match("te?t", "teest"));
    }

    #[test]
    fn complex_pattern() {
        assert!(glob_match("api*key*", "api key storage"));
        assert!(glob_match("a?c*e", "abcde"));
        assert!(glob_match("a?c*e", "axce"));
    }
}
