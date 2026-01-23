#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use termcolor::Color;

#[test]
fn resolve_color_force_returns_always() {
    assert_eq!(resolve_color(true, false), ColorChoice::Always);
}

#[test]
fn resolve_color_no_color_returns_never() {
    assert_eq!(resolve_color(false, true), ColorChoice::Never);
}

#[test]
fn resolve_color_no_color_takes_priority_over_force() {
    // no_color wins even when force_color is also set
    assert_eq!(resolve_color(true, true), ColorChoice::Never);
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
