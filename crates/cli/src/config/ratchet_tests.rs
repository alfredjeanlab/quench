// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::path::PathBuf;
use std::time::Duration;

fn parse_config(content: &str) -> Config {
    let path = PathBuf::from("quench.toml");
    parse(content, &path).unwrap()
}

#[test]
fn ratchet_config_defaults_without_section() {
    // When [ratchet] is absent, the whole struct uses Default (bool = false)
    let config = parse_config("version = 1\n");
    assert_eq!(config.ratchet.check, CheckLevel::Error);
    assert!(!config.ratchet.coverage);
    assert!(!config.ratchet.escapes);
    assert!(config.ratchet.package.is_empty());
}

#[test]
fn ratchet_config_defaults_with_section() {
    // When [ratchet] is present (even empty), serde uses default_true for coverage/escapes
    let config = parse_config("version = 1\n[ratchet]\n");
    assert_eq!(config.ratchet.check, CheckLevel::Error);
    assert!(config.ratchet.coverage);
    assert!(config.ratchet.escapes);
    assert!(!config.ratchet.binary_size);
    assert!(!config.ratchet.build_time_cold);
    assert!(!config.ratchet.build_time_hot);
    assert!(!config.ratchet.test_time_total);
    assert!(!config.ratchet.test_time_avg);
    assert!(!config.ratchet.test_time_max);
    assert_eq!(config.ratchet.stale_days, 30);
    assert!(config.ratchet.package.is_empty());
}

#[test]
fn ratchet_config_custom() {
    let config = parse_config(
        r#"
version = 1
[ratchet]
check = "warn"
coverage = false
binary_size = true
stale_days = 7
"#,
    );
    assert_eq!(config.ratchet.check, CheckLevel::Warn);
    assert!(!config.ratchet.coverage);
    assert!(config.ratchet.binary_size);
    assert_eq!(config.ratchet.stale_days, 7);
}

#[test]
fn ratchet_tolerance_parsing() {
    let config = parse_config(
        r#"
version = 1
[ratchet]
coverage_tolerance = 2.5
binary_size_tolerance = "100KB"
build_time_tolerance = "5s"
test_time_tolerance = "2s"
"#,
    );
    assert_eq!(config.ratchet.coverage_tolerance_pct(), Some(2.5));
    assert_eq!(
        config.ratchet.binary_size_tolerance_bytes(),
        Some(100 * 1024)
    );
    assert_eq!(
        config.ratchet.build_time_tolerance_duration(),
        Some(Duration::from_secs(5))
    );
    assert_eq!(
        config.ratchet.test_time_tolerance_duration(),
        Some(Duration::from_secs(2))
    );
}

#[test]
fn test_time_tolerance_falls_back_to_build_time() {
    let config = parse_config(
        r#"
version = 1
[ratchet]
build_time_tolerance = "10s"
"#,
    );
    // test_time_tolerance not set, falls back to build_time_tolerance
    assert_eq!(
        config.ratchet.test_time_tolerance_duration(),
        Some(Duration::from_secs(10))
    );
}

#[test]
fn per_package_ratchet_coverage() {
    let config = parse_config(
        r#"
version = 1
[ratchet.package.core]
coverage = false
[ratchet.package.api]
coverage = true
"#,
    );
    // Package-specific overrides
    assert!(!config.ratchet.is_coverage_ratcheted("core"));
    assert!(config.ratchet.is_coverage_ratcheted("api"));
    // Unknown packages fall back to global (true by default)
    assert!(config.ratchet.is_coverage_ratcheted("unknown"));
}

#[test]
fn per_package_ratchet_escapes() {
    let config = parse_config(
        r#"
version = 1
[ratchet.package.core]
escapes = false
"#,
    );
    assert!(!config.ratchet.is_escapes_ratcheted("core"));
    assert!(config.ratchet.is_escapes_ratcheted("unknown"));
}
