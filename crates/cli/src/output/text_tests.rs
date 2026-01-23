// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use termcolor::ColorChoice;

use super::{FormatOptions, TextFormatter};
use crate::check::{CheckResult, Violation};

#[test]
fn text_formatter_creates_successfully() {
    let _formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
}

#[test]
fn text_formatter_silent_on_pass() {
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let result = CheckResult::passed("cloc");
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
}

#[test]
fn text_formatter_tracks_violations_shown() {
    let mut formatter = TextFormatter::new(ColorChoice::Never, FormatOptions::default());
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    formatter.write_check(&result).unwrap();
    assert_eq!(formatter.violations_shown(), 2);
}

#[test]
fn text_formatter_respects_limit() {
    let options = FormatOptions::with_limit(1);
    let mut formatter = TextFormatter::new(ColorChoice::Never, options);
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(truncated);
    assert!(formatter.was_truncated());
    assert_eq!(formatter.violations_shown(), 1);
}

#[test]
fn text_formatter_no_truncation_without_limit() {
    let options = FormatOptions::no_limit();
    let mut formatter = TextFormatter::new(ColorChoice::Never, options);
    let violations = vec![
        Violation::file("src/main.rs", 42, "file_too_large", "Split into modules."),
        Violation::file("src/lib.rs", 100, "file_too_large", "Split into modules."),
    ];
    let result = CheckResult::failed("cloc", violations);
    let truncated = formatter.write_check(&result).unwrap();
    assert!(!truncated);
    assert!(!formatter.was_truncated());
    assert_eq!(formatter.violations_shown(), 2);
}
