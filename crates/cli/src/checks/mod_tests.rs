// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the check registry.

#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn all_checks_returns_9_checks() {
    let checks = all_checks();
    assert_eq!(checks.len(), 9);
}

#[test]
fn check_names_match_checks() {
    let checks = all_checks();
    for (i, name) in CHECK_NAMES.iter().enumerate() {
        assert_eq!(checks[i].name(), *name);
    }
}

#[test]
fn filter_with_enabled_returns_only_those() {
    let checks = filter_checks(&["cloc".to_string(), "escapes".to_string()], &[]);
    assert_eq!(checks.len(), 2);
    assert!(checks.iter().any(|c| c.name() == "cloc"));
    assert!(checks.iter().any(|c| c.name() == "escapes"));
}

#[test]
fn filter_with_disabled_excludes_those() {
    let checks = filter_checks(&[], &["cloc".to_string()]);
    assert!(!checks.iter().any(|c| c.name() == "cloc"));
    // Should still have other default-enabled checks
    assert!(checks.iter().any(|c| c.name() == "escapes"));
}

#[test]
fn filter_default_runs_all_checks() {
    let checks = filter_checks(&[], &[]);
    // All 9 checks run by default
    assert_eq!(checks.len(), 9);
    assert!(checks.iter().any(|c| c.name() == "git"));
    assert!(checks.iter().any(|c| c.name() == "build"));
    assert!(checks.iter().any(|c| c.name() == "license"));
    assert!(checks.iter().any(|c| c.name() == "placeholders"));
}
