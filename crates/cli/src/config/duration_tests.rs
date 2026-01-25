// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::time::Duration;

#[test]
fn parses_seconds() {
    assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
    assert_eq!(parse_duration("0s").unwrap(), Duration::from_secs(0));
}

#[test]
fn parses_fractional_seconds() {
    assert_eq!(
        parse_duration("1.5s").unwrap(),
        Duration::from_secs_f64(1.5)
    );
    assert_eq!(parse_duration("0.5s").unwrap(), Duration::from_millis(500));
    assert_eq!(
        parse_duration("2.25s").unwrap(),
        Duration::from_secs_f64(2.25)
    );
}

#[test]
fn parses_milliseconds() {
    assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    assert_eq!(parse_duration("1ms").unwrap(), Duration::from_millis(1));
    assert_eq!(parse_duration("0ms").unwrap(), Duration::from_millis(0));
    assert_eq!(
        parse_duration("1500ms").unwrap(),
        Duration::from_millis(1500)
    );
}

#[test]
fn parses_minutes() {
    assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
    assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
    assert_eq!(parse_duration("0m").unwrap(), Duration::from_secs(0));
}

#[test]
fn trims_whitespace() {
    assert_eq!(parse_duration("  30s  ").unwrap(), Duration::from_secs(30));
    assert_eq!(
        parse_duration(" 500ms ").unwrap(),
        Duration::from_millis(500)
    );
    assert_eq!(parse_duration("  1m  ").unwrap(), Duration::from_secs(60));
}

#[test]
fn trims_internal_whitespace() {
    assert_eq!(parse_duration("30 s").unwrap(), Duration::from_secs(30));
    assert_eq!(
        parse_duration("500 ms").unwrap(),
        Duration::from_millis(500)
    );
}

#[test]
fn rejects_empty_string() {
    let err = parse_duration("").unwrap_err();
    assert!(err.contains("empty"));
}

#[test]
fn rejects_missing_unit() {
    let err = parse_duration("30").unwrap_err();
    assert!(err.contains("invalid duration format"));
}

#[test]
fn rejects_invalid_number() {
    let err = parse_duration("abcs").unwrap_err();
    assert!(err.contains("invalid duration"));
}

#[test]
fn rejects_unknown_unit() {
    let err = parse_duration("30h").unwrap_err();
    assert!(err.contains("invalid duration format"));
}

#[test]
fn rejects_negative_duration() {
    let err = parse_duration("-5s").unwrap_err();
    assert!(err.contains("negative"));
}
