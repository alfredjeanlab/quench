// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::path::PathBuf;

fn parse_config(content: &str) -> Config {
    let path = PathBuf::from("quench.toml");
    parse(content, &path).unwrap()
}

#[test]
fn go_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.golang.source.contains(&"**/*.go".to_string()));
    assert!(config.golang.tests.contains(&"**/*_test.go".to_string()));
    assert!(config.golang.exclude.contains(&"vendor/**".to_string()));
    assert!(config.golang.cloc.is_none());
}

#[test]
fn go_config_custom_patterns() {
    let config = parse_config(
        r#"
version = 1
[golang]
source = ["cmd/**/*.go"]
tests = ["**/*_integration_test.go"]
exclude = ["vendor/**", "third_party/**"]
"#,
    );
    assert_eq!(config.golang.source, vec!["cmd/**/*.go"]);
    assert_eq!(config.golang.tests, vec!["**/*_integration_test.go"]);
    assert_eq!(config.golang.exclude, vec!["vendor/**", "third_party/**"]);
}

#[test]
fn go_suppress_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.golang.suppress.check, SuppressLevel::Comment);
    assert!(config.golang.suppress.comment.is_none());
    assert_eq!(
        config.golang.suppress.test.check,
        Some(SuppressLevel::Allow)
    );
}

#[test]
fn go_policy_config_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.golang.policy.lint_changes, LintChangesPolicy::None);
    assert!(
        config
            .golang
            .policy
            .lint_config
            .contains(&".golangci.yml".to_string())
    );
}

#[test]
fn go_cloc_advice_contains_package() {
    let advice = GoConfig::default_cloc_advice(750);
    assert!(advice.contains("package"));
    assert!(advice.contains("150–250 lines"));
}
