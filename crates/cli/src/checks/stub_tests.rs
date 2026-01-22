//! Unit tests for stub checks.

#![allow(clippy::unwrap_used)]

use super::*;

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
