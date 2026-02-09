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
fn python_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.python.source.contains(&"**/*.py".to_string()));
    assert!(config.python.tests.contains(&"tests/**/*.py".to_string()));
    assert!(config.python.tests.contains(&"**/conftest.py".to_string()));
    assert!(config.python.exclude.contains(&".venv/**".to_string()));
    assert!(config.python.exclude.contains(&"__pycache__/**".to_string()));
}

#[test]
fn python_suppress_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.python.suppress.check, SuppressLevel::Comment);
    assert_eq!(config.python.suppress.test.check, Some(SuppressLevel::Allow));
}

#[test]
fn python_policy_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.python.policy.lint_config.contains(&"ruff.toml".to_string()));
    assert!(config.python.policy.lint_config.contains(&"pyproject.toml".to_string()));
    assert!(config.python.policy.lint_config.contains(&".flake8".to_string()));
    assert!(config.python.policy.lint_config.contains(&"mypy.ini".to_string()));
}

#[test]
fn python_cloc_advice_mentions_packages() {
    let advice = PythonConfig::default_cloc_advice(750);
    assert!(advice.contains("__init__.py"));
    assert!(advice.contains("150–250 lines"));
}
