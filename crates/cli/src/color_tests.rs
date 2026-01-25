// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use termcolor::Color;

// NOTE: Environment variable tests for NO_COLOR and COLOR are in
// tests/specs/output/format.rs and tests/specs/config/env.rs
// because env var manipulation is not safe in parallel unit tests.
//
// The resolve_color() function behavior is:
// - NO_COLOR set -> ColorChoice::Never
// - COLOR set -> ColorChoice::Always
// - Neither -> auto-detect based on TTY and agent environment

#[test]
fn scheme_check_name_is_bold() {
    let spec = scheme::check_name();
    assert!(spec.bold());
}

#[test]
fn scheme_fail_is_red_bold() {
    let spec = scheme::fail();
    assert_eq!(spec.fg(), Some(&Color::Red));
    assert!(spec.bold());
}

#[test]
fn scheme_pass_is_green_bold() {
    let spec = scheme::pass();
    assert_eq!(spec.fg(), Some(&Color::Green));
    assert!(spec.bold());
}

#[test]
fn scheme_path_is_cyan() {
    let spec = scheme::path();
    assert_eq!(spec.fg(), Some(&Color::Cyan));
}

#[test]
fn scheme_line_number_is_yellow() {
    let spec = scheme::line_number();
    assert_eq!(spec.fg(), Some(&Color::Yellow));
}

#[test]
fn scheme_advice_has_no_color() {
    let spec = scheme::advice();
    assert!(spec.fg().is_none());
    assert!(!spec.bold());
}
