#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::*;

// =============================================================================
// FRONTMATTER PARSING TESTS
// =============================================================================

#[test]
fn parse_always_apply_rule() {
    let content = "---\ndescription: \"General coding standards\"\nalwaysApply: true\n---\n\n## Code Style\n\nUse 4 spaces.\n";
    let rule = parse_mdc(content, PathBuf::from("general.mdc")).unwrap();

    assert!(rule.always_apply);
    assert_eq!(
        rule.description.as_deref(),
        Some("General coding standards")
    );
    assert!(rule.globs.is_none());
    assert!(rule.body.contains("## Code Style"));
    assert!(rule.body.contains("Use 4 spaces."));
}

#[test]
fn parse_glob_scoped_rule() {
    let content = "---\ndescription: \"API rules\"\nglobs: \"src/api/**\"\nalwaysApply: false\n---\n\n## API Conventions\n\nUse REST.\n";
    let rule = parse_mdc(content, PathBuf::from("api.mdc")).unwrap();

    assert!(!rule.always_apply);
    assert_eq!(rule.globs.as_deref(), Some(&["src/api/**".to_string()][..]));
    assert!(rule.body.contains("## API Conventions"));
}

#[test]
fn parse_glob_array() {
    let content = "---\nglobs: [\"src/**\", \"lib/**\"]\nalwaysApply: false\n---\n\nBody.\n";
    let rule = parse_mdc(content, PathBuf::from("multi.mdc")).unwrap();

    let globs = rule.globs.unwrap();
    assert_eq!(globs.len(), 2);
    assert_eq!(globs[0], "src/**");
    assert_eq!(globs[1], "lib/**");
}

#[test]
fn parse_no_frontmatter() {
    let content = "## Just Markdown\n\nNo frontmatter here.\n";
    let rule = parse_mdc(content, PathBuf::from("plain.mdc")).unwrap();

    assert!(!rule.always_apply);
    assert!(rule.globs.is_none());
    assert!(rule.description.is_none());
    assert_eq!(rule.body, content);
}

#[test]
fn parse_empty_body() {
    let content = "---\nalwaysApply: true\n---\n";
    let rule = parse_mdc(content, PathBuf::from("empty.mdc")).unwrap();

    assert!(rule.always_apply);
    assert!(rule.body.is_empty());
}

#[test]
fn parse_unquoted_description() {
    let content = "---\ndescription: API standards\nalwaysApply: false\n---\n\nBody.\n";
    let rule = parse_mdc(content, PathBuf::from("unquoted.mdc")).unwrap();

    assert_eq!(rule.description.as_deref(), Some("API standards"));
}

#[test]
fn parse_single_quoted_values() {
    let content =
        "---\ndescription: 'My rules'\nglobs: 'src/**'\nalwaysApply: false\n---\n\nBody.\n";
    let rule = parse_mdc(content, PathBuf::from("single-quoted.mdc")).unwrap();

    assert_eq!(rule.description.as_deref(), Some("My rules"));
    assert_eq!(rule.globs.as_deref(), Some(&["src/**".to_string()][..]));
}

