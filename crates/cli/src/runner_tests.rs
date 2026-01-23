// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the check runner.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use crate::check::{Check, CheckContext, CheckResult, Violation};

/// Mock check that can be configured to pass, fail, or panic.
struct MockCheck {
    name: &'static str,
    behavior: MockBehavior,
    ran: AtomicBool,
}

enum MockBehavior {
    Pass,
    Fail(usize), // Number of violations
    Panic,
    #[allow(dead_code)] // KEEP UNTIL: Phase 050 tests skip behavior
    Skip(String),
}

impl MockCheck {
    fn new(name: &'static str, behavior: MockBehavior) -> Self {
        Self {
            name,
            behavior,
            ran: AtomicBool::new(false),
        }
    }

    fn did_run(&self) -> bool {
        self.ran.load(Ordering::SeqCst)
    }
}

impl Check for MockCheck {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        "Mock check"
    }

    fn run(&self, _ctx: &CheckContext) -> CheckResult {
        self.ran.store(true, Ordering::SeqCst);

        match &self.behavior {
            MockBehavior::Pass => CheckResult::passed(self.name),
            MockBehavior::Fail(count) => {
                let violations: Vec<_> = (0..*count)
                    .map(|i| {
                        Violation::file_only(format!("file{}.rs", i), "test_violation", "Fix this")
                    })
                    .collect();
                CheckResult::failed(self.name, violations)
            }
            MockBehavior::Panic => panic!("Mock check panicked"),
            MockBehavior::Skip(msg) => CheckResult::skipped(self.name, msg.clone()),
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

#[test]
fn runner_executes_all_checks() {
    let runner = CheckRunner::new(RunnerConfig {
        limit: None,
        changed_files: None,
        fix: false,
        dry_run: false,
    });
    let config = Config::default();
    let files = vec![];
    let root = std::path::Path::new(".");

    let checks: Vec<Arc<dyn Check>> = vec![
        Arc::new(MockCheck::new("check1", MockBehavior::Pass)),
        Arc::new(MockCheck::new("check2", MockBehavior::Fail(1))),
        Arc::new(MockCheck::new("check3", MockBehavior::Pass)),
    ];

    let results = runner.run(checks, &files, &config, root);

    assert_eq!(results.len(), 3, "all checks should have results");
}

#[test]
fn runner_isolates_panicking_check() {
    let runner = CheckRunner::new(RunnerConfig {
        limit: None,
        changed_files: None,
        fix: false,
        dry_run: false,
    });
    let config = Config::default();
    let files = vec![];
    let root = std::path::Path::new(".");

    let passing = Arc::new(MockCheck::new("passing", MockBehavior::Pass));
    let panicking = Arc::new(MockCheck::new("panicking", MockBehavior::Panic));

    let checks: Vec<Arc<dyn Check>> = vec![passing.clone(), panicking.clone()];

    let results = runner.run(checks, &files, &config, root);

    // Both checks should have results
    assert_eq!(results.len(), 2);

    // Passing check should have run and passed
    let pass_result = results.iter().find(|r| r.name == "passing").unwrap();
    assert!(pass_result.passed);

    // Panicking check should be skipped with error
    let panic_result = results.iter().find(|r| r.name == "panicking").unwrap();
    assert!(panic_result.skipped);
    assert!(panic_result.error.is_some());
}

#[test]
fn runner_continues_after_check_failure() {
    let runner = CheckRunner::new(RunnerConfig {
        limit: None,
        changed_files: None,
        fix: false,
        dry_run: false,
    });
    let config = Config::default();
    let files = vec![];
    let root = std::path::Path::new(".");

    let check1 = Arc::new(MockCheck::new("check1", MockBehavior::Fail(5)));
    let check2 = Arc::new(MockCheck::new("check2", MockBehavior::Pass));

    let checks: Vec<Arc<dyn Check>> = vec![check1.clone(), check2.clone()];

    let results = runner.run(checks, &files, &config, root);

    // Both checks should run
    assert!(check1.did_run());
    assert!(check2.did_run());

    // First failed, second passed
    let result1 = results.iter().find(|r| r.name == "check1").unwrap();
    let result2 = results.iter().find(|r| r.name == "check2").unwrap();
    assert!(!result1.passed);
    assert!(result2.passed);
}

#[test]
fn should_terminate_with_limit() {
    let runner = CheckRunner::new(RunnerConfig {
        limit: Some(10),
        changed_files: None,
        fix: false,
        dry_run: false,
    });
    assert!(!runner.should_terminate(5));
    assert!(runner.should_terminate(10));
    assert!(runner.should_terminate(15));
}

#[test]
fn should_terminate_without_limit() {
    let runner = CheckRunner::new(RunnerConfig {
        limit: None,
        changed_files: None,
        fix: false,
        dry_run: false,
    });
    assert!(!runner.should_terminate(1000));
}
