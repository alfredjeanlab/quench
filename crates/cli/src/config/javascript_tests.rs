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
fn javascript_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.javascript.source.contains(&"**/*.js".to_string()));
    assert!(config.javascript.source.contains(&"**/*.ts".to_string()));
    assert!(config.javascript.source.contains(&"**/*.tsx".to_string()));
    assert!(config.javascript.tests.contains(&"**/*.test.*".to_string()));
    assert!(config.javascript.tests.contains(&"**/*.spec.*".to_string()));
    assert!(config.javascript.exclude.contains(&"node_modules/**".to_string()));
    assert!(config.javascript.exclude.contains(&"dist/**".to_string()));
}

#[test]
fn javascript_config_custom() {
    let config = parse_config(
        r#"
version = 1
[javascript]
source = ["src/**/*.ts"]
tests = ["**/*.test.ts"]
exclude = ["node_modules/**"]
"#,
    );
    assert_eq!(config.javascript.source, vec!["src/**/*.ts"]);
    assert_eq!(config.javascript.tests, vec!["**/*.test.ts"]);
    assert_eq!(config.javascript.exclude, vec!["node_modules/**"]);
}

#[test]
fn javascript_suppress_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.javascript.suppress.check, SuppressLevel::Comment);
    assert_eq!(config.javascript.suppress.test.check, Some(SuppressLevel::Allow));
}

#[test]
fn javascript_policy_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.javascript.policy.lint_config.contains(&"eslint.config.js".to_string()));
    assert!(config.javascript.policy.lint_config.contains(&"biome.json".to_string()));
    assert!(config.javascript.policy.lint_config.contains(&"tsconfig.json".to_string()));
}