#[test]
fn parse_unterminated_frontmatter() {
    let content = "---\nalwaysApply: true\nNo closing delimiter.\n";
    let result = parse_mdc(content, PathBuf::from("bad.mdc"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("unterminated"));
}

#[test]
fn parse_unknown_keys_ignored() {
    let content = "---\nalwaysApply: true\ncustomKey: value\n---\n\nBody.\n";
    let rule = parse_mdc(content, PathBuf::from("extra.mdc")).unwrap();

    assert!(rule.always_apply);
}

#[test]
fn parse_empty_globs() {
    let content = "---\nglobs: \nalwaysApply: false\n---\n\nBody.\n";
    let rule = parse_mdc(content, PathBuf::from("empty-globs.mdc")).unwrap();

    // Empty globs should parse to Some(empty vec)
    assert_eq!(rule.globs, Some(vec![]));
}

#[test]
fn parse_body_preserves_content() {
    let body = "## Section One\n\nContent one.\n\n## Section Two\n\nContent two.";
    let content = format!("---\nalwaysApply: true\n---\n\n{body}\n");
    let rule = parse_mdc(&content, PathBuf::from("body.mdc")).unwrap();

    assert!(rule.body.contains("## Section One"));
    assert!(rule.body.contains("## Section Two"));
    assert!(rule.body.contains("Content one."));
    assert!(rule.body.contains("Content two."));
}

// =============================================================================
// SCOPE CLASSIFICATION TESTS
// =============================================================================

#[test]
fn classify_always_apply() {
    let rule = MdcRule {
        description: None,
        globs: None,
        always_apply: true,
        body: String::new(),
        path: PathBuf::from("general.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::AlwaysApply);
}

#[test]
fn classify_single_directory_double_star() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/api/**".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("api.mdc"),
    };

    assert_eq!(
        classify_scope(&rule),
        RuleScope::SingleDirectory(PathBuf::from("src/api"))
    );
}

#[test]
fn classify_single_directory_star() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/api/*".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("api.mdc"),
    };

    assert_eq!(
        classify_scope(&rule),
        RuleScope::SingleDirectory(PathBuf::from("src/api"))
    );
}

#[test]
fn classify_single_directory_double_star_star() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/api/**/*".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("api.mdc"),
    };

    assert_eq!(
        classify_scope(&rule),
        RuleScope::SingleDirectory(PathBuf::from("src/api"))
    );
}

#[test]
fn classify_deeply_nested_directory() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/components/ui/**".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("ui.mdc"),
    };

    assert_eq!(
        classify_scope(&rule),
        RuleScope::SingleDirectory(PathBuf::from("src/components/ui"))
    );
}

#[test]
fn classify_file_pattern_with_extension() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/**/*.tsx".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("tsx.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::FilePattern);
}

#[test]
fn classify_multiple_globs() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/**".to_string(), "lib/**".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("multi.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::FilePattern);
}

#[test]
fn classify_on_demand_no_globs() {
    let rule = MdcRule {
        description: Some("Use when needed".to_string()),
        globs: None,
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("on-demand.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::OnDemand);
}

#[test]
fn classify_on_demand_empty_globs() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec![]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("empty.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::OnDemand);
}

#[test]
fn classify_glob_with_wildcard_in_prefix() {
    let rule = MdcRule {
        description: None,
        globs: Some(vec!["src/*/api/**".to_string()]),
        always_apply: false,
        body: String::new(),
        path: PathBuf::from("wild-prefix.mdc"),
    };

    assert_eq!(classify_scope(&rule), RuleScope::FilePattern);
}

// =============================================================================
// HEADER STRIPPING TESTS
// =============================================================================

#[test]
fn strip_header_removes_h1() {
    let content = "# My Rule\n\n## Section\n\nContent.\n";
    let stripped = strip_leading_header(content);

    assert!(!stripped.contains("# My Rule"));
    assert!(stripped.contains("## Section"));
}

#[test]
fn strip_header_no_header() {
    let content = "## Section\n\nContent.\n";
    let stripped = strip_leading_header(content);

    assert_eq!(stripped, content);
}

#[test]
fn strip_header_empty() {
    assert_eq!(strip_leading_header(""), "");
}

#[test]
fn strip_header_only_header() {
    let content = "# Just a Title";
    let stripped = strip_leading_header(content);

    assert_eq!(stripped, "");
}

// =============================================================================
// HELPER FUNCTION TESTS
// =============================================================================

#[test]
fn unquote_double_quotes() {
    assert_eq!(unquote("\"hello\""), "hello");
}

#[test]
fn unquote_single_quotes() {
    assert_eq!(unquote("'hello'"), "hello");
}

#[test]
fn unquote_no_quotes() {
    assert_eq!(unquote("hello"), "hello");
}

#[test]
fn parse_globs_single() {
    assert_eq!(parse_globs("\"src/**\""), vec!["src/**"]);
}

#[test]
fn parse_globs_array() {
    assert_eq!(
        parse_globs("[\"src/**\", \"lib/**\"]"),
        vec!["src/**", "lib/**"]
    );
}

#[test]
fn parse_globs_unquoted() {
    assert_eq!(parse_globs("src/**"), vec!["src/**"]);
}

#[test]
fn parse_globs_empty() {
    let result: Vec<String> = parse_globs("");
    assert!(result.is_empty());
}
