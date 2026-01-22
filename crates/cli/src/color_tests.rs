#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use termcolor::Color;

#[test]
fn color_mode_always_resolves_to_always() {
    assert_eq!(ColorMode::Always.resolve(), ColorChoice::Always);
}

#[test]
fn color_mode_never_resolves_to_never() {
    assert_eq!(ColorMode::Never.resolve(), ColorChoice::Never);
}

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
