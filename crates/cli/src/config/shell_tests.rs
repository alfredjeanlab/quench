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
fn shell_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.shell.source.contains(&"**/*.sh".to_string()));
    assert!(config.shell.source.contains(&"**/*.bash".to_string()));
    assert!(config.shell.tests.contains(&"**/tests/**/*.bats".to_string()));
    assert!(config.shell.exclude.is_empty());
}

#[test]
fn shell_suppress_defaults_to_forbid() {
    let config = parse_config("version = 1\n");
    // Shell defaults to "forbid" unlike other languages
    assert_eq!(config.shell.suppress.check, SuppressLevel::Forbid);
    assert_eq!(config.shell.suppress.test.check, Some(SuppressLevel::Allow));
}

#[test]
fn shell_policy_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.shell.policy.lint_config.contains(&".shellcheckrc".to_string()));
}

#[test]
fn shell_cloc_advice_mentions_scripts() {
    let advice = ShellConfig::default_cloc_advice(750);
    assert!(advice.contains("scripts"));
}
