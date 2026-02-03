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
fn tests_config_defaults() {
    let config = parse_config("version = 1\n");
    assert!(config.check.tests.check.is_none());
    assert!(!config.check.tests.auto);
    assert!(config.check.tests.suite.is_empty());
}

#[test]
fn tests_commit_config_defaults() {
    let commit = TestsCommitConfig::default();
    assert_eq!(commit.check, "off");
    assert_eq!(commit.scope, "branch");
    assert_eq!(commit.placeholders, "allow");
    assert!(commit.exclude.is_empty());
}

#[test]
fn tests_time_config_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.check.tests.time.check, "warn");
}

#[test]
fn tests_coverage_config_defaults() {
    let config = parse_config("version = 1\n");
    assert_eq!(config.check.tests.coverage.check, "warn");
    assert!(config.check.tests.coverage.min.is_none());
    assert!(config.check.tests.coverage.package.is_empty());
}

#[test]
fn test_suite_config_parses() {
    let config = parse_config(
        r#"
version = 1

[[check.tests.suite]]
runner = "cargo"
path = "crates/cli"
ci = true
timeout = "5m"
"#,
    );
    assert_eq!(config.check.tests.suite.len(), 1);
    let suite = &config.check.tests.suite[0];
    assert_eq!(suite.runner, "cargo");
    assert_eq!(suite.path.as_deref(), Some("crates/cli"));
    assert!(suite.ci);
    assert_eq!(suite.timeout, Some(std::time::Duration::from_secs(300)));
}

#[test]
fn test_suite_config_with_custom_runner() {
    let config = parse_config(
        r#"
version = 1

[[check.tests.suite]]
runner = "custom"
name = "my-tests"
command = "make test"
"#,
    );
    let suite = &config.check.tests.suite[0];
    assert_eq!(suite.runner, "custom");
    assert_eq!(suite.name.as_deref(), Some("my-tests"));
    assert_eq!(suite.command.as_deref(), Some("make test"));
}

#[test]
fn tests_coverage_with_per_package() {
    let config = parse_config(
        r#"
version = 1
[check.tests.coverage]
check = "error"
min = 80.0
[check.tests.coverage.package.core]
min = 90.0
"#,
    );
    assert_eq!(config.check.tests.coverage.check, "error");
    assert_eq!(config.check.tests.coverage.min, Some(80.0));
    assert_eq!(config.check.tests.coverage.package["core"].min, 90.0);
}
