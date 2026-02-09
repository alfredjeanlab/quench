// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn metrics_to_json_structure() {
    let metrics = PlaceholderMetrics {
        rust: RustMetrics { ignore: 2, todo: 1 },
        javascript: JsMetrics { todo: 3, fixme: 1, skip: 0 },
    };

    let json = metrics.to_json();

    assert_eq!(json["rust"]["ignore"], 2);
    assert_eq!(json["rust"]["todo"], 1);
    assert_eq!(json["javascript"]["todo"], 3);
    assert_eq!(json["javascript"]["fixme"], 1);
    assert_eq!(json["javascript"]["skip"], 0);
}

#[test]
fn metrics_has_placeholders_true_when_present() {
    let metrics = PlaceholderMetrics {
        rust: RustMetrics { ignore: 1, todo: 0 },
        javascript: JsMetrics::default(),
    };
    assert!(metrics.has_placeholders());

    let metrics = PlaceholderMetrics {
        rust: RustMetrics::default(),
        javascript: JsMetrics { todo: 1, fixme: 0, skip: 0 },
    };
    assert!(metrics.has_placeholders());
}

#[test]
fn metrics_has_placeholders_false_when_empty() {
    let metrics = PlaceholderMetrics::default();
    assert!(!metrics.has_placeholders());
}

#[test]
fn default_rust_patterns_includes_common() {
    let patterns = default_rust_patterns();
    assert!(patterns.contains(&"ignore".to_string()));
    assert!(patterns.contains(&"todo".to_string()));
}

#[test]
fn default_js_patterns_includes_common() {
    let patterns = default_js_patterns();
    assert!(patterns.contains(&"todo".to_string()));
    assert!(patterns.contains(&"fixme".to_string()));
    assert!(patterns.contains(&"skip".to_string()));
}
