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
fn ruby_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.ruby.source.contains(&"**/*.rb".to_string()));
    assert!(config.ruby.source.contains(&"**/*.rake".to_string()));
    assert!(config.ruby.source.contains(&"Gemfile".to_string()));
    assert!(config.ruby.tests.contains(&"spec/**/*_spec.rb".to_string()));
    assert!(config.ruby.tests.contains(&"test/**/*_test.rb".to_string()));
    assert!(config.ruby.exclude.contains(&"vendor/".to_string()));
}

#[test]
fn ruby_suppress_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.ruby.suppress.check, SuppressLevel::Comment);
    assert_eq!(config.ruby.suppress.test.check, Some(SuppressLevel::Allow));
}

#[test]
fn ruby_policy_defaults() {
    let config = parse_config("version = 1\n");
    assert!(
        config
            .ruby
            .policy
            .lint_config
            .contains(&".rubocop.yml".to_string())
    );
    assert!(
        config
            .ruby
            .policy
            .lint_config
            .contains(&".rubocop_todo.yml".to_string())
    );
}

#[test]
fn ruby_cloc_advice_mentions_modules() {
    let advice = RubyConfig::default_cloc_advice(750);
    assert!(advice.contains("classes or modules"));
}
