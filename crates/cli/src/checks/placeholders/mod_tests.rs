// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn check_name_is_placeholders() {
    let check = PlaceholdersCheck;
    assert_eq!(check.name(), "placeholders");
}

#[test]
fn check_disabled_by_default() {
    let check = PlaceholdersCheck;
    assert!(!check.default_enabled());
}

#[test]
fn metrics_to_json_structure() {
    let metrics = Metrics {
        rust_ignore: 2,
        rust_todo: 1,
        js_todo: 3,
        js_fixme: 1,
    };

    let json = metrics.to_json();

    assert_eq!(json["rust"]["ignore"], 2);
    assert_eq!(json["rust"]["todo"], 1);
    assert_eq!(json["javascript"]["todo"], 3);
    assert_eq!(json["javascript"]["fixme"], 1);
}

#[test]
fn metrics_increment_rust() {
    let mut metrics = Metrics::default();

    metrics.increment_rust(rust::RustPlaceholderKind::Ignore);
    metrics.increment_rust(rust::RustPlaceholderKind::Ignore);
    metrics.increment_rust(rust::RustPlaceholderKind::Todo);

    assert_eq!(metrics.rust_ignore, 2);
    assert_eq!(metrics.rust_todo, 1);
}

#[test]
fn metrics_increment_js() {
    let mut metrics = Metrics::default();

    metrics.increment_js(javascript::JsPlaceholderKind::Todo);
    metrics.increment_js(javascript::JsPlaceholderKind::Todo);
    metrics.increment_js(javascript::JsPlaceholderKind::Fixme);
    metrics.increment_js(javascript::JsPlaceholderKind::Skip); // Not counted

    assert_eq!(metrics.js_todo, 2);
    assert_eq!(metrics.js_fixme, 1);
}

#[test]
fn default_test_patterns_are_common() {
    let patterns = default_test_patterns();

    // Should include common test directories and file patterns
    assert!(patterns.iter().any(|p| p.contains("tests")));
    assert!(patterns.iter().any(|p| p.contains("_test.")));
    assert!(patterns.iter().any(|p| p.contains(".spec.")));
}
