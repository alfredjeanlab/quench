// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use tempfile::TempDir;

#[test]
fn save_and_load_latest_metrics() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".quench/latest.json");

    let output =
        CheckOutput { timestamp: "2026-01-27T00:00:00Z".to_string(), passed: true, checks: vec![] };

    let latest =
        LatestMetrics { updated: chrono::Utc::now(), commit: Some("abc1234".to_string()), output };

    latest.save(&path).unwrap();

    let loaded = LatestMetrics::load(&path).unwrap().unwrap();
    assert_eq!(loaded.commit, Some("abc1234".to_string()));
    assert!(loaded.output.passed);
}

#[test]
fn load_returns_none_for_missing_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("nonexistent.json");

    let result = LatestMetrics::load(&path).unwrap();
    assert!(result.is_none());
}

#[test]
fn save_creates_parent_directories() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("nested/dir/.quench/latest.json");

    let output =
        CheckOutput { timestamp: "2026-01-27T00:00:00Z".to_string(), passed: true, checks: vec![] };

    let latest = LatestMetrics { updated: chrono::Utc::now(), commit: None, output };

    latest.save(&path).unwrap();
    assert!(path.exists());
}
