// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::*;

#[test]
fn new_baseline_has_current_version() {
    let baseline = Baseline::new();
    assert_eq!(baseline.version, BASELINE_VERSION);
}

#[test]
fn new_baseline_has_empty_metrics() {
    let baseline = Baseline::new();
    assert!(baseline.metrics.escapes.is_none());
    assert!(baseline.metrics.coverage.is_none());
    assert!(baseline.metrics.binary_size.is_none());
}

#[test]
fn load_nonexistent_returns_none() {
    let path = std::path::Path::new("/nonexistent/baseline.json");
    let result = Baseline::load(path).unwrap();
    assert!(result.is_none());
}

#[test]
fn save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".quench/baseline.json");

    let mut baseline = Baseline::new();
    baseline.metrics.escapes = Some(EscapesMetrics {
        source: HashMap::from([("unsafe".to_string(), 5)]),
        test: Some(HashMap::from([("unsafe".to_string(), 10)])),
    });

    baseline.save(&path).unwrap();
    let loaded = Baseline::load(&path).unwrap().unwrap();

    assert_eq!(loaded.version, baseline.version);
    assert!(loaded.metrics.escapes.is_some());
    let escapes = loaded.metrics.escapes.unwrap();
    assert_eq!(escapes.source.get("unsafe"), Some(&5));
    assert_eq!(
        escapes.test.as_ref().and_then(|t| t.get("unsafe")),
        Some(&10)
    );
}

#[test]
fn version_too_new_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("baseline.json");

    // Write a baseline with version 999
    let content = r#"{
        "version": 999,
        "updated": "2026-01-20T00:00:00Z",
        "metrics": {}
    }"#;
    std::fs::write(&path, content).unwrap();

    let result = Baseline::load(&path);
    assert!(matches!(
        result,
        Err(BaselineError::Version { found: 999, .. })
    ));
}

#[test]
fn parse_invalid_json_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("baseline.json");

    std::fs::write(&path, "not json").unwrap();

    let result = Baseline::load(&path);
    assert!(matches!(result, Err(BaselineError::Parse(_))));
}

#[test]
fn creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deeply/nested/.quench/baseline.json");

    let baseline = Baseline::new();
    baseline.save(&path).unwrap();

    assert!(path.exists());
}

#[test]
fn serializes_escapes_metrics() {
    let metrics = EscapesMetrics {
        source: HashMap::from([("unsafe".to_string(), 3), ("unwrap".to_string(), 7)]),
        test: Some(HashMap::from([("unsafe".to_string(), 15)])),
    };

    let json = serde_json::to_string(&metrics).unwrap();
    let parsed: EscapesMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.source.get("unsafe"), Some(&3));
    assert_eq!(parsed.source.get("unwrap"), Some(&7));
}

#[test]
fn touch_updates_timestamp() {
    let mut baseline = Baseline::new();
    let original = baseline.updated;

    // Sleep briefly to ensure time difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    baseline.touch();
    assert!(baseline.updated > original);
}
