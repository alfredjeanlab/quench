// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for stub checks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::check::CheckContext;

#[test]
fn stub_check_name() {
    let check = StubCheck::new("test", "Test check", true);
    assert_eq!(check.name(), "test");
}

#[test]
fn stub_check_description() {
    let check = StubCheck::new("test", "Test check", true);
    assert_eq!(check.description(), "Test check");
}

#[test]
fn stub_check_default_enabled() {
    let enabled = StubCheck::new("test", "Test check", true);
    let disabled = StubCheck::new("test2", "Test check 2", false);
    assert!(enabled.default_enabled());
    assert!(!disabled.default_enabled());
}

#[test]
fn stub_check_result_marked_as_stub() {
    use crate::config::Config;
    use std::path::Path;
    use std::sync::atomic::AtomicUsize;

    let check = StubCheck::new("test", "Test check", true);
    let config = Config::default();
    let files = [];
    let violation_count = AtomicUsize::new(0);
    let ctx = CheckContext {
        root: Path::new("."),
        files: &files,
        config: &config,
        limit: None,
        violation_count: &violation_count,
        changed_files: None,
        fix: false,
        dry_run: false,
        ci_mode: false,
        base_branch: None,
    };

    let result = check.run(&ctx);
    assert!(result.passed, "stub should pass");
    assert!(result.stub, "stub should be marked as stub");
}
