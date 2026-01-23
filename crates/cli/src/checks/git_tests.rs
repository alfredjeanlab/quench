// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the git check.

#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn git_check_name() {
    let check = GitCheck;
    assert_eq!(check.name(), "git");
}

#[test]
fn git_check_description() {
    let check = GitCheck;
    assert_eq!(check.description(), "Commit message format");
}

#[test]
fn git_check_default_disabled() {
    let check = GitCheck;
    assert!(!check.default_enabled());
}
