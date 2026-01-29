// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use tempfile::tempdir;

fn make_config(command: Option<&str>) -> TestSuiteConfig {
    TestSuiteConfig {
        runner: "custom".to_string(),
        name: None,
        path: None,
        setup: None,
        command: command.map(String::from),
        targets: vec![],
        ci: false,
        max_total: None,
        max_avg: None,
        max_test: None,
        timeout: None,
    }
}

fn make_ctx<'a>(root: &'a std::path::Path, config: &'a crate::config::Config) -> RunnerContext<'a> {
    RunnerContext {
        root,
        ci_mode: false,
        collect_coverage: false,
        config,
        verbose: false,
    }
}

#[test]
fn runner_is_always_available() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    assert!(runner.available(&ctx));
}

#[test]
fn fails_without_command() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    let config = make_config(None);

    let result = runner.run(&config, &ctx);
    assert!(!result.passed);
    assert!(result.error.unwrap().contains("requires 'command' field"));
}

#[test]
fn passes_on_success() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    let config = make_config(Some("true"));

    let result = runner.run(&config, &ctx);
    assert!(result.passed);
    assert!(result.error.is_none());
}

#[test]
fn fails_on_nonzero_exit() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    let config = make_config(Some("exit 1"));

    let result = runner.run(&config, &ctx);
    assert!(!result.passed);
}

#[test]
fn captures_stderr_on_failure() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    let config = make_config(Some("echo 'error message' >&2 && exit 1"));

    let result = runner.run(&config, &ctx);
    assert!(!result.passed);
    assert!(result.error.unwrap().contains("error message"));
}

#[test]
fn no_per_test_timing() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    let config = make_config(Some("echo test"));

    let result = runner.run(&config, &ctx);
    // Custom runner doesn't provide per-test timing
    assert!(result.tests.is_empty());
}

#[test]
fn runs_complex_command() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    // Test that complex shell commands work
    let config = make_config(Some("echo hello && echo world"));

    let result = runner.run(&config, &ctx);
    assert!(result.passed);
}

#[test]
fn runs_setup_before_command() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);

    // Create a config with setup that creates a file, then command checks for it
    let mut config = make_config(Some("test -f marker.txt"));
    config.setup = Some("touch marker.txt".to_string());

    let result = runner.run(&config, &ctx);
    assert!(result.passed);
}

#[test]
fn fails_if_setup_fails() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);

    let mut config = make_config(Some("true"));
    config.setup = Some("exit 1".to_string());

    let result = runner.run(&config, &ctx);
    assert!(!result.passed);
    assert!(result.error.unwrap().contains("setup command failed"));
}

#[test]
fn reports_exit_code_when_no_stderr() {
    let runner = CustomRunner;
    let temp = tempdir().unwrap();
    let project_config = crate::config::Config::default();
    let ctx = make_ctx(temp.path(), &project_config);
    // Command that fails without stderr output
    let config = make_config(Some("exit 42"));

    let result = runner.run(&config, &ctx);
    assert!(!result.passed);
    // Should mention exit code since there's no stderr
    let error = result.error.unwrap();
    assert!(error.contains("exit code") || error.contains("42"));
}
